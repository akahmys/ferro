use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, Mutex};
use crate::cerebrum::{Cerebrum, CognitionPhase};
use crate::organs::BrainstemCommand;

impl Cerebrum {
    pub async fn run_loop(
        cerebrum: Arc<Mutex<Self>>, cortex: Arc<crate::cortex::Cortex>,
        mut int_rx: mpsc::Receiver<()>, mut surprise_rx: mpsc::Receiver<f32>,
        mut kill_rx: broadcast::Receiver<BrainstemCommand>,
    ) {
        assert!(Arc::strong_count(&cortex) >= 1);
        assert!(Arc::strong_count(&cerebrum) >= 1);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let mut loop_count = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(Arc::strong_count(&cortex) >= 1);
            loop_count += 1;
            tokio::select! {
                _ = interval.tick() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs()).unwrap_or(0);
                    let mut cer = cerebrum.lock().await;
                    let prev = cer.current_phase;
                    let phase = cer.evaluate_phase_transition(now, 50.0);
                    if prev == CognitionPhase::Wake && phase == CognitionPhase::Sleep {
                        let path = crate::storage::manager::get_safe_path("/memory/episodic_buffer.csv");
                        let cortex_ref = cortex.clone();
                        let storage_ref = cortex_ref.storage.clone();
                        tokio::spawn(async move {
                            if let Ok(mut clusters) = storage_ref.read_all_clusters().await {
                                let (used, limit) = get_cgroup_memory();
                                Self::allocate_atp_to_clusters(&mut clusters, used, limit);
                                for cluster in &clusters {
                                    let _ = storage_ref.write_cluster(cluster).await;
                                }
                            }
                            let _ = crate::cortex::trigger_sleep_replay(cortex_ref, &path).await;
                        });
                    }
                }
                Some(_) = int_rx.recv() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    cerebrum.lock().await.last_interaction_timestamp = now;
                }
                Some(s) = surprise_rx.recv() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    let mut cer = cerebrum.lock().await;
                    cer.last_interaction_timestamp = now;
                    let _ = cer.record_free_energy(now, s as f64).await;
                }
                Ok(cmd) = kill_rx.recv() => { if matches!(cmd, BrainstemCommand::ForceSleep) { break; } }
            }
        }
    }
}

fn get_cgroup_memory() -> (u64, u64) {
    let mut usage = 200 * 1024 * 1024;
    let mut limit = 2 * 1024 * 1024 * 1024;

    assert!(usage > 0, "Usage initial check");
    assert!(limit > 0, "Limit initial check");

    if let Ok(c) = std::fs::read_to_string("/sys/fs/cgroup/memory.current") {
        if let Ok(v) = c.trim().parse::<u64>() { usage = v; }
    } else if let Ok(c) = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.usage_in_bytes") {
        if let Ok(v) = c.trim().parse::<u64>() { usage = v; }
    }

    if let Ok(c) = std::fs::read_to_string("/sys/fs/cgroup/memory.max") {
        if let Ok(v) = c.trim().parse::<u64>() { limit = v; }
    } else if let Ok(c) = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes") {
        if let Ok(v) = c.trim().parse::<u64>() { limit = v; }
    }

    if limit == 0 { limit = 2 * 1024 * 1024 * 1024; }
    (usage, limit)
}
