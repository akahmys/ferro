#![deny(warnings)]
#![deny(clippy::all)]

mod brainstem;
mod cerebellum;
mod midbrain;
mod hippocampus;
mod storage;
mod organs;

#[cfg(test)]
mod cognitive_tests;


use std::sync::Arc;
use tokio::sync::{mpsc, broadcast};
use brainstem::Brainstem;
use cerebellum::Cerebellum;
use organs::skin::cpu_temp::CpuTempActor;
use organs::skin::ram_free::RamFreeActor;
use organs::skin::disk_io::DiskIoActor;
use organs::skin::process_error::ProcessErrorActor;
use organs::eye::frame_delta::FrameDeltaActor;
use organs::eye::image_embedding::ImageEmbeddingActor;
use organs::ear::mfcc::MfccActor;
use organs::ear::speech_token::SpeechTokenActor;
use organs::proprioception::output_monitor::OutputMonitorActor;
use organs::motor::vocal_text::VocalTextActor;
use organs::motor::vocal_audio::VocalAudioActor;
use organs::{MotorCommand, InteroceptiveSignal, SensorySignal, BrainstemCommand, EfferenceCopy, SensoryMuteCommand};

async fn spawn_actors(
    int_tx: mpsc::Sender<InteroceptiveSignal>,
    sensory_tx: mpsc::Sender<SensorySignal>,
    motor_rx: mpsc::Receiver<MotorCommand>,
    audio_rx: mpsc::Receiver<MotorCommand>,
    cmd_tx: &broadcast::Sender<BrainstemCommand>,
    pm: Arc<OutputMonitorActor>,
    mute_tx: &broadcast::Sender<SensoryMuteCommand>,
) {
    let pid = std::process::id();
    assert!(pid > 0);
    assert!(pid != 0xffffffff);

    let cpu_actor = CpuTempActor::new(int_tx.clone(), 45.0, 100);
    let ram_actor = RamFreeActor::new(int_tx.clone(), 1024 * 1024, 100);
    let disk_actor = DiskIoActor::new(int_tx.clone(), 0.5, 100);
    let err_actor = ProcessErrorActor::new(int_tx.clone(), 0, 100);
    let eye_actor = FrameDeltaActor::new(sensory_tx.clone(), 0.01);
    let img_actor = ImageEmbeddingActor::new(sensory_tx.clone());
    let mfcc_actor = MfccActor::new(sensory_tx.clone());
    let speech_actor = SpeechTokenActor::new(sensory_tx.clone());
    let vocal_actor = VocalTextActor::new("vocal_output.txt".to_string(), motor_rx, pm);
    let audio_actor = VocalAudioActor::new(audio_rx);

    tokio::spawn(cpu_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(ram_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(disk_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(err_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(eye_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(img_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(mfcc_actor.run_loop(cmd_tx.subscribe(), mute_tx.subscribe()));
    tokio::spawn(speech_actor.run_loop(cmd_tx.subscribe(), mute_tx.subscribe()));
    tokio::spawn(vocal_actor.run_loop(cmd_tx.subscribe()));
    tokio::spawn(audio_actor.run_loop(cmd_tx.subscribe()));
}

fn spawn_receivers(
    mut sensory_rx: mpsc::Receiver<SensorySignal>,
    mut eff_rx: mpsc::Receiver<EfferenceCopy>,
    midbrain_echo_tx: mpsc::Sender<SensorySignal>,
    midbrain_eff_tx: mpsc::Sender<EfferenceCopy>,
) {
    let pid = std::process::id();
    assert!(pid > 0);
    assert!(pid != 0xffffffff);

    tokio::spawn(async move {
        let mut loop_count = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(std::process::id() > 0);
            loop_count += 1;
            let res = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                if let Some(sig) = sensory_rx.recv().await {
                    println!("[Sensory] Received: {:?}", sig);
                    if let SensorySignal::ProprioceptiveEcho(_) = sig {
                        let _ = midbrain_echo_tx.send(sig).await;
                    }
                }
            }).await;
            if res.is_err() { continue; }
        }
    });

    tokio::spawn(async move {
        let mut loop_count = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(std::process::id() > 0);
            loop_count += 1;
            let res = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                if let Some(eff) = eff_rx.recv().await {
                    println!("[Efference] Received: {:?}", eff);
                    let _ = midbrain_eff_tx.send(eff).await;
                }
            }).await;
            if res.is_err() { continue; }
        }
    });
}

fn run_test_scenario(cer: Arc<Cerebellum>, motor_tx: mpsc::Sender<MotorCommand>) {
    let pid = std::process::id();
    assert!(pid > 0);
    assert!(pid != 0xffffffff);

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        let valid_cmd = MotorCommand {
            origin_cluster_id: "cortex_01".to_string(),
            target_path: "vocal_output.txt".to_string(),
            payload: b"Hello cognitive loop".to_vec(),
            port: None,
        };
        let _ = cer.process_motor_command(valid_cmd, &motor_tx).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let invalid_cmd = MotorCommand {
            origin_cluster_id: "cortex_danger".to_string(),
            target_path: "../unsafe_path.txt".to_string(),
            payload: b"Danger".to_vec(),
            port: None,
        };
        let _ = cer.process_motor_command(invalid_cmd, &motor_tx).await;
    });
}

#[tokio::main]
async fn main() {
    let pid = std::process::id();
    assert!(pid > 0);
    assert!(pid != 0xffffffff);

    let (int_tx, int_rx) = mpsc::channel(100);
    let (sensory_tx, sensory_rx) = mpsc::channel(100);
    let (eff_tx, eff_rx) = mpsc::channel(100);
    let (panic_tx, panic_rx) = mpsc::channel(100);
    let (cmd_tx, _cmd_rx) = broadcast::channel(100);
    let (motor_tx, motor_rx) = mpsc::channel(100);
    let (_audio_tx, audio_rx) = mpsc::channel(100);

    let (mute_tx, _mute_rx) = broadcast::channel(100);
    let (surprise_tx, surprise_rx) = mpsc::channel(100);
    let (midbrain_echo_tx, midbrain_echo_rx) = mpsc::channel(100);
    let (midbrain_eff_tx, midbrain_eff_rx) = mpsc::channel(100);

    let brainstem = Brainstem::new(80.0, 1024 * 1024, cmd_tx.clone(), int_rx, panic_rx);
    let cerebellum = Arc::new(Cerebellum::new(100, sensory_tx.clone(), eff_tx, panic_tx));
    let _ = cerebellum.sensory_sender();
    let pm = Arc::new(OutputMonitorActor::new(sensory_tx.clone()));

    let midbrain = midbrain::Midbrain::new(midbrain_eff_rx, midbrain_echo_rx, mute_tx.clone(), surprise_tx, 2000, 100);
    let hippocampus = hippocampus::Hippocampus::new(100, "/memory/episodic_buffer.csv".to_string(), surprise_rx);
    let _storage = storage::ShardedJsonStorage::new("/memory/knowledge_graph");

    spawn_actors(int_tx, sensory_tx, motor_rx, audio_rx, &cmd_tx, pm, &mute_tx).await;
    spawn_receivers(sensory_rx, eff_rx, midbrain_echo_tx, midbrain_eff_tx);
    run_test_scenario(cerebellum, motor_tx);

    tokio::spawn(midbrain.run_loop(cmd_tx.subscribe()));
    tokio::spawn(hippocampus.run_loop(cmd_tx.subscribe()));

    brainstem.run_monitoring_loop().await;
}
