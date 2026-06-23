pub struct Regularizer;

impl Regularizer {
    /// 代謝コストを計算する
    pub fn calculate_metabolic_cost(node_atp_usage: &[f64]) -> f64 {
        // 事前アサーション
        assert!(!node_atp_usage.is_empty(), "Error: node_atp_usage must not be empty");
        assert!(node_atp_usage.len() < 100_000, "Error: too many nodes in regularizer calculation");

        let mut sum = 0.0;
        let mut limit = 0;
        for &atp in node_atp_usage {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in calculate_metabolic_cost");
            assert!(atp >= 0.0, "Error: ATP usage must be non-negative");
            sum += atp;
        }

        let cost = sum * 0.05;

        // 事後アサーション
        assert!(cost >= 0.0, "Error: cost must be non-negative");
        assert!(cost.is_finite(), "Error: cost must be finite");
        cost
    }

    /// 不協和ペナルティを計算する
    pub fn calculate_dissonance_penalty(prediction_errors: &[f64]) -> f64 {
        // 事前アサーション
        assert!(!prediction_errors.is_empty(), "Error: prediction_errors must not be empty");
        assert!(prediction_errors.len() < 100_000, "Error: too many errors in regularizer calculation");

        let mut sum_sq = 0.0;
        let mut limit = 0;
        for &err in prediction_errors {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in calculate_dissonance_penalty");
            assert!(err.is_finite(), "Error: error value must be finite");
            sum_sq += err * err;
        }

        let penalty = sum_sq * 0.10;

        // 事後アサーション
        assert!(penalty >= 0.0, "Error: penalty must be non-negative");
        assert!(penalty.is_finite(), "Error: penalty must be finite");
        penalty
    }
}
