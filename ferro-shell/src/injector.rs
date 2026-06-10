use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Injects mock data for surprise spikes and vocalizations.
pub fn inject_mock(memory_dir: &str, surprise: f64) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!memory_dir.is_empty(), "Memory dir path must not be empty");
    assert!(surprise >= 0.0, "Surprise value must be non-negative");

    let mem_path = Path::new(memory_dir);
    assert!(mem_path.is_dir(), "Memory directory must exist");

    let csv_path = mem_path.join("episodic_buffer.csv");
    let file_exists = csv_path.exists();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&csv_path)?;

    if !file_exists {
        writeln!(file, "timestamp,episode_id,target_cluster_id,raw_surprise,context_hash,payload")?;
    }

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)?
        .as_millis();

    writeln!(
        file,
        "{},ep_{}_001,cortex_visual_02,{},hash_abc123,Mock surprise event payload",
        now_ms, now_ms, surprise
    )?;

    let action_dir = mem_path.join("action");
    if !action_dir.exists() {
        fs::create_dir_all(&action_dir)?;
    }
    let json_path = action_dir.join("vocal_text.json");
    let tmp_path = action_dir.join("vocal_text.json.tmp");

    let vocal_payload = serde_json::json!({
        "timestamp": now_ms as u64,
        "origin_cluster_id": "cortex_vocal_01",
        "target_path": "memory/vocal_stream.txt",
        "text": format!("Injecting mock surprise event with level {:.2}.", surprise)
    });

    let json_string = serde_json::to_string(&vocal_payload)?;
    fs::write(&tmp_path, json_string)?;
    fs::rename(&tmp_path, &json_path)?;

    assert!(json_path.exists(), "vocal_text.json must exist after atomic write");
    assert!(csv_path.exists(), "episodic_buffer.csv must exist after append");
    Ok(())
}
