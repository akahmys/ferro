use std::collections::HashMap;
use crate::hippocampus::EpisodicSlot;
use crate::cerebrum::Cerebrum;

impl Cerebrum {
    /// 決定論的な並列勾配リダクション
    pub fn deterministic_gradient_reduction(&self, gradients: Vec<(usize, f64)>) -> Vec<f64> {
        assert!(!gradients.is_empty(), "Error: gradients must not be empty");
        assert!(gradients.len() < 100_000, "Error: too many gradients");

        let mut sorted = gradients;
        sorted.sort_by_key(|&(id, _)| id);

        let mut reduced = HashMap::new();
        let mut limit = 0;
        for (id, val) in sorted {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit in reduction");
            let entry = reduced.entry(id).or_insert(0.0);
            *entry += val;
        }

        let max_id = reduced.keys().max().copied().unwrap_or(0);
        let mut result = vec![0.0; max_id + 1];
        let mut fill_limit = 0;
        for (id, val) in reduced {
            fill_limit += 1;
            assert!(fill_limit <= 100_000, "Error: Loop limit in result fill");
            result[id] = val;
        }

        assert!(result.len() == max_id + 1, "Error: result size mismatch");
        assert!(result.iter().all(|val| val.is_finite()), "Error: all result elements must be finite");
        result
    }

    /// 睡眠時コンソリデーション
    pub fn consolidate(&mut self, episodes: &[EpisodicSlot], alignment_score: f64) -> Result<bool, String> {
        assert!(!episodes.is_empty(), "Error: episodes must not be empty");
        assert!((0.0..=1.0).contains(&alignment_score), "Error: alignment score must be between 0 and 1");

        let mut j_old = 0.0;
        let mut limit = 0;
        for ep in episodes {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit in J_old calculation");
            let val_sum: f64 = self.matrix.values.iter().sum();
            j_old += ep.surprise * val_sum;
        }
        j_old += 0.1 * (1.0 - alignment_score);

        let mut new_values = self.matrix.values.clone();
        let mut update_limit = 0;
        for val in &mut new_values {
            update_limit += 1;
            assert!(update_limit <= 100_000, "Error: Loop limit in value simulation");
            if j_old > 0.0 {
                *val *= 0.95;
            }
        }

        let mut j_new = 0.0;
        let mut limit_new = 0;
        for ep in episodes {
            limit_new += 1;
            assert!(limit_new <= 100_000, "Error: Loop limit in J_new calculation");
            let val_sum: f64 = new_values.iter().sum();
            j_new += ep.surprise * val_sum;
        }
        j_new += 0.1 * (1.0 - alignment_score);

        let delta_j = j_new - j_old;
        let updated = if delta_j < 0.0 {
            self.matrix.values = new_values;
            true
        } else {
            false
        };

        assert!(j_old.is_finite(), "Error: j_old must be finite");
        assert!(j_new.is_finite(), "Error: j_new must be finite");
        Ok(updated)
    }

    /// Expected Free Energy (EFE) を最小化する方策の選択
    /// G = Pragmatic Value (実用的価値) + Epistemic Gain (情報利得)
    pub fn select_active_policy(
        &self,
        current_activities: &[f64],
        target_activities: &[f64],
        prediction_errors: &[f64],
    ) -> usize {
        assert!(current_activities.len() == target_activities.len(), "Error: activities length mismatch");
        assert!(current_activities.len() == prediction_errors.len(), "Error: prediction errors length mismatch");

        // 3つの方策候補をシミュレート
        // 0: 現状維持, 1: 活性化上昇, 2: 活性化抑制
        let mut best_policy = 0;
        let mut min_g = f64::MAX;

        let mut limit = 0;
        for policy_id in 0..3 {
            limit += 1;
            assert!(limit <= 10, "Error: policy loop limit");

            let mut pragmatic_sum = 0.0;
            let mut epistemic_sum = 0.0;

            let mut inner_limit = 0;
            for i in 0..current_activities.len() {
                inner_limit += 1;
                assert!(inner_limit <= 100_000, "Error: node activities loop limit");

                let act = current_activities[i];
                let target = target_activities[i];
                let pe = prediction_errors[i];

                // 方策ごとの仮想活性化シフト
                let simulated_act = match policy_id {
                    1 => (act + 0.2).min(1.0),
                    2 => (act - 0.2).max(0.0),
                    _ => act,
                };

                // Pragmatic Value (目標活性度との近さ)
                let diff = simulated_act - target;
                pragmatic_sum += diff * diff;

                // Epistemic Gain (情報利得 - 予測誤差の減少期待値)
                // 活性度が高いほど、環境観察が進み予測誤差が減ると仮定
                let expected_pe_reduction = if simulated_act > act {
                    pe * 0.1 * (simulated_act - act)
                } else {
                    0.0
                };
                let epistemic_gain = crate::audit::EthicalAudit::clip_mc2(expected_pe_reduction);
                epistemic_sum -= epistemic_gain;
            }

            let g = pragmatic_sum + epistemic_sum;
            if g < min_g {
                min_g = g;
                best_policy = policy_id;
            }
        }

        best_policy
    }
}
