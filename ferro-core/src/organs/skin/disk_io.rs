use crate::organs::InteroceptiveSignal;
use crate::organs::BrainstemCommand;
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct DiskIoActor {
    pub sender: Sender<InteroceptiveSignal>,
    pub last_value: f64,
    pub check_interval_ms: u64,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct PhysicalStimulus {
    disk_io: f64,
}

impl DiskIoActor {
    pub fn new(sender: Sender<InteroceptiveSignal>, last_value: f64, check_interval_ms: u64) -> Self {
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
                            let current_io = Self::read_system_disk_io().await;
                            if (current_io - self.last_value).abs() > 0.1 {
                                self.last_value = current_io;
                                let _ = self.sender.send(InteroceptiveSignal::DiskIo(current_io)).await;
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

    async fn read_system_disk_io() -> f64 {
        let pid = std::process::id();
        assert!(pid > 0);
        let path = std::path::Path::new("/memory/stimulus/physical.json");
        if !path.is_file() {
            return 0.5;
        }
        if let Ok(content) = tokio::fs::read_to_string(path).await {
            if let Ok(stimulus) = serde_json::from_str::<PhysicalStimulus>(&content) {
                assert!(stimulus.disk_io >= 0.0);
                return stimulus.disk_io;
            }
        }
        0.5
    }
}
