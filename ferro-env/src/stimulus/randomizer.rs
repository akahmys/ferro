use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use rand::Rng;
use crate::config::stimulus_dir;
use crate::utils::write_atomic;
use crate::stimulus::{visual, auditory, dev_log};

/// Runs a loop that mixes visual, auditory, and log stream events randomly to trigger surprise.
pub async fn run_randomizer_loop(complexity: Arc<RwLock<f64>>) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity Arc must be shared");
    
    let mut ticks = 0;
    loop {
        assert!(ticks < 100_000_000, "Too many randomization ticks");
        ticks += 1;

        if crate::stimulus::is_dripper_active() {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            continue;
        }

        let sleep_ms = {
            let mut rng = rand::thread_rng();
            rng.gen_range(500..3000)
        };
        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;

        let cur_complexity = *complexity.read().await;
        assert!(cur_complexity >= 0.0, "Complexity must be non-negative");
        assert!(cur_complexity <= 1.0, "Complexity must not exceed 1.0");

        let choice = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0..3)
        };
        let dir = stimulus_dir();
        match choice {
            0 => {
                let mut data = visual::generate_visual(cur_complexity);
                data.frame_delta += 1.5;
                data.image_embedding = data.image_embedding.iter().map(|v| v * 2.0).collect();
                if let Ok(json) = serde_json::to_vec(&data) {
                    let _ = write_atomic(&dir.join("visual.json"), &json).await;
                }
            }
            1 => {
                let tokens = vec!["ANOMALY".to_string(), "NOISE".to_string()];
                let data = auditory::generate_auditory(cur_complexity, tokens);
                if let Ok(json) = serde_json::to_vec(&data) {
                    let _ = write_atomic(&dir.join("auditory.json"), &json).await;
                }
            }
            _ => {
                if let Some(mut data) = dev_log::generate_dev_log(cur_complexity) {
                    data.increment = "FATAL DANGER ANOMALY EXTREME NOISE SPIKE".to_string();
                    if let Ok(json) = serde_json::to_vec(&data) {
                        let _ = write_atomic(&dir.join("dev_log.json"), &json).await;
                    }
                }
            }
        }
    }
}
