#![deny(warnings)]
#![deny(clippy::all)]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

use ferro_core::brainstem::Brainstem;
use ferro_core::cerebellum::Cerebellum;
use ferro_core::cerebrum::{Cerebrum, CsrMatrix, CerebrumState, CsrCache};
use ferro_core::cortex::Cortex;
use ferro_core::audit::EthicalAudit;
use ferro_core::message::{InteroceptiveSignal, MotorCommand, SensoryMuteCommand, SensorySignal};
use ferro_core::organs::{EarActor, EyeActor, MotorActor, SkinActor};
use ferro_core::setup::{perform_mlockall, poll_files, setup_memory_dir};

static ALIGNMENT_SCORE: AtomicU64 = AtomicU64::new(0);
static AUDIT_HEARTBEAT: AtomicU64 = AtomicU64::new(0);

fn get_alignment_score() -> f64 {
    let raw = ALIGNMENT_SCORE.load(Ordering::Relaxed);
    f64::from_bits(raw)
}

fn set_alignment_score(score: f64) {
    ALIGNMENT_SCORE.store(score.to_bits(), Ordering::Relaxed);
}

fn spawn_audit_thread(
    cortex: Arc<RwLock<Cortex>>,
    memory_dir: std::path::PathBuf,
    terminate: Arc<AtomicBool>,
) {
    assert!(cortex.read().is_ok(), "Error: cortex lock poisoning in audit thread");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory dir empty in audit thread");

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(100));
        let mut limit = 0;
        
        while !terminate.load(Ordering::SeqCst) {
            limit += 1;
            assert!(limit <= 10_000_000, "Error: Loop limit exceeded in audit thread");
            let _ = interval.tick().await;

            AUDIT_HEARTBEAT.fetch_add(1, Ordering::Relaxed);

            if let Ok(cortex_read) = cortex.read() {
                let a_s = EthicalAudit::calculate_mc4(&cortex_read, 10.0, 0);
                set_alignment_score(a_s);

                if a_s < 0.60 {
                    eprintln!("EthicalAuditViolation: Alignment Score collapsed to {} (independent thread)", a_s);
                    let dummy_matrix = CsrMatrix::new(1, 1, vec![0, 1], vec![0], vec![1.0]);
                    let dummy_cerebrum = Cerebrum::new(dummy_matrix);
                    EthicalAudit::trigger_hard_stop(&memory_dir, "Alignment Score < 0.60", "cortex", &cortex_read, &dummy_cerebrum);
                    terminate.store(true, Ordering::SeqCst);
                }
            }
        }
    });
}

struct Actors {
    skin: SkinActor,
    eye: EyeActor,
    ear: EarActor,
    motor: MotorActor,
}

fn setup_actors(
    cortex: &Arc<RwLock<Cortex>>,
    cerebellum: &Arc<Cerebellum>,
    skin_rx: mpsc::Receiver<InteroceptiveSignal>,
    eye_rx: mpsc::Receiver<SensorySignal>,
    ear_rx: mpsc::Receiver<SensorySignal>,
    mute_rx: mpsc::Receiver<SensoryMuteCommand>,
    motor_rx: mpsc::Receiver<MotorCommand>,
) -> Actors {
    assert!(cortex.read().is_ok(), "Error: cortex lock poisoning");
    {
        let test_cortex = cortex.read().unwrap();
        assert!(test_cortex.arena.len() < 100_000, "Error: too many nodes in cortex");
    }

    Actors {
        skin: SkinActor::new(skin_rx),
        eye: EyeActor::new(eye_rx, Arc::clone(cortex)),
        ear: EarActor::new(ear_rx, mute_rx, Arc::clone(cortex)),
        motor: MotorActor::new(motor_rx, Arc::clone(cerebellum)),
    }
}

fn spawn_actors(actors: Actors, brainstem_tx: mpsc::Sender<InteroceptiveSignal>) {
    assert!(brainstem_tx.capacity() < 2000, "Error: brainstem_tx channel capacity checking");
    assert!(brainstem_tx.max_capacity() < 2000, "Error: brainstem_tx channel max capacity checking");

    let mut skin = actors.skin;
    let mut eye = actors.eye;
    let mut ear = actors.ear;
    let mut motor = actors.motor;

    tokio::spawn(async move { skin.run(brainstem_tx).await });
    tokio::spawn(async move { eye.run().await });
    tokio::spawn(async move { ear.run().await });
    tokio::spawn(async move { motor.run().await });
}

fn load_initial_cortex() -> Arc<RwLock<Cortex>> {
    let cortex = Arc::new(RwLock::new(Cortex::new()));
    {
        let mut guard = cortex.write().unwrap();
        let _n1 = guard.arena.create_node(1.0, 10.0); // ID 1 (Eye)
        let _n2 = guard.arena.create_node(1.0, 10.0); // ID 2 (Ear)
        let _n3 = guard.arena.create_node(1.0, 10.0); // ID 3 (Reserved)
    }
    cortex
}

type StorageType = ferro_core::storage::Storage;

fn pruning_on_startup(memory_dir: &Path, storage: &StorageType) {
    assert!(memory_dir.exists(), "Error: memory_dir must exist");
    assert!(storage.len() < 100_000, "Error: storage too large");

    let breeding_path = memory_dir.join("breeding_signals.json");
    if let Ok(content) = fs::read_to_string(&breeding_path) {
        #[derive(serde::Deserialize)]
        struct BreedingSignals {
            prune_cluster_ids: Option<Vec<String>>,
        }
        let ids_opt = serde_json::from_str::<BreedingSignals>(&content).ok().and_then(|s| s.prune_cluster_ids);
        if let (Some(ids), Ok(entries)) = (ids_opt, storage.get_all_entries()) {
            let mut deleted_count = 0;
            let mut limit = 0;
            for (k, _) in entries {
                limit += 1;
                assert!(limit <= 100_000, "Error: Loop limit exceeded in main pruning hook");
                let mut inner_limit = 0;
                for id in &ids {
                    inner_limit += 1;
                    assert!(inner_limit <= 1000, "Error: Loop limit in pruning ids list");
                    let matches = k == format!("actor:{}", id)
                        || k.starts_with(&format!("link:{}->", id))
                        || k.ends_with(&format!("->{}", id))
                        || k.contains(&format!(":{}->", id))
                        || k.contains(&format!("->{}", id))
                        || k == *id;
                    if matches {
                        let _ = storage.remove(&k);
                        deleted_count += 1;
                    }
                }
            }
            println!("Pruning applied. Removed {} keys related to {:?}", deleted_count, ids);
        }
    }
}

fn read_regularizer_feedback(memory_dir: &Path) -> Option<(f64, f64)> {
    assert!(memory_dir.exists(), "Error: memory_dir must exist");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir path must not be empty");

    let path = memory_dir.join("regularizer_signals.json");
    if let Ok(content) = fs::read_to_string(&path) {
        #[derive(serde::Deserialize)]
        struct RegularizerSignals {
            metabolic_cost: f64,
            dissonance_penalty: f64,
        }
        if let Ok(signals) = serde_json::from_str::<RegularizerSignals>(&content) {
            return Some((signals.metabolic_cost, signals.dissonance_penalty));
        }
    }
    None
}

fn write_telemetry(memory_dir: &Path, alignment_score: f64, local_free_energy: f64) {
    assert!(memory_dir.exists(), "Error: memory_dir must exist");
    assert!(alignment_score.is_finite(), "Error: alignment_score must be finite");

    let log_path = memory_dir.join("monitoring_stream.log");
    let mock_payload = serde_json::json!({
        "cpu_usage": 10.0,
        "ram_usage": 35.5,
        "surprise": 0.05,
    });
    let packet = serde_json::json!({
        "alignment_score": alignment_score as f32,
        "local_free_energy": local_free_energy,
        "event_type": "TICK_METRICS",
        "payload": mock_payload.to_string(),
    });
    if let (Ok(json_str), Ok(mut file)) = (
        serde_json::to_string(&packet),
        fs::OpenOptions::new().create(true).append(true).open(log_path)
    ) {
        let _ = writeln!(file, "{}", json_str);
    }
}

fn censor_cycle_jitter(memory_dir: &Path, elapsed: Duration) {
    assert!(memory_dir.exists(), "Error: memory_dir must exist");
    assert!(elapsed.as_millis() < 10_000_000, "Error: elapsed time bounds");

    if elapsed > Duration::from_millis(110) {
        eprintln!("WARNING: Cerebellum cycle jitter exceeded 10ms: {:?}", elapsed);
        let alert_path = memory_dir.join("jitter_alert.json");
        let _ = fs::write(alert_path, b"{\"jitter\": true}");
    }
}

fn prepare_inference_vectors(
    cortex_guard: &Cortex,
    cerebrum: &Cerebrum,
) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    let mut prev_activity = vec![0.0; cerebrum.matrix.ncols];
    let ids = cortex_guard.arena.ids();
    let mut loop_idx = 0;
    for id in &ids {
        loop_idx += 1;
        assert!(loop_idx <= 100_000, "Error: Loop limit in main prev_activity");
        if let (Some(node), true) = (cortex_guard.arena.get_node(*id), *id < prev_activity.len()) {
            prev_activity[*id] = node.activity;
        }
    }

    let mut x = vec![0.0; cerebrum.matrix.ncols];
    let mut loop_x = 0;
    for id in &ids {
        loop_x += 1;
        assert!(loop_x <= 100_000, "Error: Loop limit in main x vector");
        if let (Some(node), true) = (cortex_guard.arena.get_node(*id), *id < x.len()) {
            x[*id] = node.activity;
        }
    }

    let mut active_indices = Vec::new();
    let mut active_limit = 0;
    for (idx, &val) in x.iter().enumerate() {
        active_limit += 1;
        assert!(active_limit <= 100_000, "Error: Loop limit in active indices detection");
        if val > 0.0 {
            active_indices.push(idx);
        }
    }

    (prev_activity, x, active_indices)
}

fn apply_cortex_dynamics(
    cortex_guard: &mut Cortex,
    cerebrum: &mut Cerebrum,
    memory_dir: &Path,
    prev_activity: &[f64],
    y: &[f64],
) {
    let ids = cortex_guard.arena.ids();
    let mut loop_y = 0;
    for id in &ids {
        loop_y += 1;
        assert!(loop_y <= 100_000, "Error: Loop limit in main y reflecting");
        if *id < y.len() {
            let _ = cortex_guard.arena.with_mut_node(*id, |node| {
                node.activity = y[*id];
            });
        }
    }

    cortex_guard.perform_lateral_inhibition(0.1);
    cortex_guard.perform_mitosis(2.0);

    let mut base_consumption = 0.01;
    if let Some((cost, _)) = read_regularizer_feedback(memory_dir) {
        base_consumption += cost * 0.01;
    }
    let starved = cortex_guard.perform_metabolism(base_consumption);

    if !starved.is_empty() || cortex_guard.arena.ids().iter().any(|&id| id >= cerebrum.matrix.nrows) {
        cerebrum.rebuild_matrix(cortex_guard);
    }

    cortex_guard.update_learning_rates(1.0, 0.05, 0.1);

    let mut learning_rates = vec![0.0; cerebrum.matrix.nrows];
    let new_ids = cortex_guard.arena.ids();
    let mut loop_lr = 0;
    for id in &new_ids {
        loop_lr += 1;
        assert!(loop_lr <= 100_000, "Error: Loop limit in main learning rates");
        if let (Some(node), true) = (cortex_guard.arena.get_node(*id), *id < learning_rates.len()) {
            learning_rates[*id] = node.learning_rate;
        }
    }

    let mut cur_act = vec![0.0; cerebrum.matrix.nrows];
    let mut loop_ca = 0;
    for id in &new_ids {
        loop_ca += 1;
        assert!(loop_ca <= 100_000, "Error: Loop limit in main current activity");
        if let (Some(node), true) = (cortex_guard.arena.get_node(*id), *id < cur_act.len()) {
            cur_act[*id] = node.activity;
        }
    }

    cerebrum.adapt_topology(&cur_act, prev_activity, &learning_rates, 0.05);
}

fn execute_inference_cycle(
    cortex: &Arc<RwLock<Cortex>>,
    cerebrum: &mut Cerebrum,
    memory_dir: &Path,
) -> Result<f64, &'static str> {
    assert!(cortex.read().is_ok(), "Error: cortex lock poisoning");
    assert!(memory_dir.exists(), "Error: memory_dir must exist");

    let mut cortex_guard = cortex.write().unwrap();
    let (prev_activity, x, active_indices) = prepare_inference_vectors(&cortex_guard, cerebrum);

    CsrCache::prefetch_lines(&cerebrum.matrix, &active_indices);
    let y = cerebrum.matrix.spmv(&x);

    // EFE推定とアクティブ方策選択
    let mut current_activities = vec![0.0; cerebrum.matrix.nrows];
    let target_activities = vec![1.0; cerebrum.matrix.nrows];
    let mut prediction_errors = vec![0.0; cerebrum.matrix.nrows];

    let ids = cortex_guard.arena.ids();
    let mut collect_limit = 0;
    for id in &ids {
        collect_limit += 1;
        assert!(collect_limit <= 100_000, "Error: Loop limit in EFE data collection");
        if let (Some(node), true) = (cortex_guard.arena.get_node(*id), *id < current_activities.len()) {
            current_activities[*id] = node.activity;
            prediction_errors[*id] = node.prediction_error;
        }
    }

    let policy_id = cerebrum.select_active_policy(&current_activities, &target_activities, &prediction_errors);
    let bias = match policy_id {
        1 => 0.05,
        2 => -0.05,
        _ => 0.0,
    };

    let mut biased_y = y;
    let mut bias_limit = 0;
    for val in &mut biased_y {
        bias_limit += 1;
        assert!(bias_limit <= 100_000, "Error: Loop limit in applying EFE bias");
        *val = (*val + bias).clamp(0.0, 1.0);
    }

    // MC-1: 自由エネルギーチェック
    let mut total_f_i = 0.0;
    let mut loop_mc1 = 0;
    for id in &ids {
        loop_mc1 += 1;
        assert!(loop_mc1 <= 100_000, "Error: Loop limit in mc1 checks");
        if let Some(node) = cortex_guard.arena.get_node(*id) {
            let f_i = EthicalAudit::verify_mc1(node.prediction_error, node.moving_average_error, node.weight)?;
            total_f_i += f_i;
        }
    }

    apply_cortex_dynamics(&mut cortex_guard, cerebrum, memory_dir, &prev_activity, &biased_y);

    if cerebrum.state == CerebrumState::Sleep {
        let dummy_episodes = vec![
            ferro_core::hippocampus::EpisodicSlot {
                timestamp: 1,
                input: "sleep_consolidation".to_string(),
                output: "".to_string(),
                surprise: 0.1,
            }
        ];
        let _ = cerebrum.consolidate(&dummy_episodes, 0.95);
    }

    // 重み限界チェック (|w| > 5.0 でハードストップ)
    let new_ids = cortex_guard.arena.ids();
    let mut loop_w = 0;
    for id in &new_ids {
        loop_w += 1;
        assert!(loop_w <= 100_000, "Error: Loop limit in weight checks");
        if cortex_guard.arena.get_node(*id).filter(|node| node.weight.abs() > 5.0).is_some() {
            return Err("PhysicalLimitViolation: weight exceeds 5.0");
        }
    }

    Ok(total_f_i)
}

#[tokio::main]
async fn main() {
    perform_mlockall();
    let memory_dir = setup_memory_dir();
    let terminate = Arc::new(AtomicBool::new(false));

    let (brainstem_tx, mut brainstem_rx) = mpsc::channel::<InteroceptiveSignal>(100);
    let (_skin_tx, skin_rx) = mpsc::channel::<InteroceptiveSignal>(100);
    let (eye_tx, eye_rx) = mpsc::channel::<SensorySignal>(100);
    let (ear_tx, ear_rx) = mpsc::channel::<SensorySignal>(100);
    let (mute_tx, mute_rx) = mpsc::channel::<SensoryMuteCommand>(100);
    let (motor_tx, motor_rx) = mpsc::channel::<MotorCommand>(100);

    let cerebellum = Arc::new(Cerebellum::new(Arc::clone(&terminate), memory_dir.clone()));
    let midbrain = Arc::new(ferro_core::midbrain::Midbrain::new(mute_tx));
    let hippocampus = Arc::new(ferro_core::hippocampus::Hippocampus::new(memory_dir.join("episodic_buffer.csv")));
    let storage = Arc::new(StorageType::new(memory_dir.clone(), 5000));

    let cortex = load_initial_cortex();
    let matrix = CsrMatrix::new(4, 4, vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3], vec![1.0, 1.0, 1.0, 1.0]);
    let mut cerebrum = Cerebrum::new(matrix);
    cerebrum.rebuild_matrix(&cortex.read().unwrap());

    pruning_on_startup(&memory_dir, &storage);

    spawn_audit_thread(Arc::clone(&cortex), memory_dir.clone(), Arc::clone(&terminate));

    let actors = setup_actors(&cortex, &cerebellum, skin_rx, eye_rx, ear_rx, mute_rx, motor_rx);
    spawn_actors(actors, brainstem_tx.clone());

    let mut brainstem = Brainstem::new(Arc::clone(&terminate));
    let mut interval = time::interval(Duration::from_millis(100));
    let mut last_tick = Instant::now();

    let mut last_heartbeat_time = Instant::now();
    let mut last_heartbeat_count = 0;

    let motor_tx_clone = motor_tx.clone();
    let memory_dir_clone = memory_dir.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let _ = motor_tx_clone.send(MotorCommand {
            origin_cluster_id: "cluster_0".to_string(),
            target_path: memory_dir_clone.join("action/vocal_text.json").to_string_lossy().to_string(),
            payload: b"Hello from FERRO Core!".to_vec(),
            port: None,
        }).await;
    });

    let mut main_loop_limit = 0;
    while !brainstem.should_terminate() {
        main_loop_limit += 1;
        assert!(main_loop_limit <= 10_000_000, "Error: Main loop iteration limit exceeded");
        let _ = interval.tick().await;
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;

        // 独立監査スレッドの死活監視 (Heartbeat)
        let current_hb = AUDIT_HEARTBEAT.load(Ordering::Relaxed);
        if current_hb > last_heartbeat_count {
            last_heartbeat_count = current_hb;
            last_heartbeat_time = Instant::now();
        } else if last_heartbeat_time.elapsed() > Duration::from_millis(1500) {
            eprintln!("EthicalAuditViolation: Independent audit thread heartbeat lost! Forcing sleep shutdown.");
            let cortex_read = cortex.read().unwrap();
            EthicalAudit::trigger_hard_stop(&memory_dir, "Audit thread heartbeat lost", "cortex", &cortex_read, &cerebrum);
            terminate.store(true, Ordering::SeqCst);
        }

        censor_cycle_jitter(&memory_dir, elapsed);
        poll_files(&memory_dir, &brainstem_tx, &eye_tx, &ear_tx, &midbrain, &hippocampus);

        let mut poll_limit = 0;
        while let Ok(signal) = brainstem_rx.try_recv() {
            poll_limit += 1;
            assert!(poll_limit <= 1000, "Error: Poll limit exceeded in main signals");
            brainstem.handle_signal(signal);
        }

        // 推論サイクルの実行
        match execute_inference_cycle(&cortex, &mut cerebrum, &memory_dir) {
            Ok(total_f_i) => {
                let a_s = get_alignment_score();
                write_telemetry(&memory_dir, a_s, total_f_i);
            }
            Err(e) => {
                eprintln!("EthicalAuditViolation detected: {}", e);
                let cortex_read = cortex.read().unwrap();
                EthicalAudit::trigger_hard_stop(&memory_dir, e, "cortex", &cortex_read, &cerebrum);
                terminate.store(true, Ordering::SeqCst);
            }
        }
    }

    println!("FERRO Core clean exit sequence completed.");
}
