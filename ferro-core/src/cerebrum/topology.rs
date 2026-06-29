use rayon::prelude::*;
use crate::cerebrum::Cerebrum;

impl Cerebrum {
    /// 時間的共活性に基づく接続トポロジー適応および Lipschitz 境界維持
    pub fn adapt_topology(
        &mut self,
        current_activity: &[f64],
        prev_activity: &[f64],
        learning_rates: &[f64],
        decay: f64,
    ) {
        assert!(current_activity.len() == self.matrix.nrows, "Error: current_activity size mismatch");
        assert!(prev_activity.len() == self.matrix.ncols, "Error: prev_activity size mismatch");
        assert!(learning_rates.len() == self.matrix.nrows, "Error: learning_rates size mismatch");
        assert!((0.0..=1.0).contains(&decay), "Error: decay must be between 0 and 1");

        let num_rows = self.matrix.nrows;
        let mut updated_values = vec![0.0; self.matrix.values.len()];

        let row_ptr = &self.matrix.row_ptr;
        let col_indices = &self.matrix.col_indices;
        let values = &self.matrix.values;

        // Map フェーズ (並列実行): 各行の新しい重みの計算と Lipschitz スケーリング
        let results: Vec<Vec<f64>> = (0..num_rows)
            .into_par_iter()
            .map(|i| {
                let start = row_ptr[i];
                let end = row_ptr[i + 1];
                let eta_i = learning_rates[i];
                let act_i = current_activity[i];

                let mut row_vals = Vec::with_capacity(end - start);
                let mut sum_abs = 0.0;
                let mut limit = 0;

                for k in start..end {
                    limit += 1;
                    assert!(limit <= 100_000, "Error: Loop limit in adapt_topology row");
                    let j = col_indices[k];
                    let w_old = values[k];
                    
                    let coact = act_i * prev_activity[j];
                    let w_new = w_old + eta_i * coact - decay * w_old;
                    
                    row_vals.push(w_new);
                    sum_abs += w_new.abs();
                }

                if sum_abs > 3.6 {
                    row_vals.clear();
                    let mut restore_limit = 0;
                    for &val in &values[start..end] {
                        restore_limit += 1;
                        assert!(restore_limit <= 100_000, "Error: Loop limit in adapt_topology restore");
                        row_vals.push(val);
                    }
                    eprintln!("WARNING: MC-3 Lipschitz violation (sum|w| = {} > 3.6) at row {}. Reverting weight update.", sum_abs, i);
                }

                row_vals
            })
            .collect();

        // Reduce フェーズ (順序固定直列): 結果を values へ昇順に書き戻す
        let mut k = 0;
        let mut store_limit = 0;
        for row_vals in results {
            store_limit += 1;
            assert!(store_limit <= num_rows, "Error: Loop limit in topology values store");
            for val in row_vals {
                updated_values[k] = val;
                k += 1;
            }
        }

        self.matrix.values = updated_values;

        assert!(self.matrix.values.iter().all(|val| val.is_finite()), "Error: all mutated weight values must be finite");
    }
}
