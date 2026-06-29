pub mod csr;
pub mod ops;
pub mod topology;
pub mod csr_cache;

pub use csr::CsrMatrix;
pub use csr_cache::CsrCache;
use crate::cortex::Cortex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CerebrumState {
    Wake,
    Sleep,
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

    /// Cortexの現在のノード構成に合わせてCSR行列を決定論的に再構築する
    pub fn rebuild_matrix(&mut self, cortex: &Cortex) {
        assert!(cortex.arena.len() < 100_000, "Error: too many nodes in rebuild_matrix");
        let max_id = cortex.arena.ids().iter().max().copied().unwrap_or(0);
        assert!(max_id < 100_000, "Error: max_id exceeds limit");
        let n = max_id + 1;

        let mut row_ptr = vec![0; n + 1];
        let mut col_indices = Vec::new();
        let mut values = Vec::new();

        let active_ids: std::collections::HashSet<usize> = cortex.arena.ids().into_iter().collect();
        let mut current_offset = 0;
        let mut loop_limit = 0;

        for (i, ptr) in row_ptr.iter_mut().enumerate().take(n) {
            loop_limit += 1;
            assert!(loop_limit <= 100_000, "Error: Loop limit in rebuild_matrix row iteration");
            *ptr = current_offset;

            if active_ids.contains(&i) {
                let mut added_self = false;
                if i < self.matrix.nrows {
                    let start = self.matrix.row_ptr[i];
                    let end = self.matrix.row_ptr[i + 1];
                    let mut col_limit = 0;
                    for k in start..end {
                        col_limit += 1;
                        assert!(col_limit <= 100_000, "Error: Loop limit in rebuild_matrix col iteration");
                        let j = self.matrix.col_indices[k];
                        if active_ids.contains(&j) {
                            col_indices.push(j);
                            values.push(self.matrix.values[k]);
                            current_offset += 1;
                            if j == i {
                                added_self = true;
                            }
                        }
                    }
                }
                
                if !added_self {
                    col_indices.push(i);
                    values.push(1.0);
                    current_offset += 1;
                }
            }
        }
        row_ptr[n] = current_offset;

        self.matrix = CsrMatrix::new(n, n, row_ptr, col_indices, values);
    }
}
