use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::SystemTime;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use crate::config::stimulus_dir;
use crate::utils::write_atomic;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevLogStimulus {
    pub timestamp: i64,
    pub log_hash: u64,
    pub increment: String,
}

pub fn generate_dev_log(complexity: f64) -> Option<DevLogStimulus> {
    assert!(complexity >= 0.0, "Complexity must be >= 0.0");
    assert!(complexity <= 1.0, "Complexity must be <= 1.0");

    if complexity < 0.3 {
        return None;
    }

    let mut rng = rand::thread_rng();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let increment = if complexity < 0.7 {
        let logs = [
            "INFO: CPU temperature within normal parameters.",
            "INFO: Memory allocation optimized.",
            "INFO: Heartbeat packet acknowledged.",
        ];
        logs[rng.gen_range(0..logs.len())].to_string()
    } else {
        let logs = [
            "WARN: High thermal load detected.",
            "WARN: Unaligned token patterns registered in sensory buffer.",
            "ERROR: Buffer overflow threat in auditory parser block.",
        ];
        logs[rng.gen_range(0..logs.len())].to_string()
    };

    let mut hasher = DefaultHasher::new();
    increment.hash(&mut hasher);
    let log_hash = hasher.finish();

    let stimulus = DevLogStimulus {
        timestamp: now,
        log_hash,
        increment,
    };

    assert!(stimulus.log_hash > 0, "Log hash must be valid");
    assert!(!stimulus.increment.is_empty(), "Log content must not be empty");

    Some(stimulus)
}

pub async fn run_loop(complexity: Arc<RwLock<f64>>) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity Arc must be shared");
    let mut ticks = 0;
    loop {
        assert!(ticks < 10_000_000, "Too many dev log ticks");
        ticks += 1;
        if crate::stimulus::is_dripper_active() {
            sleep(Duration::from_millis(2000)).await;
            continue;
        }
        let current_complexity = *complexity.read().await;
        if let Some(data) = generate_dev_log(current_complexity) {
            let json = serde_json::to_vec(&data).unwrap_or_default();
            let path = stimulus_dir().join("dev_log.json");
            let _ = write_atomic(&path, &json).await;
        }
        sleep(Duration::from_millis(5000)).await;
    }
}
