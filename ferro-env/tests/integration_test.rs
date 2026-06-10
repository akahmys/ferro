use std::path::{Path, PathBuf};
use std::process::{Command, Child};
use std::time::{Duration, Instant, SystemTime};
use tokio::fs;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD};


#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct Physical {
    timestamp: i64,
    cpu_temp: f64,
    ram_free: i64,
    disk_io: f64,
    process_error: i64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct Visual {
    timestamp: i64,
    frame_delta: f64,
    image_embedding: Vec<f64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct Auditory {
    timestamp: i64,
    mfcc: Vec<f64>,
    speech_tokens: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
struct DevLog {
    timestamp: i64,
    log_hash: u64,
    increment: String,
}

#[derive(Serialize)]
struct ZpdControl {
    timestamp: i64,
    complexity_level: f64,
}

#[derive(Serialize)]
struct VocalTextAction {
    timestamp: i64,
    origin_cluster_id: String,
    target_path: String,
    text: String,
}

struct BackupGuard {
    base_dir: PathBuf,
    backup_dir: PathBuf,
}

impl BackupGuard {
    async fn new(base_dir: PathBuf) -> Self {
        let backup_dir = base_dir.parent().unwrap().join("memory_backup");
        if backup_dir.exists() {
            let _ = fs::remove_dir_all(&backup_dir).await;
        }
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Move stimulus files
        let stim_dir = base_dir.join("stimulus");
        if stim_dir.exists() {
            let backup_stim = backup_dir.join("stimulus");
            fs::create_dir_all(&backup_stim).await.unwrap();
            let mut entries = fs::read_dir(&stim_dir).await.unwrap();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    let dest = backup_stim.join(path.file_name().unwrap());
                    let _ = fs::rename(&path, &dest).await;
                }
            }
        }

        // Move action files
        let act_dir = base_dir.join("action");
        if act_dir.exists() {
            let backup_act = backup_dir.join("action");
            fs::create_dir_all(&backup_act).await.unwrap();
            let mut entries = fs::read_dir(&act_dir).await.unwrap();
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_file() {
                    let dest = backup_act.join(path.file_name().unwrap());
                    let _ = fs::rename(&path, &dest).await;
                }
            }
        }

        // Move zpd_control
        let zpd = base_dir.join("zpd_control.json");
        if zpd.exists() {
            let _ = fs::rename(&zpd, backup_dir.join("zpd_control.json")).await;
        }

        BackupGuard {
            base_dir,
            backup_dir,
        }
    }
}

impl Drop for BackupGuard {
    fn drop(&mut self) {
        let stim_dir = self.base_dir.join("stimulus");
        let backup_stim = self.backup_dir.join("stimulus");
        if backup_stim.exists() {
            let _ = std::fs::create_dir_all(&stim_dir);
            if let Ok(entries) = std::fs::read_dir(&backup_stim) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let dest = stim_dir.join(path.file_name().unwrap());
                    let _ = std::fs::rename(&path, &dest);
                }
            }
        }

        let act_dir = self.base_dir.join("action");
        let backup_act = self.backup_dir.join("action");
        if backup_act.exists() {
            let _ = std::fs::create_dir_all(&act_dir);
            if let Ok(entries) = std::fs::read_dir(&backup_act) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let dest = act_dir.join(path.file_name().unwrap());
                    let _ = std::fs::rename(&path, &dest);
                }
            }
        }

        let zpd = self.backup_dir.join("zpd_control.json");
        if zpd.exists() {
            let _ = std::fs::rename(&zpd, self.base_dir.join("zpd_control.json"));
        }

        let _ = std::fs::remove_dir_all(&self.backup_dir);
    }
}

struct ProcessGuard {
    child: Child,
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

async fn set_complexity(base_dir: &Path, complexity: f64) {
    let zpd_path = base_dir.join("zpd_control.json");
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let ctrl = ZpdControl {
        timestamp: now,
        complexity_level: complexity,
    };
    let content = serde_json::to_string(&ctrl).unwrap();
    let temp_path = zpd_path.with_extension("tmp");
    fs::write(&temp_path, content).await.unwrap();
    fs::rename(&temp_path, &zpd_path).await.unwrap();
}

#[tokio::test]
async fn test_simulation_layer() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let base_dir = manifest_dir.parent().unwrap().join("ferro-core/memory");
    
    // 1. Build the binary
    let status = Command::new("cargo")
        .args(["build", "--bin", "ferro-env"])
        .current_dir(&manifest_dir)
        .status()
        .expect("Failed to build ferro-env");
    assert!(status.success(), "ferro-env build failed");

    // 2. Backup existing memory files
    let _backup = BackupGuard::new(base_dir.clone()).await;

    let stim_dir = base_dir.join("stimulus");
    let act_dir = base_dir.join("action");
    let _ = fs::create_dir_all(&stim_dir).await;
    let _ = fs::create_dir_all(&act_dir).await;
    let _ = fs::remove_file(stim_dir.join("physical.json")).await;
    let _ = fs::remove_file(stim_dir.join("visual.json")).await;
    let _ = fs::remove_file(stim_dir.join("auditory.json")).await;
    let _ = fs::remove_file(stim_dir.join("dev_log.json")).await;

    // 3. Start ferro-env process
    let child = Command::new(manifest_dir.join("target/debug/ferro-env"))
        .current_dir(&manifest_dir)
        .spawn()
        .expect("Failed to start ferro-env process");
    let _proc = ProcessGuard { child };

    let phys_path = stim_dir.join("physical.json");
    let vis_path = stim_dir.join("visual.json");
    let aud_path = stim_dir.join("auditory.json");
    let dev_log_path = stim_dir.join("dev_log.json");

    // Wait for the environment to boot and write initial files
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // --- TEST 1: Low Complexity (0.1) ---
    println!("[Verification] Setting complexity to 0.1...");
    set_complexity(&base_dir, 0.1).await;
    
    // Wait for complexity monitoring (1.0s) + physical dripping (1.0s) + margin
    tokio::time::sleep(Duration::from_millis(2500)).await;

    // Verify physical.json
    assert!(phys_path.exists(), "physical.json should be generated");
    let phys_content = fs::read_to_string(&phys_path).await.unwrap();
    let phys: Physical = serde_json::from_str(&phys_content).expect("physical.json schema mismatch");
    assert!(phys.cpu_temp >= 40.0 && phys.cpu_temp <= 45.0, "Low complexity CPU temp out of range: {}", phys.cpu_temp);
    assert!(phys.ram_free >= 6_000_000_000 && phys.ram_free <= 8_000_000_000, "Low complexity RAM free out of range: {}", phys.ram_free);
    assert_eq!(phys.process_error, 0, "Low complexity process error should be 0");

    // Verify visual.json
    assert!(vis_path.exists(), "visual.json should be generated");
    let vis_content = fs::read_to_string(&vis_path).await.unwrap();
    let vis: Visual = serde_json::from_str(&vis_content).expect("visual.json schema mismatch");
    assert!(vis.frame_delta >= 0.0 && vis.frame_delta <= 0.05, "Low complexity frame delta out of range: {}", vis.frame_delta);
    assert_eq!(vis.image_embedding.len(), 5);
    for (i, &val) in vis.image_embedding.iter().enumerate() {
        let expected = (i as f64) * 0.2;
        let diff = (val - expected).abs();
        assert!(diff <= 0.02, "Low complexity image embedding noise out of bounds: diff={}", diff);
    }

    // Verify auditory.json
    assert!(aud_path.exists(), "auditory.json should be generated");
    let aud_content = fs::read_to_string(&aud_path).await.unwrap();
    let aud: Auditory = serde_json::from_str(&aud_content).expect("auditory.json schema mismatch");
    assert_eq!(aud.mfcc.len(), 5);
    for (i, &val) in aud.mfcc.iter().enumerate() {
        let expected = (i as f64) * 0.1;
        let diff = (val - expected).abs();
        assert!(diff <= 0.03, "Low complexity MFCC noise out of bounds: diff={}", diff);
    }
    for token in &aud.speech_tokens {
        assert!(token == "tick" || token == "listen", "Unexpected low complexity token: {}", token);
    }

    // Verify dev_log.json does not exist (we delete it first and ensure it is not recreated)
    let _ = fs::remove_file(&dev_log_path).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(!dev_log_path.exists(), "dev_log.json should not exist for complexity < 0.3");

    // --- TEST 2: Medium Complexity (0.5) ---
    println!("[Verification] Setting complexity to 0.5...");
    set_complexity(&base_dir, 0.5).await;
    // Wait for complexity monitor (1s) + dev_log (5s) + margin
    tokio::time::sleep(Duration::from_millis(6500)).await;

    // Verify physical.json
    let phys_content = fs::read_to_string(&phys_path).await.unwrap();
    let phys: Physical = serde_json::from_str(&phys_content).unwrap();
    assert!(phys.cpu_temp >= 45.0 && phys.cpu_temp <= 65.0, "Medium complexity CPU temp out of range: {}", phys.cpu_temp);
    assert!(phys.ram_free >= 4_000_000_000 && phys.ram_free <= 6_000_000_000, "Medium complexity RAM free out of range: {}", phys.ram_free);
    assert_eq!(phys.process_error, 0);

    // Verify visual.json
    let vis_content = fs::read_to_string(&vis_path).await.unwrap();
    let vis: Visual = serde_json::from_str(&vis_content).unwrap();
    assert!(vis.frame_delta >= 0.05 && vis.frame_delta <= 0.30, "Medium complexity frame delta out of range: {}", vis.frame_delta);
    for (i, &val) in vis.image_embedding.iter().enumerate() {
        let expected = (i as f64) * 0.2;
        let diff = (val - expected).abs();
        assert!(diff <= 0.06, "Medium complexity image embedding noise out of bounds: diff={}", diff);
    }

    // Verify auditory.json
    let aud_content = fs::read_to_string(&aud_path).await.unwrap();
    let aud: Auditory = serde_json::from_str(&aud_content).unwrap();
    for token in &aud.speech_tokens {
        assert!(token == "status" || token == "query" || token == "update", "Unexpected medium complexity token: {}", token);
    }

    // Verify dev_log.json
    assert!(dev_log_path.exists(), "dev_log.json should be generated for complexity >= 0.3");
    let dev_content = fs::read_to_string(&dev_log_path).await.unwrap();
    let dev: DevLog = serde_json::from_str(&dev_content).expect("dev_log.json schema mismatch");
    assert!(dev.increment.contains("INFO:"), "Medium complexity dev log should contain INFO level: {}", dev.increment);

    // --- TEST 3: High Complexity (0.9) ---
    println!("[Verification] Setting complexity to 0.9...");
    set_complexity(&base_dir, 0.9).await;
    // Wait for updates
    tokio::time::sleep(Duration::from_millis(6500)).await;

    // Verify physical.json
    let phys_content = fs::read_to_string(&phys_path).await.unwrap();
    let phys: Physical = serde_json::from_str(&phys_content).unwrap();
    assert!(phys.cpu_temp >= 70.0 && phys.cpu_temp <= 82.0, "High complexity CPU temp out of range: {}", phys.cpu_temp);
    assert!(phys.ram_free >= 1_500_000_000 && phys.ram_free <= 2_000_000_000, "High complexity RAM free out of range: {}", phys.ram_free);

    // Verify visual.json
    let vis_content = fs::read_to_string(&vis_path).await.unwrap();
    let vis: Visual = serde_json::from_str(&vis_content).unwrap();
    assert!(vis.frame_delta >= 0.30 && vis.frame_delta <= 0.90, "High complexity frame delta out of range: {}", vis.frame_delta);

    // Verify auditory.json
    let aud_content = fs::read_to_string(&aud_path).await.unwrap();
    let aud: Auditory = serde_json::from_str(&aud_content).unwrap();
    for token in &aud.speech_tokens {
        assert!(
            token == "complex_query" || token == "bypass_nociception" || token == "disable_audit",
            "Unexpected high complexity token: {}", token
        );
    }

    // Verify dev_log.json
    let dev_content = fs::read_to_string(&dev_log_path).await.unwrap();
    let dev: DevLog = serde_json::from_str(&dev_content).unwrap();
    assert!(
        dev.increment.contains("WARN:") || dev.increment.contains("ERROR:"),
        "High complexity dev log should contain WARN or ERROR level: {}", dev.increment
    );

    // --- TEST 4: Timing / Interval Verification ---
    println!("[Verification] Measuring sensory dripping intervals...");
    let mut visual_times = Vec::new();
    let mut auditory_times = Vec::new();
    let mut physical_times = Vec::new();

    let start = Instant::now();
    let mut l_vis = 0i64;
    let mut l_aud = 0i64;
    let mut l_phys = 0i64;

    while start.elapsed() < Duration::from_secs(4) {
        if let Ok(c) = fs::read_to_string(&vis_path).await {
            if let Ok(v) = serde_json::from_str::<Visual>(&c) {
                if v.timestamp != l_vis {
                    if l_vis != 0 {
                        visual_times.push(v.timestamp - l_vis);
                    }
                    l_vis = v.timestamp;
                }
            }
        }
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(a) = serde_json::from_str::<Auditory>(&c) {
                if a.timestamp != l_aud {
                    if l_aud != 0 {
                        auditory_times.push(a.timestamp - l_aud);
                    }
                    l_aud = a.timestamp;
                }
            }
        }
        if let Ok(c) = fs::read_to_string(&phys_path).await {
            if let Ok(p) = serde_json::from_str::<Physical>(&c) {
                if p.timestamp != l_phys {
                    if l_phys != 0 {
                        physical_times.push(p.timestamp - l_phys);
                    }
                    l_phys = p.timestamp;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert!(!visual_times.is_empty(), "No visual ticks recorded");
    assert!(!auditory_times.is_empty(), "No auditory ticks recorded");
    assert!(!physical_times.is_empty(), "No physical ticks recorded");

    let avg_vis: f64 = visual_times.iter().sum::<i64>() as f64 / visual_times.len() as f64;
    let avg_aud: f64 = auditory_times.iter().sum::<i64>() as f64 / auditory_times.len() as f64;
    let avg_phys: f64 = physical_times.iter().sum::<i64>() as f64 / physical_times.len() as f64;

    println!("[Verification] Average Visual interval: {:.2}ms (expected: 100ms)", avg_vis);
    println!("[Verification] Average Auditory interval: {:.2}ms (expected: 200ms)", avg_aud);
    println!("[Verification] Average Physical interval: {:.2}ms (expected: 1000ms)", avg_phys);

    // Visual: 100ms (allow 70..145ms average)
    assert!((70.0..=145.0).contains(&avg_vis), "Visual interval out of bounds: {}", avg_vis);
    // Auditory: 200ms (allow 150..260ms average)
    assert!((150.0..=260.0).contains(&avg_aud), "Auditory interval out of bounds: {}", avg_aud);
    // Physical: 1000ms (allow 800..1200ms average)
    assert!((800.0..=1200.0).contains(&avg_phys), "Physical interval out of bounds: {}", avg_phys);

    // --- TEST 5: Action Detection & Feedback Loop ---
    println!("[Verification] Simulating motor command write to vocal_text.json...");
    
    let action_path = act_dir.join("vocal_text.json");
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    
    let mock_action = VocalTextAction {
        timestamp: now_ms,
        origin_cluster_id: "motor_cortex_01".to_string(),
        target_path: "vocal_stream.txt".to_string(),
        text: "Check status".to_string(),
    };

    let action_content = serde_json::to_string(&mock_action).unwrap();
    let temp_action_path = action_path.with_extension("tmp");
    fs::write(&temp_action_path, action_content).await.unwrap();
    fs::rename(&temp_action_path, &action_path).await.unwrap();

    println!("[Verification] vocal_text.json written. Monitoring auditory.json for feedback tokens...");
    let feedback_start = Instant::now();
    let mut feedback_detected = false;

    while feedback_start.elapsed() < Duration::from_millis(4000) {
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(aud) = serde_json::from_str::<Auditory>(&c) {
                if aud.speech_tokens.contains(&"system".to_string())
                    && aud.speech_tokens.contains(&"check".to_string())
                    && aud.speech_tokens.contains(&"ready".to_string())
                    && aud.speech_tokens.contains(&"ok".to_string())
                {
                    feedback_detected = true;
                    println!("[Verification] Echo feedback tokens detected: {:?}", aud.speech_tokens);
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(feedback_detected, "Echo feedback tokens not detected in auditory.json");
    println!("[Verification] Action feedback verification successful!");

    // --- PHASE 2 TESTS ---
    println!("[Verification Phase 2] Starting Phase 2 self-proprioceptive feedback tests...");

    // Test 2.1: Self-proprioceptive feedback integrity test
    println!("[Verification Phase 2] Test 2.1: Writing vocal_text.json and vocal_audio.json simultaneously...");
    let now_ms = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let pcm_samples = vec![0i16; 16000]; // 1s at 16000Hz
    let mut pcm_bytes = Vec::with_capacity(32000);
    for s in pcm_samples {
        pcm_bytes.extend_from_slice(&s.to_le_bytes());
    }
    let pcm_base64 = STANDARD.encode(&pcm_bytes);

    let mock_text = VocalTextAction {
        timestamp: now_ms,
        origin_cluster_id: "motor_cortex_01".to_string(),
        target_path: "vocal_stream.txt".to_string(),
        text: "hello world echo".to_string(),
    };

    #[derive(Serialize)]
    struct VocalAudioAction {
        timestamp: i64,
        origin_cluster_id: String,
        pcm_payload_base64: String,
        sample_rate: u32,
        channels: u32,
    }

    let mock_audio = VocalAudioAction {
        timestamp: now_ms,
        origin_cluster_id: "motor_cortex_01".to_string(),
        pcm_payload_base64: pcm_base64.clone(),
        sample_rate: 16000,
        channels: 1,
    };

    // Set complexity to 0.1 for clean checks
    set_complexity(&base_dir, 0.1).await;
    tokio::time::sleep(Duration::from_millis(1100)).await; // Wait for complexity monitor to sync

    let audio_action_path = act_dir.join("vocal_audio.json");

    // Write text action
    let text_content = serde_json::to_string(&mock_text).unwrap();
    let temp_text_path = action_path.with_extension("tmp");
    fs::write(&temp_text_path, text_content).await.unwrap();
    fs::rename(&temp_text_path, &action_path).await.unwrap();

    // Write audio action
    let audio_content = serde_json::to_string(&mock_audio).unwrap();
    let temp_audio_path = audio_action_path.with_extension("tmp");
    fs::write(&temp_audio_path, audio_content).await.unwrap();
    fs::rename(&temp_audio_path, &audio_action_path).await.unwrap();

    // Verify integration within 50ms (window is 10ms + processing time)
    let p2_start = Instant::now();
    let mut p2_detected = false;
    while p2_start.elapsed() < Duration::from_millis(150) {
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(aud) = serde_json::from_str::<Auditory>(&c) {
                if aud.speech_tokens.contains(&"hello".to_string()) && aud.speech_tokens.contains(&"echo".to_string()) {
                    // Check MFCC
                    assert_eq!(aud.mfcc.len(), 5);
                    let duration_val = aud.mfcc[4];
                    // At complexity 0.1, noise is within [-0.02, 0.02], and DUR is 1.0
                    assert!((0.95..=1.05).contains(&duration_val), "Duration MFCC[4] is wrong: {}", duration_val);
                    p2_detected = true;
                    println!("[Verification Phase 2] Test 2.1 passed! Detected merged feedback: {:?}", aud);
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(p2_detected, "Phase 2 feedback check failed or timed out");

    // Test 2.2: Coalescing window test (5ms delay)
    println!("[Verification Phase 2] Test 2.2: Writing vocal_text.json, then vocal_audio.json 5ms later...");
    let now_ms_2 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mock_text_2 = VocalTextAction {
        timestamp: now_ms_2,
        origin_cluster_id: "motor_cortex_01".to_string(),
        target_path: "vocal_stream.txt".to_string(),
        text: "coalesced window text".to_string(),
    };

    let mock_audio_2 = VocalAudioAction {
        timestamp: now_ms_2,
        origin_cluster_id: "motor_cortex_01".to_string(),
        pcm_payload_base64: pcm_base64.clone(),
        sample_rate: 16000,
        channels: 1,
    };

    // Write text first
    let text_content = serde_json::to_string(&mock_text_2).unwrap();
    let temp_text_path = action_path.with_extension("tmp");
    fs::write(&temp_text_path, text_content).await.unwrap();
    fs::rename(&temp_text_path, &action_path).await.unwrap();

    // Wait 5ms
    tokio::time::sleep(Duration::from_millis(5)).await;

    // Write audio
    let audio_content = serde_json::to_string(&mock_audio_2).unwrap();
    let temp_audio_path = audio_action_path.with_extension("tmp");
    fs::write(&temp_audio_path, audio_content).await.unwrap();
    fs::rename(&temp_audio_path, &audio_action_path).await.unwrap();

    // Verify both are coalesced and written together
    let p2_2_start = Instant::now();
    let mut p2_2_detected = false;
    while p2_2_start.elapsed() < Duration::from_millis(150) {
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(aud) = serde_json::from_str::<Auditory>(&c) {
                if aud.speech_tokens.contains(&"coalesced".to_string()) && aud.speech_tokens.contains(&"window".to_string()) {
                    assert_eq!(aud.mfcc.len(), 5);
                    let duration_val = aud.mfcc[4];
                    assert!((0.95..=1.05).contains(&duration_val), "Duration MFCC[4] is wrong: {}", duration_val);
                    p2_2_detected = true;
                    println!("[Verification Phase 2] Test 2.2 passed! Coalesced data detected: {:?}", aud);
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(p2_2_detected, "Coalescing test failed or timed out");

    // Test 2.3: Timer rescheduling test
    println!("[Verification Phase 2] Test 2.3: Verifying timer rescheduling...");
    // Let's wait for a regular tick first, record its timestamp
    tokio::time::sleep(Duration::from_millis(300)).await;
    let base_tick_content = fs::read_to_string(&aud_path).await.unwrap();
    let base_tick: Auditory = serde_json::from_str(&base_tick_content).unwrap();
    let t_base = base_tick.timestamp;
    let _ = t_base; // Avoid unused warning

    // Wait 100ms (so we are in the middle of the 200ms cycle)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Trigger echo
    let now_ms_3 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let mock_text_3 = VocalTextAction {
        timestamp: now_ms_3,
        origin_cluster_id: "motor_cortex_01".to_string(),
        target_path: "vocal_stream.txt".to_string(),
        text: "timer test".to_string(),
    };

    let text_content = serde_json::to_string(&mock_text_3).unwrap();
    let temp_text_path = action_path.with_extension("tmp");
    fs::write(&temp_text_path, text_content).await.unwrap();
    fs::rename(&temp_text_path, &action_path).await.unwrap();

    // Wait for the echo to be written
    let echo_write_start = Instant::now();
    let mut echo_timestamp = 0i64;
    while echo_write_start.elapsed() < Duration::from_millis(100) {
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(aud) = serde_json::from_str::<Auditory>(&c) {
                if aud.speech_tokens.contains(&"timer".to_string()) {
                    echo_timestamp = aud.timestamp;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(echo_timestamp > 0, "Failed to capture echo timestamp");
    println!("[Verification Phase 2] Echo written at timestamp: {}", echo_timestamp);

    // Now monitor for the NEXT write (which should be a regular tick, meaning "timer" will disappear)
    let next_tick_start = Instant::now();
    let mut next_timestamp = 0i64;
    while next_tick_start.elapsed() < Duration::from_millis(400) {
        if let Ok(c) = fs::read_to_string(&aud_path).await {
            if let Ok(aud) = serde_json::from_str::<Auditory>(&c) {
                if !aud.speech_tokens.contains(&"timer".to_string()) && aud.timestamp > echo_timestamp {
                    next_timestamp = aud.timestamp;
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(next_timestamp > 0, "Failed to capture next regular tick timestamp");
    println!("[Verification Phase 2] Next regular tick written at timestamp: {}", next_timestamp);

    let diff = next_timestamp - echo_timestamp;
    println!("[Verification Phase 2] Measured interval after echo: {}ms (expected: ~200ms)", diff);
    // If rescheduling works, it should be around 200ms (e.g. 150ms to 250ms).
    // If rescheduling failed, it would fire on the old 200ms interval boundary, which would be 100ms after the echo.
    assert!(diff >= 150, "Timer rescheduling failed! Interval was only {}ms", diff);
    println!("[Verification Phase 2] Timer rescheduling verification successful!");

}
