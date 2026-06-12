use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::SystemTime;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use crate::config::stimulus_dir;
use crate::utils::write_atomic;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VisualStimulus {
    pub timestamp: i64,
    pub frame_delta: f64,
    pub image_embedding: Vec<f64>,
}

pub fn generate_visual(complexity: f64) -> VisualStimulus {
    assert!(complexity >= 0.0, "Complexity must be >= 0.0");
    assert!(complexity <= 1.0, "Complexity must be <= 1.0");

    let mut rng = rand::thread_rng();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let frame_delta = if complexity < 0.3 {
        rng.gen_range(0.00..0.05)
    } else if complexity < 0.7 {
        let angle = (now as f64) / 1000.0;
        let sine_val = (angle.sin() + 1.0) / 2.0; // 0.0 to 1.0
        0.05 + sine_val * 0.25 // 0.05 to 0.30
    } else {
        rng.gen_range(0.30..0.90)
    };

    let mut image_embedding = Vec::with_capacity(5);
    for i in 0..5 {
        let base = (i as f64) * 0.2;
        let noise = if complexity < 0.3 {
            rng.gen_range(-0.01..0.01)
        } else if complexity < 0.7 {
            rng.gen_range(-0.05..0.05)
        } else {
            rng.gen_range(-0.5..0.5)
        };
        image_embedding.push(base + noise);
    }

    let stimulus = VisualStimulus {
        timestamp: now,
        frame_delta,
        image_embedding,
    };

    assert!(stimulus.frame_delta >= 0.0, "Frame delta cannot be negative");
    assert!(stimulus.image_embedding.len() == 5, "Embedding must have exactly 5 elements");

    stimulus
}

pub async fn run_loop(complexity: Arc<RwLock<f64>>) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity Arc must be shared");
    let mut ticks = 0;
    loop {
        assert!(ticks < 100_000_000, "Too many visual ticks");
        ticks += 1;
        if crate::stimulus::is_dripper_active() {
            sleep(Duration::from_millis(500)).await;
            continue;
        }
        let current_complexity = *complexity.read().await;
        let data = generate_visual(current_complexity);
        let json = serde_json::to_vec(&data).unwrap_or_default();
        let path = stimulus_dir().join("visual.json");
        let _ = write_atomic(&path, &json).await;
        sleep(Duration::from_millis(100)).await;
    }
}
