use crate::organs::SensorySignal;
use crate::organs::BrainstemCommand;
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct FrameDeltaActor {
    pub sender: Sender<SensorySignal>,
    pub threshold: f64,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct VisualStimulus {
    frame_delta: f64,
}

impl FrameDeltaActor {
    pub fn new(sender: Sender<SensorySignal>, threshold: f64) -> Self {
        assert!(threshold >= 0.0);
        assert!(threshold <= 1.0);
        Self { sender, threshold }
    }

    pub async fn run_loop(self, mut kill_rx: Receiver<BrainstemCommand>) {
        assert!(self.threshold >= 0.0);
        assert!(self.threshold <= 1.0);
        let mut loop_count: u64 = 0;
        loop {
            let pid = std::process::id();
            assert!(pid > 0);
            assert!(loop_count < 1_000_000_000);
            loop_count += 1;

            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(1000),
                async {
                    tokio::select! {
                        delta_opt = Self::read_frame_delta() => {
                            if let Some(delta) = delta_opt {
                                if delta >= self.threshold {
                                     let _ = self.sender.send(SensorySignal::FrameDelta(delta)).await;
                                }
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            false
                        }
                        cmd_res = kill_rx.recv() => {
                            matches!(cmd_res, Ok(BrainstemCommand::ForceSleep))
                        }
                    }
                }
            ).await;

            if let Ok(true) = res {
                break;
            }
        }
    }

    async fn read_frame_delta() -> Option<f64> {
        let pid = std::process::id();
        assert!(pid > 0);
        let path = std::path::Path::new("/memory/stimulus/visual.json");
        if !path.is_file() {
            return None;
        }
        let content = tokio::fs::read_to_string(path).await.ok()?;
        let stimulus: VisualStimulus = serde_json::from_str(&content).ok()?;
        Some(stimulus.frame_delta)
    }
}
