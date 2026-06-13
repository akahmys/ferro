use crate::organs::{SensorySignal, BrainstemCommand, SensoryMuteCommand};
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct SpeechTokenActor {
    pub sender: Sender<SensorySignal>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct AuditoryStimulus {
    timestamp: u64,
    speech_tokens: Vec<String>,
}

impl SpeechTokenActor {
    pub fn new(sender: Sender<SensorySignal>) -> Self {
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(pid != 0xffffffff);
        Self { sender }
    }

    pub async fn run_loop(
        self,
        mut kill_rx: Receiver<BrainstemCommand>,
        mut mute_rx: Receiver<SensoryMuteCommand>,
    ) {
        let pid = std::process::id();
        assert!(pid > 0);
        let mut loop_count: u64 = 0;
        let mut is_muted = false;
        let mut last_timestamp = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            loop_count += 1;
            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(1000),
                async {
                    tokio::select! {
                        tokens_opt = Self::read_speech_tokens(is_muted) => {
                            if let Some((ts, tokens)) = tokens_opt {
                                if ts != last_timestamp {
                                    last_timestamp = ts;
                                    let _ = self.sender.send(SensorySignal::SpeechToken(tokens)).await;
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                            false
                        }
                        cmd_res = kill_rx.recv() => {
                            matches!(cmd_res, Ok(BrainstemCommand::ForceSleep))
                        }
                        Ok(mute_cmd) = mute_rx.recv() => {
                            is_muted = mute_cmd.mute;
                            false
                        }
                    }
                }
            ).await;
            if let Ok(true) = res {
                break;
            }
        }
    }

    async fn read_speech_tokens(is_muted: bool) -> Option<(u64, Vec<String>)> {
        let pid = std::process::id();
        assert!(pid > 0);
        if is_muted {
            assert!(is_muted);
            None
        } else {
            assert!(!is_muted);
            let path = std::path::Path::new("/memory/stimulus/auditory.json");
            if !path.is_file() {
                return None;
            }
            let content = tokio::fs::read_to_string(path).await.ok()?;
            let stimulus: AuditoryStimulus = serde_json::from_str(&content).ok()?;
            Some((stimulus.timestamp, stimulus.speech_tokens))
        }
    }
}
