use crate::message::{SensoryMuteCommand, SensorySignal};
use crate::cortex::Cortex;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub struct EarActor {
    rx: mpsc::Receiver<SensorySignal>,
    mute_rx: mpsc::Receiver<SensoryMuteCommand>,
    is_muted: bool,
    cortex: Arc<RwLock<Cortex>>,
}

impl EarActor {
    pub fn new(
        rx: mpsc::Receiver<SensorySignal>,
        mute_rx: mpsc::Receiver<SensoryMuteCommand>,
        cortex: Arc<RwLock<Cortex>>,
    ) -> Self {
        let ear = Self {
            rx,
            mute_rx,
            is_muted: false,
            cortex,
        };
        assert!(!ear.is_muted, "Error: initial mute state must be false");
        assert!(ear.rx.capacity() < 2000, "Error: rx capacity limit check");
        ear
    }

    pub async fn run(&mut self) {
        assert!(!self.is_muted, "Error: initial mute state must be false");

        let mut processed_signals = 0;
        let mut loop_limit = 0;

        loop {
            loop_limit += 1;
            assert!(loop_limit <= 100_000, "Error: EarActor loop iteration limit exceeded");

            let select_fut = async {
                tokio::select! {
                    res_mute = self.mute_rx.recv() => {
                        if let Some(mute_cmd) = res_mute {
                            self.is_muted = mute_cmd.mute;
                            true
                        } else {
                            false
                        }
                    }
                    res_sig = self.rx.recv() => {
                        if let Some(signal) = res_sig {
                            if !self.is_muted {
                                match signal {
                                    SensorySignal::Mfcc(ref data) => {
                                        processed_signals += 1;
                                        // ID 2 (聴覚入力ノード) の活性とPEを更新
                                        let mut guard = self.cortex.write().unwrap();
                                        let _ = guard.arena.with_mut_node(2, |node| {
                                            node.activity = 1.0;
                                            node.prediction_error = data.iter().map(|&x| x as f64).sum::<f64>();
                                        });
                                    }
                                    SensorySignal::SpeechToken(_) => {
                                        processed_signals += 1;
                                    }
                                    _ => {}
                                }
                            }
                            true
                        } else {
                            false
                        }
                    }
                }
            };

            match tokio::time::timeout(std::time::Duration::from_millis(500), select_fut).await {
                Ok(keep_going) => {
                    if !keep_going {
                        break;
                    }
                }
                Err(_) => {
                    // タイムアウト
                }
            }
        }

        assert!(processed_signals >= 0, "Error: processed signals cannot be negative");
    }
}
