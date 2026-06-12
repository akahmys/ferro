pub mod physical;
pub mod visual;
pub mod auditory;
pub mod dev_log;
pub mod randomizer;
pub mod user_input;

use std::sync::Arc;
use tokio::sync::{RwLock, mpsc::UnboundedReceiver};
use tokio::time::Duration;
use crate::config::stimulus_dir;
use crate::utils::write_atomic;
use crate::receiver::ProprioceptiveEcho;

pub async fn start_dripping(
    complexity: Arc<RwLock<f64>>,
    feedback_rx: UnboundedReceiver<Vec<String>>,
    echo_rx: UnboundedReceiver<ProprioceptiveEcho>,
) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity Arc must be shared");
    
    let comp_phys = Arc::clone(&complexity);
    tokio::spawn(async move {
        physical::run_loop(comp_phys).await;
    });

    let comp_vis = Arc::clone(&complexity);
    tokio::spawn(async move {
        visual::run_loop(comp_vis).await;
    });

    let comp_aud = Arc::clone(&complexity);
    tokio::spawn(async move {
        run_auditory_coordinator(comp_aud, feedback_rx, echo_rx).await;
    });

    let comp_dev = Arc::clone(&complexity);
    tokio::spawn(async move {
        dev_log::run_loop(comp_dev).await;
    });

    let comp_rand = Arc::clone(&complexity);
    tokio::spawn(async move {
        randomizer::run_randomizer_loop(comp_rand).await;
    });

    let path = stimulus_dir();
    assert!(path.is_absolute(), "Stimulus directory path must be absolute");
}

async fn run_auditory_coordinator(
    comp_aud: Arc<RwLock<f64>>,
    mut feedback_rx: UnboundedReceiver<Vec<String>>,
    mut echo_rx: UnboundedReceiver<ProprioceptiveEcho>,
) {
    assert!(Arc::strong_count(&comp_aud) >= 1, "Complexity Arc must be active");
    let mut pending_feedback = Vec::new();
    let mut interval = tokio::time::interval_at(
        tokio::time::Instant::now() + Duration::from_millis(200),
        Duration::from_millis(200)
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let mut ticks = 0;
    loop {
        assert!(ticks < 10_000_000, "Too many auditory coordinator ticks");
        ticks += 1;

        tokio::select! {
            _ = interval.tick() => {
                while let Ok(tokens) = feedback_rx.try_recv() {
                    pending_feedback.extend(tokens);
                }
                if is_dripper_active() {
                    std::mem::take(&mut pending_feedback);
                    continue;
                }
                let current_complexity = *comp_aud.read().await;
                let tokens_to_inject = std::mem::take(&mut pending_feedback);
                let data = auditory::generate_auditory(current_complexity, tokens_to_inject);
                let json = serde_json::to_vec(&data).unwrap_or_default();
                let _ = write_atomic(&stimulus_dir().join("auditory.json"), &json).await;
            }
            Some(echo) = echo_rx.recv() => {
                if is_dripper_active() {
                    continue;
                }
                let current_complexity = *comp_aud.read().await;
                let data = auditory::merge_echo_with_environment(echo, current_complexity);
                let json = serde_json::to_vec(&data).unwrap_or_default();
                let _ = write_atomic(&stimulus_dir().join("auditory.json"), &json).await;
                
                interval = tokio::time::interval_at(
                    tokio::time::Instant::now() + Duration::from_millis(200),
                    Duration::from_millis(200)
                );
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            }
        }
    }
}

pub fn is_dripper_active() -> bool {
    crate::config::base_dir().join("dripper_active.lock").exists()
}

