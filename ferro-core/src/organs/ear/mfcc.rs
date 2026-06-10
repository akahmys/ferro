use crate::organs::{SensorySignal, BrainstemCommand, SensoryMuteCommand};
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct MfccActor {
    pub sender: Sender<SensorySignal>,
}

impl MfccActor {
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
        loop {
            assert!(loop_count < 1_000_000_000);
            loop_count += 1;
            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(1000),
                async {
                    tokio::select! {
                        mfcc_opt = Self::read_mfcc(is_muted) => {
                            if let Some(mfcc) = mfcc_opt {
                                let _ = self.sender.send(SensorySignal::Mfcc(mfcc)).await;
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

    async fn read_mfcc(is_muted: bool) -> Option<Vec<f32>> {
        let pid = std::process::id();
        assert!(pid > 0);
        if is_muted {
            assert!(is_muted);
            None
        } else {
            assert!(!is_muted);
            Some(vec![1.0, -1.0])
        }
    }
}
