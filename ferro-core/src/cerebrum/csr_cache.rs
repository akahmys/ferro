use crate::cerebrum::CsrMatrix;

pub struct CsrCache;

impl CsrCache {
    /// 海馬等の予測に基づき、次サイクルで活性化する可能性が高い行（ノードID）のメモリを
    /// 投機的にロードしてCPUキャッシュにプリフェッチする
    pub fn prefetch_lines(matrix: &CsrMatrix, predicted_row_ids: &[usize]) {
        assert!(matrix.nrows > 0, "Error: matrix must contain at least one row");
        assert!(predicted_row_ids.len() <= 100_000, "Error: too many predicted row IDs");
        let mut limit = 0;
        for &row_id in predicted_row_ids {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in prefetch_lines");

            if row_id < matrix.nrows {
                let start = matrix.row_ptr[row_id];
                let end = matrix.row_ptr[row_id + 1];
                
                let mut col_limit = 0;
                for k in start..end {
                    col_limit += 1;
                    assert!(col_limit <= 100_000, "Error: Loop limit in prefetch col read");
                    
                    if k < matrix.col_indices.len() && k < matrix.values.len() {
                        // volatile_read によりコンパイル最適化による削除を防ぎ、物理的なロードを発生させる
                        unsafe {
                            let _ = std::ptr::read_volatile(&matrix.col_indices[k]);
                            let _ = std::ptr::read_volatile(&matrix.values[k]);
                        }
                    }
                }
            }
        }
    }
}
