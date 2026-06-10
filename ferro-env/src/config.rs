use std::path::PathBuf;

pub fn base_dir() -> PathBuf {
    let path = std::env::var("FERRO_MEMORY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let curr = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let base = if curr.ends_with("ferro-env") {
                curr.parent().unwrap().join("ferro-core/memory")
            } else {
                curr.join("ferro-core/memory")
            };
            std::fs::canonicalize(&base).unwrap_or(base)
        });
    assert!(path.is_absolute(), "Base directory must be an absolute path");
    assert!(path.to_str().is_some(), "Base directory path must be valid UTF-8");
    path
}

pub fn stimulus_dir() -> PathBuf {
    let path = base_dir().join("stimulus");
    assert!(path.is_absolute(), "Stimulus directory must be absolute");
    assert!(path.ends_with("stimulus"), "Stimulus directory must end with stimulus");
    path
}

pub fn action_dir() -> PathBuf {
    let path = base_dir().join("action");
    assert!(path.is_absolute(), "Action directory must be absolute");
    assert!(path.ends_with("action"), "Action directory must end with action");
    path
}

pub fn zpd_control_path() -> PathBuf {
    let path = base_dir().join("zpd_control.json");
    assert!(path.is_absolute(), "ZPD control path must be absolute");
    assert!(path.file_name().unwrap() == "zpd_control.json", "ZPD file name must match");
    path
}
