use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::{mpsc, broadcast, Mutex};
use tokio::io::AsyncWriteExt;
use crate::organs::BrainstemCommand;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CognitionPhase {
    Wake,
    Sleep,
}

pub struct Cerebrum {
    pub current_phase: CognitionPhase,
    pub global_free_energy: f64,
    pub last_interaction_timestamp: u64,
    pub phase_sender: broadcast::Sender<CognitionPhase>,
    pub surprise_history_path: PathBuf,
    pub history_limit: usize,
    pub history: Vec<(u64, f64, CognitionPhase)>,
}

impl Cerebrum {
    pub fn new<P: AsRef<Path>>(phase_tx: broadcast::Sender<CognitionPhase>, history_path: P, limit: usize) -> Self {
        let path = history_path.as_ref().to_path_buf();
        assert!(!path.as_os_str().is_empty()); assert!(limit > 0);
        Self {
            current_phase: CognitionPhase::Wake, global_free_energy: 0.0, last_interaction_timestamp: 0,
            phase_sender: phase_tx, surprise_history_path: path, history_limit: limit, history: Vec::new(),
        }
    }

    pub fn evaluate_phase_transition(&mut self, cur_time: u64, temp: f32) -> CognitionPhase {
        assert!(cur_time > 0); assert!(temp > -100.0);
        let next = if cur_time.saturating_sub(self.last_interaction_timestamp) > 900 && temp < 65.0 { CognitionPhase::Sleep } else { CognitionPhase::Wake };
        if std::mem::discriminant(&self.current_phase) != std::mem::discriminant(&next) {
            self.current_phase = next;
            let _ = self.phase_sender.send(self.current_phase);
        }
        assert!(self.current_phase == CognitionPhase::Wake || self.current_phase == CognitionPhase::Sleep);
        self.current_phase
    }

    pub async fn record_free_energy(&mut self, timestamp: u64, fep: f64) -> Result<(), std::io::Error> {
        assert!(fep >= 0.0); assert!(timestamp > 0);
        self.global_free_energy = fep;
        self.history.push((timestamp, fep, self.current_phase));
        if self.history.len() > self.history_limit { self.history.remove(0); }
        if let Some(p) = self.surprise_history_path.parent() { tokio::fs::create_dir_all(p).await?; }
        let temp = format!("{}.tmp", self.surprise_history_path.display());
        let mut csv = String::from("timestamp,global_free_energy,phase\n");
        for (t, f, p) in &self.history { csv.push_str(&format!("{},{:.4},{:?}\n", t, f, p)); }
        let mut file = tokio::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp).await?;
        file.write_all(csv.as_bytes()).await?; file.sync_all().await?;
        tokio::fs::rename(&temp, &self.surprise_history_path).await?;
        assert!(self.history.len() <= self.history_limit); Ok(())
    }

    pub async fn run_loop(
        cerebrum: Arc<Mutex<Self>>, cortex: Arc<crate::cortex::Cortex>,
        mut int_rx: mpsc::Receiver<()>, mut surprise_rx: mpsc::Receiver<f32>,
        mut kill_rx: broadcast::Receiver<BrainstemCommand>,
    ) {
        assert!(Arc::strong_count(&cortex) >= 1);
        assert!(Arc::strong_count(&cerebrum) >= 1);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let mut loop_count = 0;
        loop {
            assert!(loop_count < 1_000_000_000);
            assert!(Arc::strong_count(&cortex) >= 1);
            loop_count += 1;
            tokio::select! {
                _ = interval.tick() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs()).unwrap_or(0);
                    let mut cer = cerebrum.lock().await;
                    let prev = cer.current_phase;
                    let phase = cer.evaluate_phase_transition(now, 50.0);
                    if prev == CognitionPhase::Wake && phase == CognitionPhase::Sleep {
                        let path = crate::storage::manager::get_safe_path("/memory/episodic_buffer.csv");
                        let cortex_ref = cortex.clone();
                        tokio::spawn(async move { let _ = crate::cortex::trigger_sleep_replay(cortex_ref, &path).await; });
                    }
                }
                Some(_) = int_rx.recv() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    cerebrum.lock().await.last_interaction_timestamp = now;
                }
                Some(s) = surprise_rx.recv() => {
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                    let mut cer = cerebrum.lock().await;
                    cer.last_interaction_timestamp = now;
                    let _ = cer.record_free_energy(now, s as f64).await;
                }
                Ok(cmd) = kill_rx.recv() => { if matches!(cmd, BrainstemCommand::ForceSleep) { break; } }
            }
        }
    }
}
