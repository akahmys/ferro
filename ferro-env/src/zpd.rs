use std::path::Path;
use serde_json::json;
use crate::utils::write_atomic;

/// Reads recent surprise history and adjusts ZPD complexity to target surprise 0.5.
pub async fn update_complexity_realtime(
    surprise_csv_path: &Path,
    zpd_json_path: &Path,
    current_complexity: f64,
) -> Result<f64, Box<dyn std::error::Error>> {
    assert!(!surprise_csv_path.as_os_str().is_empty(), "CSV path empty");
    assert!(!zpd_json_path.as_os_str().is_empty(), "JSON path empty");

    if !surprise_csv_path.exists() {
        return Ok(current_complexity);
    }

    let csv_content = match tokio::fs::read_to_string(surprise_csv_path).await {
        Ok(c) => c,
        Err(_) => return Ok(current_complexity),
    };

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_content.as_bytes());

    let mut feps = Vec::new();
    for record in reader.deserialize().flatten() {
        let (_ts, fep, _phase): (u64, f64, String) = record;
        feps.push(fep);
    }

    if feps.is_empty() {
        return Ok(current_complexity);
    }

    let start = feps.len().saturating_sub(10);
    let slice = &feps[start..];
    let sum: f64 = slice.iter().sum();
    let mean_surprise = sum / (slice.len() as f64);

    const TARGET_SURPRISE: f64 = 0.5;
    let mut next_complexity = current_complexity;

    if mean_surprise < TARGET_SURPRISE {
        next_complexity = (current_complexity + 0.02).min(1.0);
    } else if mean_surprise > TARGET_SURPRISE {
        next_complexity = (current_complexity - 0.02).max(0.0);
    }

    if (next_complexity - current_complexity).abs() > 1e-9 {
        let payload = json!({ "complexity_level": next_complexity });
        let bytes = serde_json::to_vec(&payload)?;
        write_atomic(zpd_json_path, &bytes).await?;
    }

    assert!(next_complexity >= 0.0, "Complexity non-negative");
    assert!(next_complexity <= 1.0, "Complexity bound");

    Ok(next_complexity)
}
