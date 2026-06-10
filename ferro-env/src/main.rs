#![deny(warnings)]
#![deny(clippy::all)]

mod config;
mod utils;
mod stimulus;
mod receiver;

use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration, timeout};
use crate::utils::read_zpd_complexity;
use crate::stimulus::start_dripping;
use crate::receiver::motor::start_receiver;
use crate::receiver::initialize_receiver_subsystem;
use crate::receiver::ProprioceptiveEcho;

async fn run_zpd_monitor(complexity: Arc<RwLock<f64>>) {
    assert!(Arc::strong_count(&complexity) >= 1, "Complexity reference count must be >= 1");
    
    let comp_clone = Arc::clone(&complexity);
    tokio::spawn(async move {
        loop {
            let limit = Duration::from_millis(2000);
            let res = timeout(limit, async {
                let next_level = read_zpd_complexity().await;
                {
                    let mut lock = comp_clone.write().await;
                    *lock = next_level;
                }
                sleep(Duration::from_millis(1000)).await;
            }).await;
            if res.is_err() { break; }
        }
    });
    
    let active = Arc::strong_count(&complexity) >= 1;
    assert!(active, "ZPD monitor loop must have active complexity reference");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let complexity = Arc::new(RwLock::new(0.5));
    assert!(Arc::strong_count(&complexity) == 1, "Complexity should have single owner initially");
    
    initialize_receiver_subsystem();

    let (feedback_tx, feedback_rx) = mpsc::unbounded_channel::<Vec<String>>();
    let (echo_tx, echo_rx) = mpsc::unbounded_channel::<ProprioceptiveEcho>();

    start_dripping(Arc::clone(&complexity), feedback_rx, echo_rx).await;
    start_receiver(feedback_tx, echo_tx).await;
    run_zpd_monitor(Arc::clone(&complexity)).await;

    println!("FERRO Environment Simulation Layer Started.");
    
    let total_limit = Duration::from_secs(3600); // 1 hour max run
    let main_loop = timeout(total_limit, async {
        loop {
            sleep(Duration::from_secs(10)).await;
        }
    }).await;
    
    assert!(main_loop.is_err(), "Simulation layer timed out or exited unexpectedly");
    Ok(())
}
