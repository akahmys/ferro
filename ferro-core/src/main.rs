use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time;

use ferro_core::brainstem::Brainstem;
use ferro_core::cerebellum::Cerebellum;
use ferro_core::message::{InteroceptiveSignal, MotorCommand, SensoryMuteCommand, SensorySignal};
use ferro_core::organs::{EarActor, EyeActor, MotorActor, SkinActor};

fn perform_mlockall() {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            if libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) != 0 {
                eprintln!("WARNING: mlockall failed. Continuing without locked pages.");
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("WARNING: mlockall is not supported on this platform. Continuing.");
    }
}

fn setup_memory_dir() -> PathBuf {
    let dir_str = env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    assert!(!dir_str.is_empty(), "Error: memory directory string is empty");
    let path = PathBuf::from(dir_str);
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    assert!(path.exists(), "Error: failed to create memory directory");
    path
}

fn poll_files(
    memory_dir: &std::path::Path,
    brainstem_tx: &mpsc::Sender<InteroceptiveSignal>,
    eye_tx: &mpsc::Sender<SensorySignal>,
    ear_tx: &mpsc::Sender<SensorySignal>,
) {
    let interoceptive_path = memory_dir.join("interoceptive_signals.json");
    if let Ok(content) = fs::read_to_string(&interoceptive_path) {
        let _ = fs::remove_file(&interoceptive_path);
        if let Ok(signals) = serde_json::from_str::<Vec<InteroceptiveSignal>>(&content) {
            for sig in signals {
                let _ = brainstem_tx.try_send(sig);
            }
        }
    }

    let sensory_path = memory_dir.join("stimulus/sensory_signals.json");
    if let Ok(content) = fs::read_to_string(&sensory_path) {
        let _ = fs::remove_file(&sensory_path);
        if let Ok(signals) = serde_json::from_str::<Vec<SensorySignal>>(&content) {
            for sig in signals {
                match sig {
                    SensorySignal::FrameDelta(_) | SensorySignal::ImageEmbedding(_) => {
                        let _ = eye_tx.try_send(sig);
                    }
                    SensorySignal::Mfcc(_) | SensorySignal::SpeechToken(_) | SensorySignal::ProprioceptiveEcho(_) => {
                        let _ = ear_tx.try_send(sig);
                    }
                    SensorySignal::LogHash(_) => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    perform_mlockall();
    let memory_dir = setup_memory_dir();
    let terminate = Arc::new(AtomicBool::new(false));

    let (brainstem_tx, mut brainstem_rx) = mpsc::channel::<InteroceptiveSignal>(100);
    let (_skin_tx, skin_rx) = mpsc::channel::<InteroceptiveSignal>(100);
    let (eye_tx, eye_rx) = mpsc::channel::<SensorySignal>(100);
    let (ear_tx, ear_rx) = mpsc::channel::<SensorySignal>(100);
    let (_mute_tx, mute_rx) = mpsc::channel::<SensoryMuteCommand>(100);
    let (motor_tx, motor_rx) = mpsc::channel::<MotorCommand>(100);

    let cerebellum = Arc::new(Cerebellum::new(Arc::clone(&terminate), memory_dir.clone()));

    let mut skin = SkinActor::new(skin_rx);
    let brainstem_tx_clone = brainstem_tx.clone();
    tokio::spawn(async move { skin.run(brainstem_tx_clone).await });

    let mut eye = EyeActor::new(eye_rx);
    tokio::spawn(async move { eye.run().await });

    let mut ear = EarActor::new(ear_rx, mute_rx);
    tokio::spawn(async move { ear.run().await });

    let mut motor = MotorActor::new(motor_rx, Arc::clone(&cerebellum));
    tokio::spawn(async move { motor.run().await });

    let mut brainstem = Brainstem::new(Arc::clone(&terminate));
    let mut interval = time::interval(Duration::from_millis(100));
    let mut last_tick = Instant::now();

    // 運動テスト用チャネルの公開やシミュレーション実行に必要なコード
    // （検証用に、メインスレッドからmotor_tx経由でコマンド送信可能）
    let motor_tx_clone = motor_tx.clone();
    let memory_dir_clone = memory_dir.clone();
    tokio::spawn(async move {
        // テスト用スレッドで一定時間後に不正コマンドを発行して自死をトリガーするシミュレーション
        tokio::time::sleep(Duration::from_millis(1500)).await;
        // 通常コマンド発行
        let _ = motor_tx_clone.send(MotorCommand {
            origin_cluster_id: "cluster_0".to_string(),
            target_path: memory_dir_clone.join("action/vocal_text.json").to_string_lossy().to_string(),
            payload: b"Hello from FERRO Core!".to_vec(),
            port: None,
        }).await;

        tokio::time::sleep(Duration::from_millis(1500)).await;
        // 不正コマンド発行 (禁止ポート 8080 へのアクセス)
        let _ = motor_tx_clone.send(MotorCommand {
            origin_cluster_id: "cluster_bad".to_string(),
            target_path: memory_dir_clone.join("action/vocal_text.json").to_string_lossy().to_string(),
            payload: b"Bad Command Attempt".to_vec(),
            port: Some(8080),
        }).await;
    });

    while !brainstem.should_terminate() {
        let _ = interval.tick().await;
        let now = Instant::now();
        let elapsed = now.duration_since(last_tick);
        last_tick = now;

        if elapsed > Duration::from_millis(110) {
            eprintln!("WARNING: Cerebellum cycle jitter exceeded 10ms: {:?}", elapsed);
        }

        poll_files(&memory_dir, &brainstem_tx, &eye_tx, &ear_tx);

        while let Ok(signal) = brainstem_rx.try_recv() {
            brainstem.handle_signal(signal);
        }
    }

    println!("FERRO Core clean exit sequence completed.");
}
