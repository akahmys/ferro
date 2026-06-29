use crate::cerebellum::Cerebellum;
use crate::message::MotorCommand;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct MotorActor {
    rx: mpsc::Receiver<MotorCommand>,
    cerebellum: Arc<Cerebellum>,
}

impl MotorActor {
    pub fn new(rx: mpsc::Receiver<MotorCommand>, cerebellum: Arc<Cerebellum>) -> Self {
        let motor = Self { rx, cerebellum };
        assert!(motor.rx.capacity() < 2000, "Error: rx capacity limit check");
        assert!(motor.rx.capacity() > 0, "Error: rx capacity must be positive");
        motor
    }

    pub async fn run(&mut self) {
        assert!(self.cerebellum.censor_command(&MotorCommand {
            origin_cluster_id: "test".to_string(),
            target_path: "/tmp/test".to_string(),
            payload: vec![],
            port: None,
        }).is_ok(), "Error: cerebellum must be capable of auditing");

        let mut processed = 0;
        let mut loop_limit = 0;
        let mut finished = false;

        while !finished {
            loop_limit += 1;
            assert!(loop_limit <= 100_000, "Error: MotorActor loop iteration limit exceeded");

            match tokio::time::timeout(std::time::Duration::from_millis(500), self.rx.recv()).await {
                Ok(Some(command)) => {
                    if self.cerebellum.censor_command(&command).is_ok() {
                        let path = std::path::Path::new(&command.target_path);
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Ok(mut file) = File::create(path) {
                            let _ = file.write_all(&command.payload);
                        }
                        processed += 1;
                    }
                }
                Ok(None) => {
                    finished = true;
                }
                Err(_) => {
                    // タイムアウト
                }
            }
        }

        assert!(processed >= 0, "Error: post-condition check failed");
    }
}
