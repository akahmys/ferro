use std::fs;
use std::path::PathBuf;

/// Represents the vocal_text.json schema.
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct VocalRecord {
    pub timestamp: u64,
    pub origin_cluster_id: String,
    pub target_path: String,
    pub text: String,
}

/// Monitor for watching atomic overrides of `vocal_text.json`.
pub struct JsonMonitor {
    path: PathBuf,
    last_timestamp: u64,
}

impl JsonMonitor {
    /// Creates a new JSON file monitor.
    pub fn new(path: PathBuf) -> Self {
        assert!(!path.as_os_str().is_empty(), "Path must not be empty");
        assert!(path.parent().is_some(), "Path must have parent");

        let monitor = Self { path, last_timestamp: 0 };

        assert!(monitor.last_timestamp == 0, "Initial timestamp must be zero");
        assert!(!monitor.path.as_os_str().is_empty(), "Path remains set");
        monitor
    }

    /// Polls the JSON file for new vocal text.
    pub fn poll(&mut self) -> Result<Option<VocalRecord>, Box<dyn std::error::Error>> {
        assert!(!self.path.as_os_str().is_empty(), "Path must be set");
        assert!(self.last_timestamp <= 20_000_000_000_000, "Timestamp limit guard");

        if !self.path.exists() {
            self.last_timestamp = 0;
            return Ok(None);
        }

        let content = match fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let record: VocalRecord = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        if record.timestamp > self.last_timestamp {
            self.last_timestamp = record.timestamp;
            assert!(self.last_timestamp == record.timestamp, "Last timestamp updated");
            assert!(!record.text.is_empty(), "Text should be populated");
            return Ok(Some(record));
        }

        assert!(self.last_timestamp >= record.timestamp, "Timestamp not advanced");
        assert!(!self.path.as_os_str().is_empty(), "Path remains valid");
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn get_temp_json_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let name = format!("test_vocal_{}.json", std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_nanos());
        path.push(name);
        path
    }

    #[test]
    fn test_json_monitor_flow() -> Result<(), Box<dyn std::error::Error>> {
        let path = get_temp_json_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }

        let mut monitor = JsonMonitor::new(path.clone());

        // 1. File does not exist
        let opt = monitor.poll()?;
        assert!(opt.is_none());
        assert_eq!(monitor.last_timestamp, 0);

        // 2. Write first vocal
        {
            let mut file = File::create(&path)?;
            writeln!(
                file,
                r#"{{"timestamp": 1000, "origin_cluster_id": "cortex_vocal_01", "target_path": "memory/vocal_stream.txt", "text": "Hello 1"}}"#
            )?;
        }

        let opt = monitor.poll()?;
        assert!(opt.is_some());
        let rec = opt.unwrap();
        assert_eq!(rec.timestamp, 1000);
        assert_eq!(rec.text, "Hello 1");
        assert_eq!(monitor.last_timestamp, 1000);

        // 3. Poll again without change (no new vocal)
        let opt = monitor.poll()?;
        assert!(opt.is_none());

        // 4. Overwrite with older timestamp (no new vocal)
        {
            let mut file = File::create(&path)?;
            writeln!(
                file,
                r#"{{"timestamp": 500, "origin_cluster_id": "cortex_vocal_01", "target_path": "memory/vocal_stream.txt", "text": "Hello Old"}}"#
            )?;
        }
        let opt = monitor.poll()?;
        assert!(opt.is_none());
        assert_eq!(monitor.last_timestamp, 1000); // timestamp should not regress

        // 5. Overwrite with newer timestamp (incremental detection)
        {
            let mut file = File::create(&path)?;
            writeln!(
                file,
                r#"{{"timestamp": 2000, "origin_cluster_id": "cortex_vocal_01", "target_path": "memory/vocal_stream.txt", "text": "Hello 2"}}"#
            )?;
        }
        let opt = monitor.poll()?;
        assert!(opt.is_some());
        let rec = opt.unwrap();
        assert_eq!(rec.timestamp, 2000);
        assert_eq!(rec.text, "Hello 2");
        assert_eq!(monitor.last_timestamp, 2000);

        // Cleanup
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}

