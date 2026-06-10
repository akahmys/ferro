use std::sync::Arc;
use tokio::sync::{mpsc, broadcast};
use crate::organs::{MotorCommand, InteroceptiveSignal, SensorySignal, BrainstemCommand, EfferenceCopy, SensoryMuteCommand};
use crate::organs::skin::{cpu_temp::CpuTempActor, ram_free::RamFreeActor, disk_io::DiskIoActor, process_error::ProcessErrorActor};
use crate::organs::eye::{frame_delta::FrameDeltaActor, image_embedding::ImageEmbeddingActor};
use crate::organs::ear::{mfcc::MfccActor, speech_token::SpeechTokenActor};
use crate::organs::proprioception::output_monitor::OutputMonitorActor;
use crate::organs::motor::{vocal_text::VocalTextActor, vocal_audio::VocalAudioActor};
use crate::cerebellum::Cerebellum;

pub async fn spawn_actors(
    int_tx: mpsc::Sender<InteroceptiveSignal>,
    sensory_tx: mpsc::Sender<SensorySignal>,
    motor_rx: mpsc::Receiver<MotorCommand>,
    audio_rx: mpsc::Receiver<MotorCommand>,
    cmd_tx: &broadcast::Sender<BrainstemCommand>,
    pm: Arc<OutputMonitorActor>,
    mute_tx: &broadcast::Sender<SensoryMuteCommand>,
) {
    assert!(std::process::id() > 0);
    assert!(Arc::strong_count(&pm) >= 1);

    tokio::spawn(CpuTempActor::new(int_tx.clone(), 45.0, 100).run_loop(cmd_tx.subscribe()));
    tokio::spawn(RamFreeActor::new(int_tx.clone(), 1024 * 1024, 100).run_loop(cmd_tx.subscribe()));
    tokio::spawn(DiskIoActor::new(int_tx.clone(), 0.5, 100).run_loop(cmd_tx.subscribe()));
    tokio::spawn(ProcessErrorActor::new(int_tx.clone(), 0, 100).run_loop(cmd_tx.subscribe()));
    tokio::spawn(FrameDeltaActor::new(sensory_tx.clone(), 0.01).run_loop(cmd_tx.subscribe()));
    tokio::spawn(ImageEmbeddingActor::new(sensory_tx.clone()).run_loop(cmd_tx.subscribe()));
    tokio::spawn(MfccActor::new(sensory_tx.clone()).run_loop(cmd_tx.subscribe(), mute_tx.subscribe()));
    tokio::spawn(SpeechTokenActor::new(sensory_tx.clone()).run_loop(cmd_tx.subscribe(), mute_tx.subscribe()));
    tokio::spawn(VocalTextActor::new("vocal_output.txt".to_string(), motor_rx, pm).run_loop(cmd_tx.subscribe()));
    tokio::spawn(VocalAudioActor::new(audio_rx).run_loop(cmd_tx.subscribe()));
}

pub fn spawn_receivers(
    mut sensory_rx: mpsc::Receiver<SensorySignal>,
    mut eff_rx: mpsc::Receiver<EfferenceCopy>,
    midbrain_echo_tx: mpsc::Sender<SensorySignal>,
    midbrain_eff_tx: mpsc::Sender<EfferenceCopy>,
    interaction_tx: mpsc::Sender<()>,
) {
    assert!(std::process::id() > 0);
    assert!(mpsc::Sender::strong_count(&interaction_tx) >= 1);

    let int_tx1 = interaction_tx.clone();
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            assert!(count < 1_000_000_000); count += 1;
            let res = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                if let Some(sig) = sensory_rx.recv().await {
                    let _ = int_tx1.try_send(());
                    if let SensorySignal::ProprioceptiveEcho(_) = sig {
                        let _ = midbrain_echo_tx.send(sig).await;
                    }
                }
            }).await;
            if res.is_err() { continue; }
        }
    });

    let int_tx2 = interaction_tx;
    tokio::spawn(async move {
        let mut count = 0;
        loop {
            assert!(count < 1_000_000_000); count += 1;
            let res = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                if let Some(eff) = eff_rx.recv().await {
                    let _ = int_tx2.try_send(());
                    let _ = midbrain_eff_tx.send(eff).await;
                }
            }).await;
            if res.is_err() { continue; }
        }
    });
}

pub fn run_test_scenario(cer: Arc<Cerebellum>, motor_tx: mpsc::Sender<MotorCommand>) {
    assert!(std::process::id() > 0);
    assert!(Arc::strong_count(&cer) >= 1);

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        let _ = cer.process_motor_command(MotorCommand {
            origin_cluster_id: "cortex_01".to_string(),
            target_path: "vocal_output.txt".to_string(),
            payload: b"Hello cognitive loop".to_vec(),
            port: None,
        }, &motor_tx).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let _ = cer.process_motor_command(MotorCommand {
            origin_cluster_id: "cortex_danger".to_string(),
            target_path: "../unsafe_path.txt".to_string(),
            payload: b"Danger".to_vec(),
            port: None,
        }, &motor_tx).await;
    });
}
