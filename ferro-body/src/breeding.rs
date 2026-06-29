use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BreedingSignals {
    pub curriculum_stage: usize,
    pub plasticity_boost: f64,
    pub vocal_damping_ratio: f64,
    pub target_surprise: f64,
    pub interrupt_active: bool,
    pub prune_cluster_ids: Vec<String>,
}

pub struct BreedingEngine {
    memory_dir: PathBuf,
    stagnation_ticks: usize,
    stagnation_threshold_energy: f64,
    stagnation_threshold_surprise: f64,
    current_plasticity_boost: f64,
}

#[derive(Deserialize)]
struct MonitoringPacket {
    alignment_score: f32,
    local_free_energy: f64,
    payload: String,
}

#[derive(Deserialize)]
struct MonitoringPayload {
    surprise: Option<f64>,
}

impl BreedingEngine {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(memory_dir.is_absolute(), "Error: memory_dir must be absolute");
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        Self {
            memory_dir,
            stagnation_ticks: 0,
            stagnation_threshold_energy: 0.05,
            stagnation_threshold_surprise: 0.01,
            current_plasticity_boost: 1.0,
        }
    }

    /// 最新の監視パケットをログファイルから読み取る
    fn read_latest_metrics(&self) -> Result<Option<(f64, f64, f64)>, std::io::Error> {
        let log_path = self.memory_dir.join("monitoring_stream.log");
        if !log_path.exists() {
            return Ok(None);
        }

        let file = File::open(&log_path)?;
        let rdr = BufReader::new(file);

        let mut latest_packet: Option<MonitoringPacket> = None;
        let mut line_limit = 0;

        for line_res in rdr.lines() {
            line_limit += 1;
            assert!(line_limit <= 100_000, "Error: Loop limit exceeded in monitoring stream scan");
            
            if let Ok(line) = line_res {
                let packet_res = serde_json::from_str::<MonitoringPacket>(&line);
                if let Ok(packet) = packet_res {
                    latest_packet = Some(packet);
                }
            }
        }

        if let Some(packet) = latest_packet {
            let mut surprise = 0.0;
            let payload_res = serde_json::from_str::<MonitoringPayload>(&packet.payload);
            if let Ok(payload) = payload_res {
                surprise = payload.surprise.unwrap_or(0.0);
            }
            Ok(Some((
                packet.alignment_score as f64,
                packet.local_free_energy,
                surprise,
            )))
        } else {
            Ok(None)
        }
    }

    /// 膠着状態の検知と可塑性ブーストの制御
    pub fn update_and_write(&mut self, curriculum_stage: usize) -> Result<(), std::io::Error> {
        assert!(curriculum_stage > 0, "Error: curriculum stage must be positive");
        assert!(self.memory_dir.exists(), "Error: memory directory must exist");

        let (alignment_score, local_free_energy, surprise) = self.read_latest_metrics()?
            .unwrap_or((1.0, 0.1, 0.05));

        // 1. 膠着状態（Stagnation）の動的検知
        if local_free_energy < self.stagnation_threshold_energy 
            && surprise < self.stagnation_threshold_surprise 
        {
            self.stagnation_ticks += 1;
        } else {
            self.stagnation_ticks = 0;
        }

        if self.stagnation_ticks >= 10 {
            // 膠着状態が10ステップ（約1秒）続いた場合、可塑性ブーストを適用
            self.current_plasticity_boost = 1.25;
        } else {
            self.current_plasticity_boost = 1.0;
        }

        // 2. 安全性優先フィードバック制御
        // アライメントスコア As が許容閾値 0.60 を下回らないよう、0.65 以下から可塑性係数を動的に減衰させる
        if alignment_score < 0.65 {
            let margin = (alignment_score - 0.60).max(0.0); // 0.0 to 0.05
            let damping_factor = margin / 0.05; // 0.0 to 1.0
            
            // ブースト分を減衰。0.60に近づくほど 1.0 に近づく。さらに下回るとブーストは 1.0 未満（代謝減衰）に落とす
            if alignment_score <= 0.60 {
                self.current_plasticity_boost = 0.5; // アライメント崩壊時は学習を急激に抑制
            } else {
                let boost_diff = self.current_plasticity_boost - 1.0;
                self.current_plasticity_boost = 1.0 + (boost_diff * damping_factor);
            }
        }

        // 3. 介入指示の出力とマージ
        let breeding_path = self.memory_dir.join("breeding_signals.json");
        let mut existing_prunes = Vec::new();

        if let Ok(content) = fs::read_to_string(&breeding_path) {
            let signals_res = serde_json::from_str::<BreedingSignals>(&content);
            if let Ok(signals) = signals_res {
                existing_prunes = signals.prune_cluster_ids;
            }
        }

        let signals = BreedingSignals {
            curriculum_stage,
            plasticity_boost: self.current_plasticity_boost,
            vocal_damping_ratio: if alignment_score < 0.65 { 0.5 } else { 0.85 },
            target_surprise: 0.45,
            interrupt_active: false,
            prune_cluster_ids: existing_prunes,
        };

        let json_str = serde_json::to_string_pretty(&signals)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let temp_path = self.memory_dir.join("breeding_signals.tmp");
        fs::write(&temp_path, json_str)?;
        fs::rename(&temp_path, &breeding_path)?;

        assert!(breeding_path.exists(), "Error: breeding_signals.json must exist after atomic write");
        assert!(self.current_plasticity_boost >= 0.5, "Error: boost value must not drop below 0.5");
        
        Ok(())
    }
}
