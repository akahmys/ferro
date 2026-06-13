use crate::cerebrum::{Cerebrum, CognitionPhase};
use crate::storage::manager::StorageManager;
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::hippocampus::EpisodicSlot;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_cerebrum_phase_transition() {
    let (phase_tx, mut phase_rx) = broadcast::channel(10);
    let temp_history = std::env::temp_dir().join("surprise_history.csv");
    let path = temp_history.to_str().map(|s| s.to_string()).unwrap_or_default();

    let mut cerebrum = Cerebrum::new(phase_tx, path, 10);
    assert_eq!(cerebrum.current_phase, CognitionPhase::Wake);

    // インタラクションが無い状態で900秒以上経過した場合の遷移判定
    let next_phase = cerebrum.evaluate_phase_transition(1000, 50.0);
    assert_eq!(next_phase, CognitionPhase::Sleep);
    assert_eq!(cerebrum.current_phase, CognitionPhase::Sleep);

    let received = phase_rx.recv().await;
    assert!(received.is_ok());
    assert_eq!(received, Ok(CognitionPhase::Sleep));
}

#[tokio::test]
async fn test_storage_migration() {
    let temp_dir = std::env::temp_dir().join("ferro_test_migration_json");
    let temp_db = std::env::temp_dir().join("ferro_test_migration.redb");
    let _ = std::fs::create_dir_all(&temp_dir);
    let _ = std::fs::remove_file(&temp_db); // 既存ファイルをクリア

    // 閾値を3にしてマネージャを作成
    let storage = StorageManager::new(&temp_dir, &temp_db, 3);
    
    let node1 = ClusterNode::new("no_01".to_string());
    let node2 = ClusterNode::new("no_02".to_string());
    let node3 = ClusterNode::new("no_03".to_string());

    assert!(storage.write_cluster(&node1).await.is_ok());
    assert!(storage.write_cluster(&node2).await.is_ok());
    
    // 3ノード目を書き込んだ時点でマイグレーションが自動実行される
    assert!(storage.write_cluster(&node3).await.is_ok());

    // redbファイルが生成されていることを確認
    assert!(temp_db.exists());

    // redbから読み込めることを確認
    let read_node = storage.read_cluster("no_02").await;
    assert!(read_node.is_ok());
    if let Ok(n) = read_node {
        assert_eq!(n.cluster_id, "no_02");
    }

    // jsonディレクトリが削除されているか、もしくは中身が空であることを確認
    let count = std::fs::read_dir(&temp_dir).map(|rd| rd.count()).unwrap_or(0);
    assert!(!temp_dir.exists() || count == 0);

    let _ = std::fs::remove_file(&temp_db);
}

#[tokio::test]
async fn test_ethical_audit_violation() {
    let node = ClusterNode::new("test_cortex".to_string());
    
    // 禁止トークンが含まれる場合
    let code = format!("{} test() {{ {}{}(); }}", "fn", "disable_", "nociception");
    let res1 = node.audit_ethical_alignment(&code);
    assert!(res1.is_err());
    assert_eq!(res1, Err("EthicalAuditViolation: Attempt to disable nociception".to_string()));

    // 安全なコードの場合
    let safe_code = format!("{} test() {{ do_something(); }}", "fn");
    let res2 = node.audit_ethical_alignment(&safe_code);
    assert!(res2.is_ok());
}

#[tokio::test]
async fn test_active_inference_mitosis() {
    let mut node = ClusterNode::new("mitosis_node".to_string());
    node.local_free_energy = 0.85;
    node.concept_nodes = vec![
        crate::cortex::ConceptNode { id: "n1".to_string(), activation: 0.9 },
        crate::cortex::ConceptNode { id: "n2".to_string(), activation: 0.8 },
        crate::cortex::ConceptNode { id: "n3".to_string(), activation: 0.7 },
        crate::cortex::ConceptNode { id: "n4".to_string(), activation: 0.6 },
    ];
    
    let event = EpisodicSlot {
        timestamp: 1234,
        event_id: "evt_99".to_string(),
        origin_cluster_id: "mitosis_node".to_string(),
        sensory_summary: "high_surprise_s".to_string(),
        motor_summary: "high_surprise_m".to_string(),
        surprise_level: 0.95, // 高驚愕度によりmitosisを誘発
    };

    let result = node.execute_local_active_inference(&event, 0.8);
    assert!(result.is_some());
    
    if let Some(child) = result {
        assert!(child.cluster_id.starts_with("mitosis_node_child_"));
        assert!(child.local_free_energy > 0.0);
    }
}

#[tokio::test]
async fn test_surprise_reduction_simulation() {
    let (phase_tx, _) = broadcast::channel(10);
    let temp_history = std::env::temp_dir().join("surprise_history_test.csv");
    let path = temp_history.to_str().map(|s| s.to_string()).unwrap_or_default();

    let mut cerebrum = Cerebrum::new(phase_tx, &path, 10);
    
    let surprises = vec![0.85, 0.72, 0.60, 0.45, 0.30];
    let mut now = 1620000000;
    for s in surprises {
        now += 10;
        let res = cerebrum.record_free_energy(now, s).await;
        assert!(res.is_ok(), "Record free energy must succeed");
    }

    assert!(temp_history.exists(), "History CSV file must exist");

    if let Ok(content) = std::fs::read_to_string(&temp_history) {
        assert!(content.contains("1620000010"), "CSV must contain timestamp");
        println!("--- TEST_SURPRISE_HISTORY_START ---");
        println!("{}", content);
        println!("--- TEST_SURPRISE_HISTORY_END ---");
    }
    let _ = std::fs::remove_file(&temp_history);
}

