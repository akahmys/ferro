use std::collections::HashMap;
use rayon::prelude::*;
use crate::hippocampus::EpisodicSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CerebrumState {
    Wake,
    Sleep,
}

pub struct CsrMatrix {
    pub nrows: usize,
    pub ncols: usize,
    pub row_ptr: Vec<usize>,
    pub col_indices: Vec<usize>,
    pub values: Vec<f64>,
}

impl CsrMatrix {
    pub fn new(nrows: usize, ncols: usize, row_ptr: Vec<usize>, col_indices: Vec<usize>, values: Vec<f64>) -> Self {
        assert!(row_ptr.len() == nrows + 1, "Error: row_ptr size must be nrows + 1");
        assert!(col_indices.len() == values.len(), "Error: col_indices and values must have the same size");
        Self {
            nrows,
            ncols,
            row_ptr,
            col_indices,
            values,
        }
    }

    /// 並列かつ決定論的な SPMV 演算
    pub fn spmv(&self, x: &[f64]) -> Vec<f64> {
        assert!(x.len() == self.ncols, "Error: x size must equal ncols");
        assert!(self.row_ptr.len() == self.nrows + 1, "Error: row_ptr size check failed");

        let y: Vec<f64> = (0..self.nrows)
            .into_par_iter()
            .map(|i| {
                let start = self.row_ptr[i];
                let end = self.row_ptr[i + 1];
                let mut sum = 0.0;
                let mut limit = 0;
                for k in start..end {
                    limit += 1;
                    assert!(limit <= 100_000, "Error: Loop limit exceeded in spmv row iteration");
                    let col = self.col_indices[k];
                    sum += self.values[k] * x[col];
                }
                sum
            })
            .collect();

        assert!(y.len() == self.nrows, "Error: output vector size must equal nrows");
        assert!(y.iter().all(|val| val.is_finite()), "Error: all output elements must be finite");
        y
    }
}

pub struct Cerebrum {
    pub state: CerebrumState,
    pub cycle_count: usize,
    pub wake_cycle_limit: usize,
    pub sleep_cycle_limit: usize,
    pub matrix: CsrMatrix,
}

impl Cerebrum {
    pub fn new(matrix: CsrMatrix) -> Self {
        assert!(matrix.nrows > 0, "Error: matrix must have at least one row");
        let cerebrum = Self {
            state: CerebrumState::Wake,
            cycle_count: 0,
            wake_cycle_limit: 10,
            sleep_cycle_limit: 5,
            matrix,
        };
        assert!(cerebrum.state == CerebrumState::Wake, "Error: starting state must be Wake");
        assert!(cerebrum.cycle_count == 0, "Error: initial cycle count must be 0");
        cerebrum
    }

    /// サイクルごとの更新と自動状態遷移
    pub fn tick(&mut self) {
        self.cycle_count += 1;
        match self.state {
            CerebrumState::Wake => {
                if self.cycle_count >= self.wake_cycle_limit {
                    self.state = CerebrumState::Sleep;
                    self.cycle_count = 0;
                }
            }
            CerebrumState::Sleep => {
                if self.cycle_count >= self.sleep_cycle_limit {
                    self.state = CerebrumState::Wake;
                    self.cycle_count = 0;
                }
            }
        }
        assert!(self.cycle_count < self.wake_cycle_limit || self.cycle_count < self.sleep_cycle_limit, "Error: cycle count overflow");
        assert!(self.state == CerebrumState::Wake || self.state == CerebrumState::Sleep, "Error: invalid state");
    }

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
}
