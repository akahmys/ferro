use crate::message::SensorySignal;
use crate::cortex::Cortex;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub struct EyeActor {
    rx: mpsc::Receiver<SensorySignal>,
    cortex: Arc<RwLock<Cortex>>,
}

impl EyeActor {
    pub fn new(rx: mpsc::Receiver<SensorySignal>, cortex: Arc<RwLock<Cortex>>) -> Self {
        let eye = Self { rx, cortex };
        assert!(eye.rx.capacity() < 2000, "Error: rx capacity limit check");
        assert!(eye.rx.capacity() > 0, "Error: rx capacity must be positive");
        eye
    }

    pub async fn run(&mut self) {
        let mut frame_count = 0;
        assert!(frame_count == 0, "Error: initial frame count must be zero");

        let mut loop_limit = 0;
        let mut finished = false;

        while !finished {
            loop_limit += 1;
            assert!(loop_limit <= 100_000, "Error: EyeActor loop iteration limit exceeded");

            match tokio::time::timeout(std::time::Duration::from_millis(500), self.rx.recv()).await {
                Ok(Some(signal)) => {
                    if let SensorySignal::FrameDelta(delta) = signal {
                        assert!(delta >= 0.0, "Error: delta must be non-negative");
                        frame_count += 1;

                        // ID 1 (入力ノード) の活性とPEを更新
                        let mut guard = self.cortex.write().unwrap();
                        let _ = guard.arena.with_mut_node(1, |node| {
                            node.activity = 1.0;
                            node.prediction_error = delta;
                        });
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

        assert!(frame_count >= 0, "Error: frame count should not be negative");
    }
}
