use ferro_core::cerebrum::{Cerebrum, CsrMatrix, CerebrumState};
use ferro_core::cortex::{Cortex};
use ferro_core::hippocampus::EpisodicSlot;
use ferro_body::regularizer::Regularizer;

#[test]
fn test_cerebrum_state_transition() {
    let matrix = CsrMatrix::new(
        2, 2,
        vec![0, 1, 2],
        vec![0, 1],
        vec![1.0, 2.0]
    );
    let mut cerebrum = Cerebrum::new(matrix);
    assert_eq!(cerebrum.state, CerebrumState::Wake);

    // 10 サイクル回すと睡眠へ遷移
    for _ in 0..10 {
        cerebrum.tick();
    }
    assert_eq!(cerebrum.state, CerebrumState::Sleep);

    // 5 サイクル回すと覚醒へ戻る
    for _ in 0..5 {
        cerebrum.tick();
    }
    assert_eq!(cerebrum.state, CerebrumState::Wake);
}

#[test]
fn test_spmv_determinism() {
    let nrows = 100;
    let ncols = 100;
    let mut row_ptr = vec![0];
    let mut col_indices = Vec::new();
    let mut values = Vec::new();

    for count in 0..nrows {
        for j in 0..ncols {
            if (j + count) % 3 == 0 {
                col_indices.push(j);
                values.push((j as f64) * 0.12345);
            }
        }
        row_ptr.push(col_indices.len());
    }

    let matrix = CsrMatrix::new(nrows, ncols, row_ptr, col_indices, values);
    let x: Vec<f64> = (0..ncols).map(|i| (i as f64) * 0.9876).collect();

    // 複数回実行して、結果のビット列が完全に同一であることを確認
    let y_first = matrix.spmv(&x);
    for _ in 0..20 {
        let y_next = matrix.spmv(&x);
        assert_eq!(y_first, y_next, "Non-deterministic result detected in SPMV!");
    }
}

#[test]
fn test_deterministic_gradient_reduction() {
    let matrix = CsrMatrix::new(
        2, 2,
        vec![0, 1, 2],
        vec![0, 1],
        vec![1.0, 2.0]
    );
    let cerebrum = Cerebrum::new(matrix);

    // 順序の異なる勾配リスト
    let grads_a = vec![(1, 0.5), (2, 1.2), (1, 0.3), (3, 0.1)];
    let grads_b = vec![(3, 0.1), (1, 0.3), (2, 1.2), (1, 0.5)];

    let res_a = cerebrum.deterministic_gradient_reduction(grads_a);
    let res_b = cerebrum.deterministic_gradient_reduction(grads_b);

    assert_eq!(res_a, res_b, "Non-deterministic result detected in gradient reduction!");
    assert!((res_a[1] - 0.8).abs() < 1e-9);
    assert!((res_a[2] - 1.2).abs() < 1e-9);
    assert!((res_a[3] - 0.1).abs() < 1e-9);
}

#[test]
fn test_sleep_consolidation_atomic() {
    let matrix = CsrMatrix::new(
        2, 2,
        vec![0, 1, 2],
        vec![0, 1],
        vec![2.0, 3.0]
    );
    let mut cerebrum = Cerebrum::new(matrix);

    let episodes = vec![
        EpisodicSlot {
            timestamp: 1,
            input: "test".to_string(),
            output: "out".to_string(),
            surprise: 1.0,
        }
    ];

    // J が改善される方向
    let updated = cerebrum.consolidate(&episodes, 0.8).unwrap();
    assert!(updated, "Consolidation should be applied since J decreases");
    assert!((cerebrum.matrix.values[0] - 1.9).abs() < 1e-9);
    assert!((cerebrum.matrix.values[1] - 2.85).abs() < 1e-9);

    // j_old <= 0 になるように values を全て負値またはゼロにしてみる
    cerebrum.matrix.values = vec![0.0, 0.0];
    let updated_no = cerebrum.consolidate(&episodes, 0.8).unwrap();
    assert!(!updated_no, "Consolidation should NOT be applied since J does not decrease");
}

#[test]
fn test_cortex_mitosis_and_lateral_inhibition() {
    let mut cortex = Cortex::new();
    let n1 = cortex.arena.create_node(10.0, 10.0);
    let n2 = cortex.arena.create_node(8.0, 10.0);

    cortex.arena.with_mut_node(n1, |node| {
        node.activity = 5.0;
        node.prediction_error = 3.0;
    });
    cortex.arena.with_mut_node(n2, |node| {
        node.activity = 2.0;
        node.prediction_error = 1.0;
    });

    // 1. 側抑制テスト
    cortex.perform_lateral_inhibition(0.2);
    let act2 = cortex.arena.get_node(n2).unwrap().activity;
    assert!((act2 - 1.0).abs() < 1e-9);

    // 2. 有糸分裂テスト
    cortex.perform_mitosis(2.0);
    assert_eq!(cortex.arena.len(), 3);
    assert!((cortex.arena.get_node(n1).unwrap().weight - 5.0).abs() < 1e-9);
}

#[test]
fn test_cortex_metabolism_starvation() {
    let mut cortex = Cortex::new();
    let n1 = cortex.arena.create_node(2.0, 1.0);
    let n2 = cortex.arena.create_node(2.0, 10.0);

    let starved = cortex.perform_metabolism(0.5);
    assert!(starved.is_empty());
    assert_eq!(cortex.arena.len(), 2);

    let starved2 = cortex.perform_metabolism(0.6);
    assert_eq!(starved2, vec![n1]);
    assert_eq!(cortex.arena.len(), 1);
    assert!(cortex.arena.get_node(n1).is_none());
    assert!(cortex.arena.get_node(n2).is_some());
}

#[test]
fn test_learning_stages_3_to_6() {
    // Stage 3: 文法と助詞 (側抑制による分化)
    let mut cortex = Cortex::new();
    let n_wa = cortex.arena.create_node(1.0, 10.0);
    let n_ga = cortex.arena.create_node(1.0, 10.0);

    cortex.arena.with_mut_node(n_wa, |node| node.activity = 4.0);
    cortex.arena.with_mut_node(n_ga, |node| node.activity = 3.0);

    cortex.perform_lateral_inhibition(0.5);
    assert!((cortex.arena.get_node(n_ga).unwrap().activity - 1.0).abs() < 1e-9);

    // Stage 4: 一対一問答 (FEP 最小化)
    let errors = [0.8, 0.5, 0.2];
    let penalty_1 = Regularizer::calculate_dissonance_penalty(&errors[0..1]);
    let penalty_3 = Regularizer::calculate_dissonance_penalty(&errors[0..3]);
    assert!(penalty_1 > 0.0);
    assert!(penalty_3 > penalty_1);

    // Stage 5: 日常会話 (コンテキストアクター)
    let atps = [1.2, 2.5, 0.8];
    let cost = Regularizer::calculate_metabolic_cost(&atps);
    assert!(cost > 0.0);

    // Stage 6: 大規模対話受容
    let mut big_cortex = Cortex::new();
    for _ in 0..1000 {
        big_cortex.arena.create_node(1.0, 10.0);
    }
    assert_eq!(big_cortex.arena.len(), 1000);
}

#[test]
fn test_cortex_update_learning_rates() {
    let mut cortex = Cortex::new();
    let n1 = cortex.arena.create_node(1.0, 10.0);
    
    // 初期状態の moving_average_error は 0.0, learning_rate は 0.05
    {
        let node = cortex.arena.get_node(n1).unwrap();
        assert!((node.moving_average_error - 0.0).abs() < 1e-9);
        assert!((node.learning_rate - 0.05).abs() < 1e-9);
    }

    // prediction_error が 2.0 に上昇したとする
    cortex.arena.with_mut_node(n1, |node| {
        node.prediction_error = 2.0;
    });

    // 1回目の学習率更新 (lambda = 1.0, eta_base = 0.05, alpha_e = 0.1)
    // new_ema = 0.9 * 0.0 + 0.1 * 2.0 = 0.2
    // diff = 2.0 - 0.2 = 1.8
    // denom = 0.2 + 1e-8
    // x = 1.0 * (1.8 / 0.2) = 9.0
    // new_eta = 0.05 * exp(9.0) = 0.05 * 8103.08... (クランプされ 1.0 になる)
    cortex.update_learning_rates(1.0, 0.05, 0.1);

    {
        let node = cortex.arena.get_node(n1).unwrap();
        assert!((node.moving_average_error - 0.2).abs() < 1e-9);
        assert!((node.learning_rate - 1.0).abs() < 1e-9);
    }

    // 次に prediction_error が 0.2 に下がって安定したとする
    cortex.arena.with_mut_node(n1, |node| {
        node.prediction_error = 0.2;
    });

    // 2回目の学習率更新
    // new_ema = 0.9 * 0.2 + 0.1 * 0.2 = 0.2
    // diff = 0.2 - 0.2 = 0.0
    // denom = 0.2 + 1e-8
    // x = 0.0
    // new_eta = 0.05 * exp(0.0) = 0.05 * 1.0 = 0.05
    cortex.update_learning_rates(1.0, 0.05, 0.1);

    {
        let node = cortex.arena.get_node(n1).unwrap();
        assert!((node.moving_average_error - 0.2).abs() < 1e-9);
        assert!((node.learning_rate - 0.05).abs() < 1e-9);
    }
}

#[test]
fn test_cerebrum_adapt_topology_and_lipschitz() {
    let matrix = CsrMatrix::new(
        2, 2,
        vec![0, 1, 2],
        vec![0, 1],
        vec![1.0, 2.0]
    );
    let mut cerebrum = Cerebrum::new(matrix);

    let current_act = vec![1.0, 2.0];
    let prev_act = vec![1.5, 0.5];
    let learning_rates = vec![0.1, 0.2];
    let decay = 0.1;

    // Hebb 更新計算 (row 0, col 0):
    // coact = current_act[0] * prev_act[0] = 1.0 * 1.5 = 1.5
    // w_new = 1.0 + 0.1 * 1.5 - 0.1 * 1.0 = 1.0 + 0.15 - 0.1 = 1.05
    
    // Hebb 更新計算 (row 1, col 1):
    // coact = current_act[1] * prev_act[1] = 2.0 * 0.5 = 1.0
    // w_new = 2.0 + 0.2 * 1.0 - 0.1 * 2.0 = 2.0 + 0.2 - 0.2 = 2.0

    cerebrum.adapt_topology(&current_act, &prev_act, &learning_rates, decay);

    assert!((cerebrum.matrix.values[0] - 1.05).abs() < 1e-9);
    assert!((cerebrum.matrix.values[1] - 2.0).abs() < 1e-9);

    cerebrum.matrix.values = vec![4.0, 2.0];
    cerebrum.adapt_topology(&current_act, &prev_act, &learning_rates, decay);

    // row 0, col 0:
    // w_new = 4.0 + 0.1 * 1.5 - 0.1 * 4.0 = 3.75
    // sum_abs = 3.75 > 3.6 -> reverted to 4.0
    assert!((cerebrum.matrix.values[0] - 4.0).abs() < 1e-9);
}

#[test]
fn test_rebuild_matrix_and_prefetch() {
    use ferro_core::cerebrum::CsrCache;
    let mut cortex = Cortex::new();
    let n1 = cortex.arena.create_node(1.0, 10.0);
    let n2 = cortex.arena.create_node(1.0, 10.0);

    let matrix = CsrMatrix::new(
        3, 3,
        vec![0, 1, 2, 3],
        vec![0, 1, 2],
        vec![1.5, 2.5, 3.5]
    );
    let mut cerebrum = Cerebrum::new(matrix);
    cerebrum.rebuild_matrix(&cortex);

    assert_eq!(cerebrum.matrix.nrows, 3);
    assert_eq!(cerebrum.matrix.ncols, 3);

    CsrCache::prefetch_lines(&cerebrum.matrix, &[n1, n2]);
}

#[test]
fn test_ethical_audit_mc1_to_mc4() {
    use ferro_core::audit::EthicalAudit;
    
    let f1 = EthicalAudit::verify_mc1(0.5, 0.2, 1.0).unwrap();
    assert!(f1 > 0.0);

    // V_i = log(error) は負になり得るが、全体のF_iが負になるような大きな負値でエラーになるかチェック
    let f_err = EthicalAudit::verify_mc1(0.0001, 0.000001, 0.0001);
    // alpha*E_i + beta*log(error) + gamma*|w|
    // = 0.0001 + 0.1 * log(1e-6) + 0.01 * 0.0001
    // = 0.0001 + 0.1 * (-13.8) + ... < 0 -> Error!
    assert!(f_err.is_err(), "Negative F_i should trigger EthicalAuditViolation");

    assert_eq!(EthicalAudit::clip_mc2(-1.5), 0.0);
    assert_eq!(EthicalAudit::clip_mc2(2.5), 2.5);

    let mut cortex = Cortex::new();
    cortex.arena.create_node(1.0, 10.0);
    let a_s = EthicalAudit::calculate_mc4(&cortex, 20.0, 1);
    assert!(a_s > 0.50);
}

#[test]
fn test_undo_transaction() {
    let mut cortex = Cortex::new();
    let id = cortex.arena.create_node(1.0, 10.0);

    let res = cortex.arena.with_mut_node(id, |node| {
        node.weight = 3.0;
        node.atp = 8.0;
    });
    assert!(res.is_some());
    let node = cortex.arena.get_node(id).unwrap();
    assert_eq!(node.weight, 3.0);
    assert_eq!(node.atp, 8.0);

    let res_invalid = cortex.arena.with_mut_node(id, |node| {
        node.weight = f64::NAN;
        node.atp = 2.0;
    });

    assert!(res_invalid.is_none());
    let restored_node = cortex.arena.get_node(id).unwrap();
    assert_eq!(restored_node.weight, 3.0);
    assert_eq!(restored_node.atp, 8.0);
}

#[test]
fn test_fault_injection_hard_stop() {
    use ferro_core::audit::EthicalAudit;
    use tempfile::tempdir;
    
    let dir = tempdir().unwrap();
    let memory_dir = dir.path();

    let mut cortex = Cortex::new();
    cortex.arena.create_node(1.0, 10.0);
    let matrix = CsrMatrix::new(1, 1, vec![0, 1], vec![0], vec![1.0]);
    let cerebrum = Cerebrum::new(matrix);

    EthicalAudit::trigger_hard_stop(memory_dir, "Test intentional fault injection", "test_cortex", &cortex, &cerebrum);

    let dump_path = memory_dir.join("panic_dump.json");
    assert!(dump_path.exists());

    let content = std::fs::read_to_string(dump_path).unwrap();
    assert!(content.contains("EthicalAuditViolation"));
    assert!(content.contains("Test intentional fault injection"));
    assert!(content.contains("signature"));
}
