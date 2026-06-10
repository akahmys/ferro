use crate::organs::{EfferenceCopy, SensorySignal, BrainstemCommand, SensoryMuteCommand};
use std::collections::VecDeque;
use tokio::sync::{mpsc, broadcast};

pub struct Midbrain {
    pub efference_rx: mpsc::Receiver<EfferenceCopy>,
    pub echo_rx: mpsc::Receiver<SensorySignal>,
    pub mute_tx: broadcast::Sender<SensoryMuteCommand>,
    pub surprise_tx: mpsc::Sender<f32>,
    pub pending_efference: VecDeque<EfferenceCopy>,
    pub match_window_ms: u64,
    pub max_pending: usize,
}

impl Midbrain {
    pub fn new(
        efference_rx: mpsc::Receiver<EfferenceCopy>,
        echo_rx: mpsc::Receiver<SensorySignal>,
        mute_tx: broadcast::Sender<SensoryMuteCommand>,
        surprise_tx: mpsc::Sender<f32>,
        match_window_ms: u64,
        max_pending: usize,
    ) -> Self {
        assert!(match_window_ms > 0); assert!(max_pending > 0);
        Self {
            efference_rx, echo_rx, mute_tx, surprise_tx,
            pending_efference: VecDeque::with_capacity(max_pending),
            match_window_ms, max_pending,
        }
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
        if self.pending_efference.len() >= self.max_pending {
            let _ = self.pending_efference.pop_front();
        }
        self.pending_efference.push_back(eff);
        let _ = self.mute_tx.send(SensoryMuteCommand { mute: true, attenuation_db: -40.0 });
    }

    pub(crate) async fn handle_sensory_echo(&mut self, signal: SensorySignal) {
        if let SensorySignal::ProprioceptiveEcho(tokens) = signal {
            assert!(!tokens.is_empty()); assert!(self.match_window_ms > 0);
            let mut matched = false;
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            let mut match_idx = None;
            for (idx, eff) in self.pending_efference.iter().enumerate() {
                let time_diff = now.saturating_sub(eff.timestamp);
                if time_diff <= self.match_window_ms / 1000 && eff.expected_tokens == tokens {
                    matched = true;
                    match_idx = Some(idx);
                    break;
                }
            }
            let surprise = if matched {
                if let Some(i) = match_idx { let _ = self.pending_efference.remove(i); }
                0.0
            } else {
                1.0
            };
            let _ = self.surprise_tx.send(surprise).await;
            let _ = self.mute_tx.send(SensoryMuteCommand { mute: false, attenuation_db: 0.0 });
        }
    }
}
