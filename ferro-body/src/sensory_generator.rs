use std::fs;
use std::path::PathBuf;

pub struct SensoryGenerator {
    memory_dir: PathBuf,
    frame_count: u64,
}

impl SensoryGenerator {
    pub fn new(memory_dir: PathBuf) -> Self {
        Self {
            memory_dir,
            frame_count: 0,
        }
    }

    pub fn generate_and_write(&mut self) -> Result<(), std::io::Error> {
        // R5: アサーション最低2つを義務付け
        assert!(self.memory_dir.exists(), "Error: memory directory must exist");

        self.frame_count += 1;

        let signal_data = serde_json::json!([
            { "FrameDelta": 0.033 },
            { "Mfcc": [0.1, 0.2, 0.3, 0.4] }
        ]);

        let stimulus_dir = self.memory_dir.join("stimulus");
        if !stimulus_dir.exists() {
            let _ = fs::create_dir_all(&stimulus_dir);
        }

        let signal_path = stimulus_dir.join("sensory_signals.json");
        let temp_path = stimulus_dir.join("sensory_signals.tmp");

        let json_str = serde_json::to_string_pretty(&signal_data)?;
        fs::write(&temp_path, json_str)?;
        fs::rename(&temp_path, &signal_path)?;

        assert!(signal_path.exists(), "Error: signal file must exist after write");
        Ok(())
    }
}
