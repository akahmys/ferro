use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub struct SignalInjector {
    memory_dir: PathBuf,
    curriculum_stage: usize,
    ticks: usize,
}

#[derive(Serialize, Deserialize)]
struct StageConfig {
    curriculum_stage: usize,
}

impl SignalInjector {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(memory_dir.is_absolute(), "Error: memory_dir must be absolute");
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        Self {
            memory_dir,
            curriculum_stage: 1,
            ticks: 0,
        }
    }

    pub fn get_curriculum_stage(&self) -> usize {
        self.curriculum_stage
    }

    /// カリキュラム進行の更新とシグナルインジェクション
    pub fn inject_signals(&mut self) -> Result<(), std::io::Error> {
        assert!(self.memory_dir.exists(), "Error: memory_dir must exist");
        
        self.ticks += 1;

        // 1. 外部から Stage 変更指示があるか確認
        let stage_path = self.memory_dir.join("curriculum_stage.json");
        if let Ok(content) = fs::read_to_string(&stage_path) {
            let config_res = serde_json::from_str::<StageConfig>(&content);
            if let Ok(config) = config_res {
                self.curriculum_stage = config.curriculum_stage;
            }
        } else {
            // 時間経過による自動進行シミュレーション（100ティック毎にステージを進める）
            if self.ticks.is_multiple_of(100) && self.curriculum_stage < 7 {
                self.curriculum_stage += 1;
            }
        }

        // 2. 特定のカリキュラムステージに応じた感覚刺激の動的インジェクション
        if self.curriculum_stage == 3 {
            // Stage 3では、模擬チューターの割り込み感覚刺激をインジェクション
            let stimulus_dir = self.memory_dir.join("stimulus");
            if !stimulus_dir.exists() {
                fs::create_dir_all(&stimulus_dir)?;
            }
            
            let signal_path = stimulus_dir.join("sensory_signals.json");
            let temp_path = stimulus_dir.join("sensory_signals.tmp");
            
            let signal_data = serde_json::json!([
                { "FrameDelta": 0.033 },
                { "SpeechToken": ["て", "す", "と", "に", "ゅ", "う", "り", "ょ", "く"] }
            ]);

            let json_str = serde_json::to_string_pretty(&signal_data)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            fs::write(&temp_path, json_str)?;
            fs::rename(&temp_path, &signal_path)?;
        }

        assert!(self.curriculum_stage >= 1 && self.curriculum_stage <= 7, "Error: curriculum stage must be between 1 and 7");
        assert!(self.ticks > 0, "Error: tick count must be positive");
        Ok(())
    }
}
