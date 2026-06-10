use std::fs::File;
use std::io::{Seek, SeekFrom, Read};
use std::path::PathBuf;
use crate::monitor::stats::SurpriseStats;

/// Represents a row in `episodic_buffer.csv`.
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct EpisodeRecord {
    pub timestamp: u64,
    pub episode_id: String,
    pub target_cluster_id: String,
    pub raw_surprise: f64,
    pub context_hash: String,
    pub payload: String,
}

/// Monitor for parsing new rows in `episodic_buffer.csv` incrementally.
pub struct CsvMonitor {
    path: PathBuf,
    offset: u64,
}

impl CsvMonitor {
    /// Creates a new CSV file monitor.
    pub fn new(path: PathBuf) -> Self {
        assert!(!path.as_os_str().is_empty(), "Path must not be empty");
        assert!(path.parent().is_some(), "Path must have parent");

        let monitor = Self { path, offset: 0 };

        assert!(monitor.offset == 0, "Initial offset must be zero");
        assert!(!monitor.path.as_os_str().is_empty(), "Path remains set");
        monitor
    }

    /// Polls new lines from CSV and updates stats.
    pub fn poll(&mut self, stats: &mut SurpriseStats) -> Result<Vec<EpisodeRecord>, Box<dyn std::error::Error>> {
        assert!(!self.path.as_os_str().is_empty(), "Path must be set");
        assert!(self.offset <= 10_000_000_000, "Offset is within reasonable limits");

        if !self.path.exists() {
            self.offset = 0;
            return Ok(Vec::new());
        }

        let metadata = std::fs::metadata(&self.path)?;
        let size = metadata.len();
        if size < self.offset {
            self.offset = 0;
        }
        if size == self.offset {
            return Ok(Vec::new());
        }

        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.offset))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let last_nl = buffer.iter().rposition(|&b| b == b'\n');
        let (parse_buf, read_len) = match last_nl {
            Some(pos) => (&buffer[..=pos], pos + 1),
            None => return Ok(Vec::new()),
        };

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(self.offset == 0)
            .from_reader(parse_buf);
        let mut records = Vec::new();
        for result in reader.deserialize() {
            let record: EpisodeRecord = result?;
            stats.add_surprise(record.raw_surprise);
            records.push(record);
        }

        self.offset += read_len as u64;
        assert!(self.offset <= size, "Offset must not exceed file size");
        assert!(records.len() <= 10000, "Records limit guard");
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use crate::monitor::stats::SurpriseStats;

    fn get_temp_csv_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let name = format!("test_episodic_{}.csv", std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos());
        path.push(name);
        path
    }

    #[test]
    fn test_csv_monitor_basic_and_truncation() -> Result<(), Box<dyn std::error::Error>> {
        let path = get_temp_csv_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }

        let mut stats = SurpriseStats::new(10);
        let mut monitor = CsvMonitor::new(path.clone());

        // 1. When file does not exist
        let records = monitor.poll(&mut stats)?;
        assert!(records.is_empty());
        assert_eq!(monitor.offset, 0);

        // 2. Write headers and 1 row
        {
            let mut file = File::create(&path)?;
            writeln!(file, "timestamp,episode_id,target_cluster_id,raw_surprise,context_hash,payload")?;
            writeln!(file, "12345,ep_1,cortex_visual_01,0.35,hash_x,payload_x")?;
        }

        let records = monitor.poll(&mut stats)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].episode_id, "ep_1");
        assert_eq!(records[0].raw_surprise, 0.35);
        assert!(monitor.offset > 0);
        let first_offset = monitor.offset;

        // 3. Write additional row (incremental update)
        {
            let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
            writeln!(file, "12346,ep_2,cortex_visual_01,0.85,hash_y,payload_y")?;
        }

        let records = monitor.poll(&mut stats)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].episode_id, "ep_2");
        assert_eq!(records[0].raw_surprise, 0.85);
        assert!(monitor.offset > first_offset);

        // 4. Truncation Recovery
        // Overwrite file to make it smaller (truncation)
        {
            let mut file = File::create(&path)?;
            writeln!(file, "timestamp,episode_id,target_cluster_id,raw_surprise,context_hash,payload")?;
            writeln!(file, "12347,ep_3,cortex_visual_01,0.95,hash_z,payload_z")?;
        }

        // poll should detect that file size < offset, reset offset to 0, and parse the new row
        let records = monitor.poll(&mut stats)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].episode_id, "ep_3");
        assert_eq!(records[0].raw_surprise, 0.95);

        // Cleanup
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}

