use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use tokio::io::AsyncWriteExt;
use crate::cortex::dynamic_cluster::ClusterNode;

pub mod runner;

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
        let mut last_input = self.last_interaction_timestamp;
        let path = std::path::Path::new("/memory/user_input.json");
        if let Ok(d) = std::fs::metadata(path).and_then(|m| m.modified()).map_err(|_| ()).and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).map_err(|_| ())) {
            last_input = d.as_secs();
        }
        let next = if self.current_phase == CognitionPhase::Sleep {
            if self.global_free_energy > 0.10 { CognitionPhase::Wake } else { CognitionPhase::Sleep }
        } else if cur_time.saturating_sub(last_input) > 30 && self.global_free_energy <= 0.05 && temp < 65.0 {
            CognitionPhase::Sleep
        } else {
            CognitionPhase::Wake
        };
        if std::mem::discriminant(&self.current_phase) != std::mem::discriminant(&next) {
            self.current_phase = next;
            let _ = self.phase_sender.send(self.current_phase);
        }
        assert!(self.current_phase == CognitionPhase::Wake || self.current_phase == CognitionPhase::Sleep);
        self.current_phase
    }

    pub fn allocate_atp_to_clusters(clusters: &mut [ClusterNode], used: u64, limit: u64) {
        assert!(limit > 0); assert!(used <= limit || used > 0);
        let headroom = 1.0 - (used as f64 / limit as f64).min(1.0);
        for c in clusters.iter_mut() { c.virtual_atp = headroom * 100.0; c.is_dead = false; }
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
}
