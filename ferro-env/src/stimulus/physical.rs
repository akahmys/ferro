use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, timeout};
use crate::config::stimulus_dir;
use crate::utils::write_atomic;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhysicalStimulus {
    pub timestamp: i64,
    pub cpu_temp: f64,
    pub ram_free: i64,
    pub disk_io: f64,
    pub process_error: i64,
}

pub fn generate_physical(complexity: f64) -> PhysicalStimulus {
    assert!(complexity >= 0.0, "Complexity must be >= 0.0");
    assert!(complexity <= 1.0, "Complexity must be <= 1.0");

    let mut rng = rand::thread_rng();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let (cpu_temp, ram_free, process_error) = if complexity < 0.3 {
        (
            rng.gen_range(40.0..45.0),
            rng.gen_range(6_000_000_000..8_000_000_000),
            0,
        )
    } else if complexity < 0.7 {
        (
            rng.gen_range(45.0..65.0),
            rng.gen_range(4_000_000_000..6_000_000_000),
            0,
        )
    } else {
        let err = if rng.gen_bool(0.2) { 1 } else { 0 };
        (
            rng.gen_range(70.0..82.0),
            rng.gen_range(1_500_000_000..2_000_000_000),
            err,
        )
    };

    let disk_io = rng.gen_range(0.1..50.0) * (1.0 + complexity);

    let stimulus = PhysicalStimulus {
        timestamp: now,
        cpu_temp,
        ram_free,
        disk_io,
        process_error,
    };

    assert!(stimulus.timestamp >= 0, "Timestamp must be non-negative");
    assert!(stimulus.cpu_temp >= 0.0, "CPU temp must be non-negative");

    stimulus
}

pub async fn run_loop(complexity: Arc<RwLock<f64>>) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity Arc must be shared");
    let mut ticks = 0;
    loop {
        assert!(ticks < 10_000_000, "Too many physical ticks");
        ticks += 1;
        let limit = Duration::from_millis(2000);
        let res = timeout(limit, async {
            let current_complexity = *complexity.read().await;
            let data = generate_physical(current_complexity);
            let json = serde_json::to_vec(&data).unwrap_or_default();
            let path = stimulus_dir().join("physical.json");
            let _ = write_atomic(&path, &json).await;
            sleep(Duration::from_millis(1000)).await;
        }).await;
        if res.is_err() { break; }
    }
}
