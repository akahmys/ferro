use serde::{Deserialize, Serialize};
use crate::organs::BrainstemCommand;
use tokio::sync::{mpsc, broadcast};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodicSlot {
    pub timestamp: u64,
    pub event_id: String,
    pub origin_cluster_id: String,
    pub sensory_summary: String,
    pub motor_summary: String,
    pub surprise_level: f32,
}

pub struct Hippocampus {
    pub buffer: Vec<Option<EpisodicSlot>>,
    pub head: usize,
    pub count: usize,
    pub capacity: usize,
    pub storage_path: String,
    pub surprise_rx: mpsc::Receiver<f32>,
}

impl Hippocampus {
    pub fn new(capacity: usize, storage_path: String, surprise_rx: mpsc::Receiver<f32>) -> Self {
        assert!(capacity > 0); assert!(!storage_path.is_empty());
        Self { buffer: vec![None; capacity], head: 0, count: 0, capacity, storage_path, surprise_rx }
    }

    pub async fn run_loop(mut self, mut kill_rx: broadcast::Receiver<BrainstemCommand>) {
        let mut loop_count: u64 = 0;
        loop {
            assert!(loop_count < 1_000_000_000); assert!(self.capacity > 0);
            loop_count += 1;
            let tick = tokio::time::timeout(tokio::time::Duration::from_millis(500), async {
                tokio::select! {
                    Some(val) = self.surprise_rx.recv() => { self.register_surprise_episode(val).await; false }
                    Ok(cmd) = kill_rx.recv() => { matches!(cmd, BrainstemCommand::ForceSleep) }
                }
            }).await;
            if let Ok(true) = tick { break; }
        }
    }

    pub fn push_slot(&mut self, slot: EpisodicSlot) {
        assert!(self.head < self.capacity); assert!(!slot.event_id.is_empty());
        self.buffer[self.head] = Some(slot);
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity { self.count += 1; }
    }

    async fn register_surprise_episode(&mut self, surprise: f32) {
        assert!(surprise >= 0.0); assert!(self.capacity > 0);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let slot = EpisodicSlot {
            timestamp: now, event_id: format!("evt_{}", now),
            origin_cluster_id: "cortex_midbrain_gate".to_string(),
            sensory_summary: "audio_feedback_processed".to_string(),
            motor_summary: "vocal_output_logged".to_string(), surprise_level: surprise,
        };
        self.push_slot(slot);
        if surprise > 0.5 || self.head == 0 { let _ = self.persist_buffer().await; }
    }

    pub async fn persist_buffer(&self) -> Result<(), std::io::Error> {
        assert!(self.count > 0); assert!(!self.storage_path.is_empty());
        let temp_path = format!("{}.tmp", self.storage_path);
        let mut csv_data = String::from("timestamp,event_id,origin_cluster_id,sensory_summary,motor_summary,surprise_level\n");
        for s in self.buffer.iter().flatten() {
            csv_data.push_str(&format!(
                "{},{},{},{},{},{:.2}\n",
                s.timestamp, s.event_id, s.origin_cluster_id, s.sensory_summary, s.motor_summary, s.surprise_level
            ));
        }
        let mut file = tokio::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path).await?;
        file.write_all(csv_data.as_bytes()).await?;
        file.sync_all().await?;
        tokio::fs::rename(&temp_path, &self.storage_path).await?;
        Ok(())
    }
}
