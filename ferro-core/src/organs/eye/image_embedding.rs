use crate::organs::SensorySignal;
use crate::organs::BrainstemCommand;
use tokio::sync::mpsc::Sender;
use tokio::sync::broadcast::Receiver;

pub struct ImageEmbeddingActor {
    pub sender: Sender<SensorySignal>,
}

impl ImageEmbeddingActor {
    pub fn new(sender: Sender<SensorySignal>) -> Self {
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(pid != 0xffffffff);
        Self { sender }
    }

    pub async fn run_loop(self, mut kill_rx: Receiver<BrainstemCommand>) {
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
                        embedding_opt = Self::read_image_embedding() => {
                            if let Some(embedding) = embedding_opt {
                                let _ = self.sender.send(SensorySignal::ImageEmbedding(embedding)).await;
                            }
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
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

    async fn read_image_embedding() -> Option<Vec<f32>> {
        let embedding = vec![0.1, 0.2, 0.3];
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(embedding.len() == 3);
        Some(embedding)
    }
}
