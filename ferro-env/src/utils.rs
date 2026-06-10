use std::path::Path;
use tokio::fs;
use crate::config::zpd_control_path;

pub async fn write_atomic(target: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    assert!(target.is_absolute(), "Target path must be absolute");
    assert!(!content.is_empty(), "Content must not be empty");

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).await?;
    }
    
    let temp_ext = format!("tmp_{}", rand::random::<u32>());
    let temp_path = target.with_extension(temp_ext);
    
    fs::write(&temp_path, content).await?;
    
    let rename_result = fs::rename(&temp_path, target).await;
    if rename_result.is_err() {
        let _ = fs::remove_file(&temp_path).await;
    }
    
    let success = rename_result.is_ok();
    assert!(success, "Atomic rename operation must succeed");
    
    rename_result
}

pub async fn read_zpd_complexity() -> f64 {
    let path = zpd_control_path();
    assert!(path.is_absolute(), "ZPD path must be absolute");
    assert!(path.to_str().is_some(), "ZPD path must be valid UTF-8");

    if !path.exists() {
        return 0.5; // Default safe level
    }

    match fs::read_to_string(&path).await {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(val) => {
                    if let Some(level) = val.get("complexity_level").and_then(|v| v.as_f64()) {
                        assert!(level >= 0.0, "Complexity level must be non-negative");
                        assert!(level <= 1.0, "Complexity level must not exceed 1.0");
                        level
                    } else {
                        0.5
                    }
                }
                Err(_) => 0.5,
            }
        }
        Err(_) => 0.5,
    }
}
