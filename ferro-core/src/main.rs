#![deny(warnings)]
#![deny(clippy::all)]

mod setup;
mod cerebrum;
mod cortex;
mod brainstem;
mod cerebellum;
mod midbrain;
mod hippocampus;
mod storage;
mod organs;

#[cfg(test)]
mod cognitive_tests;

use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, Mutex};
use brainstem::Brainstem;
use cerebellum::Cerebellum;
use organs::proprioception::output_monitor::OutputMonitorActor;
use storage::manager::{StorageManager, get_safe_path};
use cerebrum::Cerebrum;
use cortex::Cortex;

#[tokio::main]
async fn main() {
    let pid = std::process::id();
    assert!(pid > 0); assert!(pid != 0xffffffff);

    let (int_tx, int_rx) = mpsc::channel(100);
    let (sensory_tx, sensory_rx) = mpsc::channel(100);
    let (eff_tx, eff_rx) = mpsc::channel(100);
    let (panic_tx, panic_rx) = mpsc::channel(100);
    let (cmd_tx, _cmd_rx) = broadcast::channel(100);
    let (motor_tx, motor_rx) = mpsc::channel(100);
    let (_audio_tx, audio_rx) = mpsc::channel(100);

    let (mute_tx, _mute_rx) = broadcast::channel(100);
    let (midbrain_surprise_tx, mut midbrain_surprise_rx) = mpsc::channel(100);
    let (hippo_surprise_tx, hippo_surprise_rx) = mpsc::channel(100);
    let (cerebrum_surprise_tx, cerebrum_surprise_rx) = mpsc::channel(100);

    let (midbrain_echo_tx, midbrain_echo_rx) = mpsc::channel(100);
    let (midbrain_eff_tx, midbrain_eff_rx) = mpsc::channel(100);
    let (interaction_tx, interaction_rx) = mpsc::channel(100);

    let (phase_tx, _phase_rx) = broadcast::channel(100);

    tokio::spawn(async move {
        let mut count = 0;
        while let Some(surprise) = midbrain_surprise_rx.recv().await {
            assert!(count < 1_000_000_000); count += 1;
            let _ = hippo_surprise_tx.send(surprise).await;
            let _ = cerebrum_surprise_tx.send(surprise).await;
        }
    });

    let storage_json_path = get_safe_path("/memory/knowledge_graph");
    let storage_redb_path = get_safe_path("/memory/storage.redb");
    let surprise_history_path = get_safe_path("/memory/surprise_history.csv");
    let episodic_buffer_path = get_safe_path("/memory/episodic_buffer.csv");

    let storage = Arc::new(StorageManager::new(storage_json_path, storage_redb_path, 5000));
    let cortex = Arc::new(Cortex::new(storage.clone()));
    let cerebrum = Arc::new(Mutex::new(Cerebrum::new(phase_tx, surprise_history_path, 1000)));

    let brainstem = Brainstem::new(80.0, 1024 * 1024, cmd_tx.clone(), int_rx, panic_rx);
    let cerebellum = Arc::new(Cerebellum::new(100, sensory_tx.clone(), eff_tx, panic_tx));
    let pm = Arc::new(OutputMonitorActor::new(sensory_tx.clone()));

    let midbrain = midbrain::Midbrain::new(midbrain_eff_rx, midbrain_echo_rx, mute_tx.clone(), midbrain_surprise_tx, 2000, 100);
    let hippocampus = hippocampus::Hippocampus::new(100, episodic_buffer_path.to_string_lossy().to_string(), hippo_surprise_rx);

    setup::spawn_actors(int_tx, sensory_tx, motor_rx, audio_rx, &cmd_tx, pm, &mute_tx).await;
    setup::spawn_receivers(sensory_rx, eff_rx, midbrain_echo_tx, midbrain_eff_tx, interaction_tx);
    setup::run_test_scenario(cerebellum, motor_tx);

    tokio::spawn(midbrain.run_loop(cmd_tx.subscribe()));
    tokio::spawn(hippocampus.run_loop(cmd_tx.subscribe()));
    tokio::spawn(Cerebrum::run_loop(cerebrum, cortex, interaction_rx, cerebrum_surprise_rx, cmd_tx.subscribe()));

    brainstem.run_monitoring_loop().await;
}
