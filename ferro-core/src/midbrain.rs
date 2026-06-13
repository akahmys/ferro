use crate::organs::{EfferenceCopy, SensorySignal, BrainstemCommand, SensoryMuteCommand};
use std::collections::VecDeque;
use tokio::sync::{mpsc, broadcast};

pub struct Midbrain {
    pub efference_rx: mpsc::Receiver<EfferenceCopy>,
    pub echo_rx: mpsc::Receiver<SensorySignal>,
    pub mute_tx: broadcast::Sender<SensoryMuteCommand>,
    pub surprise_tx: mpsc::Sender<(f32, String)>,
    pub pending_efference: VecDeque<EfferenceCopy>,
    pub match_window_ms: u64,
    pub max_pending: usize,
}

impl Midbrain {
    pub fn new(efference_rx: mpsc::Receiver<EfferenceCopy>, echo_rx: mpsc::Receiver<SensorySignal>, mute_tx: broadcast::Sender<SensoryMuteCommand>, surprise_tx: mpsc::Sender<(f32, String)>, match_window_ms: u64, max_pending: usize) -> Self {
        assert!(match_window_ms > 0); assert!(max_pending > 0);
        Self { efference_rx, echo_rx, mute_tx, surprise_tx, pending_efference: VecDeque::with_capacity(max_pending), match_window_ms, max_pending }
    }

    pub async fn run_loop(mut self, mut kill_rx: broadcast::Receiver<BrainstemCommand>) {
        let mut loop_count: u64 = 0;
        loop {
            assert!(loop_count < 1_000_000_000); assert!(self.max_pending > 0);
            loop_count += 1;
            let tick = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                tokio::select! {
                    Some(eff) = self.efference_rx.recv() => { self.handle_efference_copy(eff).await; false }
                    Some(sig) = self.echo_rx.recv() => { self.handle_sensory_echo(sig).await; false }
                    Ok(cmd) = kill_rx.recv() => { matches!(cmd, BrainstemCommand::ForceSleep) }
                }
            }).await;
            if let Ok(true) = tick { break; }
        }
    }

    pub(crate) async fn handle_efference_copy(&mut self, eff: EfferenceCopy) {
        assert!(!eff.origin_cluster_id.is_empty()); assert!(self.max_pending > 0);
        if self.pending_efference.len() >= self.max_pending { let _ = self.pending_efference.pop_front(); }
        self.pending_efference.push_back(eff);
        let _ = self.mute_tx.send(SensoryMuteCommand { mute: true, attenuation_db: -40.0 });
    }

    pub(crate) async fn handle_sensory_echo(&mut self, signal: SensorySignal) {
        match signal {
            SensorySignal::ProprioceptiveEcho(tokens) => {
                assert!(!tokens.is_empty()); assert!(self.match_window_ms > 0);
                let mut matched = false;
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let mut match_idx = None;
                let window_secs = (self.match_window_ms + 999) / 1000;
                for (idx, eff) in self.pending_efference.iter().enumerate() {
                    let time_diff = now.saturating_sub(eff.timestamp);
                    let tokens_match = !eff.expected_tokens.is_empty() && !tokens.is_empty() &&
                        eff.expected_tokens.iter().any(|et| tokens.iter().any(|t| t.contains(et) || et.contains(t)));
                    if time_diff <= window_secs && tokens_match {
                        matched = true; match_idx = Some(idx); break;
                    }
                }
                let surprise = if matched {
                    if let Some(i) = match_idx { let _ = self.pending_efference.remove(i); }
                    0.0
                } else {
                    1.0
                };
                let _ = self.surprise_tx.send((surprise, "cortex_midbrain_gate".to_string())).await;
                let _ = self.mute_tx.send(SensoryMuteCommand { mute: false, attenuation_db: 0.0 });
            }
            SensorySignal::FrameDelta(delta) => {
                assert!(self.match_window_ms > 0);
                let surprise = (delta as f32).min(1.0);
                if surprise > 0.01 { let _ = self.surprise_tx.send((surprise, "cortex_midbrain_gate".to_string())).await; }
            }
            SensorySignal::SpeechToken(tokens) => {
                assert!(self.match_window_ms > 0);
                for token in tokens {
                    let cluster_id = if token.len() >= 2 { token } else { "cortex_midbrain_gate".to_string() };
                    let _ = self.surprise_tx.send((0.3, cluster_id)).await;
                }
            }
            _ => {}
        }
    }
}
