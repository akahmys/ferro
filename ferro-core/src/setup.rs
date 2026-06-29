use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::message::{InteroceptiveSignal, SensorySignal};

pub fn perform_mlockall() {
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

pub fn setup_memory_dir() -> PathBuf {
    let dir_str = std::env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    assert!(!dir_str.is_empty(), "Error: memory directory string is empty");
    let path = PathBuf::from(dir_str);
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    assert!(path.exists(), "Error: failed to create memory directory");
    path
}

pub fn poll_files(
    memory_dir: &Path,
    brainstem_tx: &mpsc::Sender<InteroceptiveSignal>,
    eye_tx: &mpsc::Sender<SensorySignal>,
    ear_tx: &mpsc::Sender<SensorySignal>,
    midbrain: &Arc<crate::midbrain::Midbrain>,
    hippocampus: &Arc<crate::hippocampus::Hippocampus>,
) {
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
    assert!(brainstem_tx.capacity() < 2000, "Error: brainstem_tx capacity check");

    let interoceptive_path = memory_dir.join("interoceptive_signals.json");
    if let Ok(content) = fs::read_to_string(&interoceptive_path) {
        let _ = fs::remove_file(&interoceptive_path);
        if let Ok(signals) = serde_json::from_str::<Vec<InteroceptiveSignal>>(&content) {
            let mut limit = 0;
            for sig in signals {
                limit += 1;
                assert!(limit <= 1000, "Error: Loop limit exceeded in poll_files interoceptive");
                let _ = brainstem_tx.try_send(sig);
            }
        }
    }

    let efference_path = memory_dir.join("stimulus/efference_copy.json");
    if let Ok(content) = fs::read_to_string(&efference_path) {
        let _ = fs::remove_file(&efference_path);
        if let Ok(copies) = serde_json::from_str::<Vec<crate::message::EfferenceCopy>>(&content) {
            let mut limit = 0;
            for copy in copies {
                limit += 1;
                assert!(limit <= 1000, "Error: Loop limit exceeded in poll_files efference");
                let midbrain_clone = midbrain.clone();
                tokio::spawn(async move {
                    let _ = midbrain_clone.handle_efference_copy(copy).await;
                });
            }
        }
    }

    let sensory_path = memory_dir.join("stimulus/sensory_signals.json");
    if let Ok(content) = fs::read_to_string(&sensory_path) {
        let _ = fs::remove_file(&sensory_path);
        if let Ok(signals) = serde_json::from_str::<Vec<SensorySignal>>(&content) {
            let mut limit = 0;
            for sig in signals {
                limit += 1;
                assert!(limit <= 1000, "Error: Loop limit exceeded in poll_files sensory");
                match sig {
                    SensorySignal::FrameDelta(_) | SensorySignal::ImageEmbedding(_) => {
                        let _ = eye_tx.try_send(sig);
                    }
                    SensorySignal::Mfcc(_) | SensorySignal::SpeechToken(_) => {
                        let _ = ear_tx.try_send(sig);
                    }
                    SensorySignal::ProprioceptiveEcho(tokens) => {
                        let midbrain_clone = midbrain.clone();
                        let hippocampus_clone = hippocampus.clone();
                        tokio::spawn(async move {
                            if let Ok(surprise) = midbrain_clone.handle_proprioceptive_echo(tokens.clone()) {
                                let slot = crate::hippocampus::EpisodicSlot {
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
                                    input: format!("{:?}", tokens),
                                    output: "".to_string(),
                                    surprise,
                                };
                                let _ = hippocampus_clone.record_episode(slot);
                            }
                        });
                    }
                    SensorySignal::LogHash(_) => {}
                }
            }
        }
    }
}
