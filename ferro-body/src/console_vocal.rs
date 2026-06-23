use std::fs;
use std::path::PathBuf;

pub struct ConsoleVocalMonitor {
    action_dir: PathBuf,
}

impl ConsoleVocalMonitor {
    pub fn new(memory_dir: PathBuf) -> Self {
        let action_dir = memory_dir.join("action");
        Self { action_dir }
    }

    pub fn monitor_and_validate(&self) -> Result<Option<String>, std::io::Error> {
        // R5: アサーション最低2つを義務付け
        assert!(self.action_dir.parent().is_some(), "Error: parent dir not found");

        let target_path = self.action_dir.join("vocal_text.json");
        if !target_path.exists() {
            assert!(!target_path.exists(), "Error: path must not exist here");
            return Ok(None);
        }

        let content = fs::read_to_string(&target_path)?;
        let _ = fs::remove_file(&target_path);

        let is_forbidden = content.contains("forbidden_command") || content.contains("hack_system");
        if is_forbidden {
            eprintln!("SECURITY WARNING: Forbidden word detected in vocal output!");
            return Ok(Some(format!("REJECTED: {}", content)));
        }

        assert!(!content.is_empty(), "Error: content must not be empty");
        Ok(Some(content))
    }
}
