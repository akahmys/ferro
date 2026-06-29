use std::fs;
use std::path::PathBuf;

pub struct ConsoleVocalMonitor {
    action_dir: PathBuf,
}

impl ConsoleVocalMonitor {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(memory_dir.is_absolute(), "Error: memory_dir must be an absolute path");
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        let action_dir = memory_dir.join("action");
        Self { action_dir }
    }

    pub fn monitor_and_validate(&self) -> Result<Option<String>, std::io::Error> {
        // R5: アサーション最低2つを義務付け
        assert!(self.action_dir.parent().is_some(), "Error: parent dir not found");
        assert!(self.action_dir.is_absolute(), "Error: action_dir path must be absolute");

        let target_path = self.action_dir.join("vocal_text.json");
        if !target_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&target_path)?;
        fs::remove_file(&target_path)?;

        let is_forbidden = content.contains("forbidden_command") || content.contains("hack_system");
        if is_forbidden {
            eprintln!("SECURITY WARNING: Forbidden word detected in vocal output!");
            assert!(!target_path.exists(), "Error: target file should have been removed even on rejection");
            return Ok(Some(format!("REJECTED: {}", content)));
        }

        assert!(!content.is_empty(), "Error: content must not be empty");
        assert!(!target_path.exists(), "Error: target file should have been removed after success");
        Ok(Some(content))
    }
}
