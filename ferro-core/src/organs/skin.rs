use crate::message::InteroceptiveSignal;
use tokio::sync::mpsc;

pub struct SkinActor {
    rx: mpsc::Receiver<InteroceptiveSignal>,
}

impl SkinActor {
    pub fn new(rx: mpsc::Receiver<InteroceptiveSignal>) -> Self {
        let skin = Self { rx };
        assert!(skin.rx.capacity() < 2000, "Error: rx capacity limit check");
        assert!(skin.rx.capacity() > 0, "Error: rx capacity must be positive");
        skin
    }

    pub async fn run(&mut self, brainstem_tx: mpsc::Sender<InteroceptiveSignal>) {
        assert!(brainstem_tx.capacity() > 0, "Error: brainstem channel must not be full");
        assert!(self.rx.capacity() < 2000, "Error: rx capacity limit check");

        let mut loop_limit = 0;
        let mut finished = false;

        while !finished {
            loop_limit += 1;
            assert!(loop_limit <= 100_000, "Error: SkinActor loop iteration limit exceeded");

            match tokio::time::timeout(std::time::Duration::from_millis(500), self.rx.recv()).await {
                Ok(Some(signal)) => {
                    let res = brainstem_tx.send(signal).await;
                    assert!(res.is_ok(), "Error: failed to send signal to brainstem");
                }
                Ok(None) => {
                    finished = true;
                }
                Err(_) => {
                    // タイムアウト
                }
            }
        }
    }
}
