use crate::message::SensorySignal;
use tokio::sync::mpsc;

pub struct EyeActor {
    rx: mpsc::Receiver<SensorySignal>,
}

impl EyeActor {
    pub fn new(rx: mpsc::Receiver<SensorySignal>) -> Self {
        Self { rx }
    }

    pub async fn run(&mut self) {
        let mut frame_count = 0;
        assert!(frame_count == 0, "Error: initial frame count must be zero");
        while let Some(signal) = self.rx.recv().await {
            if let SensorySignal::FrameDelta(delta) = signal {
                assert!(delta >= 0.0, "Error: delta must be non-negative");
                frame_count += 1;
            }
        }

        assert!(frame_count >= 0, "Error: frame count should not be negative");
    }
}
