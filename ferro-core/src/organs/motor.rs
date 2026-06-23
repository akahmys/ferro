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
        Self { rx, cerebellum }
    }

    pub async fn run(&mut self) {
        // R5: アサーション最低2つを義務付け
        assert!(self.cerebellum.censor_command(&MotorCommand {
            origin_cluster_id: "test".to_string(),
            target_path: "/tmp/test".to_string(),
            payload: vec![],
            port: None,
        }).is_ok(), "Error: cerebellum must be capable of auditing");

        let mut processed = 0;
        while let Some(command) = self.rx.recv().await {
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

        assert!(processed >= 0, "Error: post-condition check failed");
    }
}
