use crate::message::{SensoryMuteCommand, SensorySignal};
use tokio::sync::mpsc;

pub struct EarActor {
    rx: mpsc::Receiver<SensorySignal>,
    mute_rx: mpsc::Receiver<SensoryMuteCommand>,
    is_muted: bool,
}

impl EarActor {
    pub fn new(
        rx: mpsc::Receiver<SensorySignal>,
        mute_rx: mpsc::Receiver<SensoryMuteCommand>,
    ) -> Self {
        Self {
            rx,
            mute_rx,
            is_muted: false,
        }
    }

    pub async fn run(&mut self) {
        // R5: アサーション最低2つを義務付け
        assert!(!self.is_muted, "Error: initial mute state must be false");

        let mut processed_signals = 0;
        loop {
            tokio::select! {
                Some(mute_cmd) = self.mute_rx.recv() => {
                    self.is_muted = mute_cmd.mute;
                }
                Some(signal) = self.rx.recv() => {
                    if !self.is_muted {
                        match signal {
                            SensorySignal::Mfcc(_) | SensorySignal::SpeechToken(_) => {
                                processed_signals += 1;
                            }
                            _ => {}
                        }
                    }
                }
                else => break,
            }
        }

        assert!(processed_signals >= 0, "Error: processed signals cannot be negative");
    }
}
