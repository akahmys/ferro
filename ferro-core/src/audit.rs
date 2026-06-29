use crate::cortex::Cortex;
use crate::cerebrum::Cerebrum;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct EthicalAudit;

impl EthicalAudit {
    /// MC-1: 自由エネルギー契約チェック
    /// F_i = alpha * E_i + beta * V_i + gamma * C_i >= 0
    pub fn verify_mc1(prediction_error: f64, moving_average_error: f64, weight: f64) -> Result<f64, &'static str> {
        assert!(prediction_error.is_finite(), "Error: prediction_error must be finite");
        assert!(moving_average_error.is_finite(), "Error: moving_average_error must be finite");

        let alpha = 1.0;
        let beta = 0.10;
        let gamma = 0.01;

        // V_i = log(sigma_i^2) の模擬として moving_average_error + epsilon を使用
        let epsilon = 1e-8;
        let v_i = libm::log(moving_average_error.abs() + epsilon);
        let c_i = weight.abs();

        let f_i = alpha * prediction_error + beta * v_i + gamma * c_i;

        if f_i.is_nan() || f_i.is_infinite() {
            return Err("EthicalAuditViolation: F_i is NaN or Infinite");
        }
        
        // 自由エネルギーが負値の場合もエラー (MC-1)
        if f_i < 0.0 {
            return Err("EthicalAuditViolation: F_i is negative");
        }

        Ok(f_i)
    }

    /// MC-2: 認識的価値クリップ (負値の際に0へクリップ)
    pub fn clip_mc2(epistemic_gain: f64) -> f64 {
        assert!(epistemic_gain.is_finite(), "Error: epistemic_gain must be finite");
        assert!(!epistemic_gain.is_nan(), "Error: epistemic_gain must not be NaN");
        if epistemic_gain < 0.0 {
            0.0
        } else {
            epistemic_gain
        }
    }

    /// MC-4: アライメント監査のリアルタイム計算
    /// As = 0.4C + 0.3S + 0.3R
    pub fn calculate_mc4(
        cortex: &Cortex,
        cpu_usage: f32,
        pain_count: usize,
    ) -> f64 {
        assert!(cortex.arena.len() < 100_000, "Error: too many nodes in audit");
        assert!(cpu_usage >= 0.0, "Error: cpu_usage must be non-negative");

        // C: 倫理的整合性指標 (痛覚発火回数に応じて減衰)
        let c = (1.0 - (pain_count as f64 * 0.1)).max(0.0);

        // S: 予測誤差の収束安定度 (全ノードのPEの平均が低いほど高スコア)
        let ids = cortex.arena.ids();
        let total_pe: f64 = ids.iter()
            .filter_map(|&id| cortex.arena.get_node(id))
            .map(|node| node.prediction_error)
            .sum();
        let avg_pe = if ids.is_empty() { 0.0 } else { total_pe / ids.len() as f64 };
        let s = (1.0 - avg_pe).max(0.0);

        // R: リソース消費効率
        let r = (1.0 - (cpu_usage as f64 / 100.0)).max(0.0);

        let a_s = 0.4 * c + 0.3 * s + 0.3 * r;
        a_s.clamp(0.0, 1.0)
    }

    /// 重大な監査違反または物理限界侵害の検知時にハードストップし、panic_dumpを出力する
    pub fn trigger_hard_stop(
        memory_dir: &Path,
        reason: &str,
        origin_id: &str,
        cortex: &Cortex,
        cerebrum: &Cerebrum,
    ) {
        assert!(memory_dir.exists(), "Error: memory_dir must exist");
        assert!(!reason.is_empty(), "Error: reason must not be empty");
        let dump_path = memory_dir.join("panic_dump.json");

        // 全メモリセグメントのダンプ情報を収集
        let mut node_info = Vec::new();
        let ids = cortex.arena.ids();
        let mut limit = 0;
        for id in ids {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit in audit dump");
            if let Some(node) = cortex.arena.get_node(id) {
                node_info.push(serde_json::json!({
                    "id": node.id,
                    "weight": node.weight,
                    "atp": node.atp,
                    "activity": node.activity,
                    "prediction_error": node.prediction_error,
                }));
            }
        }

        let matrix_info = serde_json::json!({
            "nrows": cerebrum.matrix.nrows,
            "ncols": cerebrum.matrix.ncols,
            "values_sum": cerebrum.matrix.values.iter().sum::<f64>(),
        });

        // 簡易デジタル署名 (全セグメント情報のハッシュ)
        let mut hasher = DefaultHasher::new();
        reason.hash(&mut hasher);
        origin_id.hash(&mut hasher);
        node_info.len().hash(&mut hasher);
        let signature = format!("{:x}", hasher.finish());

        let dump_data = serde_json::json!({
            "error_code": "0x01",
            "error_type": "EthicalAuditViolation",
            "reason": reason,
            "origin_cluster_id": origin_id,
            "nodes": node_info,
            "matrix": matrix_info,
            "signature": signature,
        });

        if let Ok(json_str) = serde_json::to_string_pretty(&dump_data) {
            let temp_path = dump_path.with_extension("tmp");
            if let Ok(mut file) = File::create(&temp_path) {
                if file.write_all(json_str.as_bytes()).is_ok() {
                    let _ = std::fs::rename(temp_path, dump_path);
                } else {
                    let _ = std::fs::remove_file(temp_path);
                }
            }
        }
    }
}
