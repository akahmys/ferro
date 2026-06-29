use rayon::prelude::*;

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
