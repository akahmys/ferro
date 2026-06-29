use std::fs;
use std::path::PathBuf;

pub struct SensoryGenerator {
    memory_dir: PathBuf,
    frame_count: u64,
}

impl SensoryGenerator {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(memory_dir.is_absolute(), "Error: memory_dir must be an absolute path");
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        Self {
            memory_dir,
            frame_count: 0,
        }
    }

    pub fn generate_and_write(&mut self) -> Result<(), std::io::Error> {
        // R5: アサーション最低2つを義務付け
        assert!(self.memory_dir.exists(), "Error: memory directory must exist");
        assert!(self.memory_dir.is_absolute(), "Error: memory directory must be absolute");

        self.frame_count += 1;

        let t = self.frame_count as f64 * 0.1;
        // libm ではなく標準の sin / cos を使用 (f64標準メソッド)
        let m1 = (t.sin() * 0.5 + 0.5) as f32;
        let m2 = ((t * 1.5).cos() * 0.5 + 0.5) as f32;
        let m3 = ((t * 2.0).sin() * 0.3 + 0.5) as f32;
        let m4 = (t.cos() * 0.4 + 0.5) as f32;

        let signal_data = serde_json::json!([
            { "FrameDelta": 0.033 },
            { "Mfcc": [m1, m2, m3, m4] }
        ]);

        let stimulus_dir = self.memory_dir.join("stimulus");
        if !stimulus_dir.exists() {
            fs::create_dir_all(&stimulus_dir)?;
        }

        let signal_path = stimulus_dir.join("sensory_signals.json");
        let temp_path = stimulus_dir.join("sensory_signals.tmp");

        let json_str = serde_json::to_string_pretty(&signal_data)?;
        fs::write(&temp_path, json_str)?;
        fs::rename(&temp_path, &signal_path)?;

        assert!(signal_path.exists(), "Error: signal file must exist after write");
        assert!(self.frame_count > 0, "Error: frame count must be positive");
        Ok(())
    }
}
