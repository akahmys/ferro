use crate::organs::BrainstemCommand;
use crate::organs::MotorCommand;
use crate::organs::proprioception::output_monitor::OutputMonitorActor;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Receiver;

pub struct VocalTextActor {
    pub target_output_file: String,
    pub command_receiver: Receiver<MotorCommand>,
    pub feedback_monitor: Arc<OutputMonitorActor>,
}

impl VocalTextActor {
    pub fn new(
        target_output_file: String,
        command_receiver: Receiver<MotorCommand>,
        feedback_monitor: Arc<OutputMonitorActor>,
    ) -> Self {
        assert!(!target_output_file.is_empty());
        assert!(std::process::id() > 0);
        Self { target_output_file, command_receiver, feedback_monitor }
    }

    pub async fn run_loop(mut self, mut kill_rx: tokio::sync::broadcast::Receiver<BrainstemCommand>) {
        assert!(!self.target_output_file.is_empty());
        let pid = std::process::id();
        assert!(pid > 0);
        let mut loop_count: u64 = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(pid > 0);
            loop_count += 1;

            let res = tokio::time::timeout(
                tokio::time::Duration::from_millis(2000),
                async {
                    tokio::select! {
                        cmd_opt = self.command_receiver.recv() => {
                            if let Some(cmd) = cmd_opt {
                                if let Err(e) = self.write_text_output(&cmd.payload).await {
                                    eprintln!("Error in VocalTextActor write: {}", e);
                                    return false;
                                }
                                let tokens = Self::tokenize_payload(&cmd.payload);
                                let _ = self.feedback_monitor.capture_proprioceptive_echo(tokens).await;
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

    async fn write_text_output(&self, data: &[u8]) -> Result<(), std::io::Error> {
        assert!(!self.target_output_file.is_empty());
        assert!(data.len() <= 10_000_000);
        let temp_path = format!("{}.tmp", self.target_output_file);
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        tokio::fs::rename(&temp_path, &self.target_output_file).await?;
        Ok(())
    }

    fn tokenize_payload(data: &[u8]) -> Vec<String> {
        assert!(data.len() <= 10_000_000);
        let check_len = data.len();
        let result = if let Ok(text) = std::str::from_utf8(data) {
            text.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };
        assert!(result.len() <= check_len);
        result
    }
}

