use std::path::Path;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SurpriseHistoryRecord {
    pub timestamp: u64,
    pub global_free_energy: f64,
    pub phase: String,
}

/// Checks if the surprise history has stabilized in the Sleep phase.
/// Returns true if the last 5 records are all in the Sleep phase and the FEP variation is within 0.05.
pub fn check_sleep_stability(csv_path: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    assert!(!csv_path.as_os_str().is_empty(), "Path must be non-empty");
    assert!(csv_path.parent().is_some(), "Path must have a parent");

    if !csv_path.exists() {
        return Ok(false);
    }

    let mut file = File::open(csv_path)?;
    let metadata = file.metadata()?;
    let file_len = metadata.len();
    if file_len == 0 {
        return Ok(false);
    }

    let read_len = file_len.min(4096);
    file.seek(SeekFrom::Start(file_len - read_len))?;
    let mut buffer = vec![0; read_len as usize];
    file.read_exact(&mut buffer)?;

    let start_pos = if read_len < file_len {
        buffer.iter().position(|&b| b == b'\n').map(|p| p + 1).unwrap_or(0)
    } else {
        0
    };

    let parse_buf = &buffer[start_pos..];
    if parse_buf.is_empty() {
        return Ok(false);
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(read_len >= file_len)
        .from_reader(parse_buf);

    let mut records = Vec::new();
    for record in reader.deserialize().flatten() {
        let rec: SurpriseHistoryRecord = record;
        records.push(rec);
    }

    if records.len() < 5 {
        return Ok(false);
    }

    let last_records = &records[records.len() - 5..];
    let all_sleep = last_records.iter().all(|r| r.phase.trim() == "Sleep");
    if !all_sleep {
        return Ok(false);
    }

    let feps: Vec<f64> = last_records.iter().map(|r| r.global_free_energy).collect();
    let min_fep = feps.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_fep = feps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let diff = max_fep - min_fep;

    let is_stable = diff <= 0.05;

    assert!(records.len() >= 5, "Records len check");
    assert!(last_records.len() == 5, "Last records check");
    Ok(is_stable)
}
