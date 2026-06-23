use crate::message::InteroceptiveSignal;
use tokio::sync::mpsc;

pub struct SkinActor {
    rx: mpsc::Receiver<InteroceptiveSignal>,
}

impl SkinActor {
    pub fn new(rx: mpsc::Receiver<InteroceptiveSignal>) -> Self {
        Self { rx }
    }

    pub async fn run(&mut self, brainstem_tx: mpsc::Sender<InteroceptiveSignal>) {
        // R5: アサーション最低2つを義務付け
        assert!(brainstem_tx.capacity() > 0, "Error: brainstem channel must not be full");

        while let Some(signal) = self.rx.recv().await {
            let res = brainstem_tx.send(signal).await;
            assert!(res.is_ok(), "Error: failed to send signal to brainstem");
        }
    }
}
