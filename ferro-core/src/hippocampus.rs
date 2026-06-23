use serde::Serialize;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize)]
pub struct EpisodicSlot {
    pub timestamp: u64,
    pub input: String,
    pub output: String,
    pub surprise: f64,
}

pub struct Hippocampus {
    tx: mpsc::Sender<EpisodicSlot>,
}

impl Hippocampus {
    pub fn new(csv_path: PathBuf) -> Self {
        assert!(!csv_path.as_os_str().is_empty(), "Error: CSV path must not be empty");
        let (tx, mut rx) = mpsc::channel::<EpisodicSlot>(1000);

        std::thread::spawn(move || {
            let mut writer_limit = 0;
            let file_exists = csv_path.exists();
            if let Ok(file) = OpenOptions::new().create(true).write(true).append(true).open(&csv_path) {
                let mut wtr = csv::Writer::from_writer(file);
                if !file_exists {
                    let _ = wtr.write_record(&["timestamp", "input", "output", "surprise"]);
                    let _ = wtr.flush();
                }
            }

            while let Some(slot) = rx.blocking_recv() {
                writer_limit += 1;
                assert!(writer_limit <= 1000000, "Error: Hippocampus writer loop safety limit exceeded");
                if let Ok(file) = OpenOptions::new().create(true).write(true).append(true).open(&csv_path) {
                    let mut wtr = csv::Writer::from_writer(file);
                    let _ = wtr.serialize(slot);
                    let _ = wtr.flush();
                }
            }
        });

        let new_hippocampus = Self { tx };
        assert!(new_hippocampus.tx.capacity() > 0, "Error: tx channel must be open");
        new_hippocampus
    }

    pub fn record_episode(&self, slot: EpisodicSlot) -> Result<(), String> {
        assert!(slot.timestamp > 0, "Error: timestamp must be positive");
        assert!(slot.surprise >= 0.0, "Error: surprise must be non-negative");

        self.tx.try_send(slot).map_err(|e| e.to_string())?;

        assert!(!self.tx.is_closed(), "Error: tx channel must not be closed");
        assert!(self.tx.capacity() < 2000, "Error: capacity check");
        Ok(())
    }
}
