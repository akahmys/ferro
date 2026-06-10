use crate::organs::InteroceptiveSignal;
use crate::organs::BrainstemCommand;
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct ProcessErrorActor {
    pub sender: Sender<InteroceptiveSignal>,
    pub last_value: u32,
    pub check_interval_ms: u64,
}

impl ProcessErrorActor {
    pub fn new(sender: Sender<InteroceptiveSignal>, last_value: u32, check_interval_ms: u64) -> Self {
        assert!(check_interval_ms > 0);
        assert!(last_value <= 100_000);
        Self { sender, last_value, check_interval_ms }
    }

    pub async fn run_loop(mut self, mut kill_rx: Receiver<BrainstemCommand>) {
        assert!(self.check_interval_ms > 0);
        assert!(self.last_value <= 100_000);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(self.check_interval_ms));
        let mut loop_count: u64 = 0;
        loop {
            let pid = std::process::id();
            assert!(pid > 0);
            assert!(loop_count < 1_000_000_000);
            loop_count += 1;

            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(self.check_interval_ms * 2 + 1000),
                async {
                    tokio::select! {
                        _ = interval.tick() => {
                            let current_errs = Self::read_process_errors();
                            if current_errs != self.last_value {
                                self.last_value = current_errs;
                                let _ = self.sender.send(InteroceptiveSignal::ProcessError(current_errs)).await;
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

    fn read_process_errors() -> u32 {
        let val = 0;
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(val == 0);
        val
    }
}
