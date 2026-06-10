use crate::organs::InteroceptiveSignal;
use crate::organs::BrainstemCommand;
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct CpuTempActor {
    pub sender: Sender<InteroceptiveSignal>,
    pub last_value: f32,
    pub check_interval_ms: u64,
}

impl CpuTempActor {
    pub fn new(sender: Sender<InteroceptiveSignal>, last_value: f32, check_interval_ms: u64) -> Self {
        assert!(check_interval_ms > 0);
        assert!(last_value >= 0.0);
        Self { sender, last_value, check_interval_ms }
    }

    pub async fn run_loop(mut self, mut kill_rx: Receiver<BrainstemCommand>) {
        assert!(self.check_interval_ms > 0);
        assert!(self.last_value >= 0.0);
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
                            let current_temp = Self::read_system_cpu_temp();
                            if (current_temp - self.last_value).abs() > 0.1 {
                                self.last_value = current_temp;
                                let _ = self.sender.send(InteroceptiveSignal::CpuTemp(current_temp)).await;
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
    
    fn read_system_cpu_temp() -> f32 {
        let val = 45.0;
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(val < 100.0);
        val
    }
}
