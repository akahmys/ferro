use crate::storage::manager::StorageManager;
use crate::cortex::dynamic_cluster::ClusterNode;

#[tokio::test]
async fn test_storage_manager_json() {
    let temp_dir = std::env::temp_dir().join("ferro_test_storage_json");
    let temp_db = std::env::temp_dir().join("ferro_test_storage_json.redb");
    let _ = std::fs::create_dir_all(&temp_dir);
    
    let storage = StorageManager::new(&temp_dir, &temp_db, 100);
    let mut node = ClusterNode::new("test_cluster".to_string());
    node.local_free_energy = 0.5;

    let res = storage.write_cluster(&node).await;
    assert!(res.is_ok());

    let read_res = storage.read_cluster("test_cluster").await;
    assert!(read_res.is_ok());
    if let Ok(read) = read_res {
        assert_eq!(read.cluster_id, "test_cluster");
        assert_eq!(read.local_free_energy, 0.5);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::remove_file(&temp_db);
}

