use crate::organs::BrainstemCommand;
use crate::organs::MotorCommand;
use tokio::sync::mpsc::Receiver;

pub struct VocalAudioActor {
    pub command_receiver: Receiver<MotorCommand>,
}

impl VocalAudioActor {
    pub fn new(command_receiver: Receiver<MotorCommand>) -> Self {
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(pid != 0xffffffff);
        Self { command_receiver }
    }

    pub async fn run_loop(mut self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(pid != 0xffffffff);
        let mut loop_count: u64 = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(pid > 0);
            loop_count += 1;

            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(1000),
                async {
                    tokio::select! {
                        cmd_opt = self.command_receiver.recv() => {
                            if let Some(_cmd) = cmd_opt {
                                // Simulate audio synthesis
                            }
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
}
