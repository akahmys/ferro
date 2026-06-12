use crate::storage::manager::StorageManager;
use crate::cortex::dynamic_cluster::ClusterNode;
use std::sync::Arc;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_concurrent_storage_migration_stress() {
    let rand_id = rand::random::<u32>();
    let temp_dir = std::env::temp_dir().join(format!("ferro_stress_migration_json_{}", rand_id));
    let temp_db = std::env::temp_dir().join(format!("ferro_stress_migration_{}.redb", rand_id));
    let _ = std::fs::create_dir_all(&temp_dir);
    let _ = std::fs::remove_file(&temp_db);

    // Set a threshold of 50 to trigger auto-migration during concurrent writes
    let storage = Arc::new(StorageManager::new(&temp_dir, &temp_db, 50));
    
    let num_tasks = 10;
    let writes_per_task = 10; // Total 100 writes, exceeding the 50 threshold
    let barrier = Arc::new(Barrier::new(num_tasks));
    
    let mut handles = Vec::new();

    for task_idx in 0..num_tasks {
        let storage_clone = Arc::clone(&storage);
        let barrier_clone = Arc::clone(&barrier);
        
        let handle = tokio::spawn(async move {
            // Synchronize all tasks to start concurrently
            barrier_clone.wait().await;
            
            for write_idx in 0..writes_per_task {
                let node_id = format!("cn_{:02}_{:02}", task_idx, write_idx);
                let node = ClusterNode::new(node_id.clone());
                
                // Concurrent write
                let write_res = storage_clone.write_cluster(&node).await;
                assert!(write_res.is_ok(), "Concurrent write must succeed");
                
                // Concurrent read to verify consistency
                let read_res = storage_clone.read_cluster(&node_id).await;
                assert!(read_res.is_ok(), "Concurrent read must succeed");
                if let Ok(c) = read_res {
                    assert_eq!(c.cluster_id, node_id);
                }
            }
        });
        
        handles.push(handle);
    }

    // Wait for all concurrent threads to finish
    for handle in handles {
        let join_res = handle.await;
        assert!(join_res.is_ok(), "Task thread finished successfully without panics");
    }

    // Assert that the database file was successfully created
    assert!(temp_db.exists(), "redb database file must be successfully generated");

    // Double check that we can read back all nodes from the migrated redb KVS
    for task_idx in 0..num_tasks {
        for write_idx in 0..writes_per_task {
            let node_id = format!("cn_{:02}_{:02}", task_idx, write_idx);
            let read_res = storage.read_cluster(&node_id).await;
            assert!(read_res.is_ok(), "Should read back migrated nodes from redb");
            if let Ok(c) = read_res {
                assert_eq!(c.cluster_id, node_id);
            }
        }
    }

    // JSON base directory must either be removed or empty
    let count = std::fs::read_dir(&temp_dir).map(|rd| rd.count()).unwrap_or(0);
    assert!(!temp_dir.exists() || count == 0, "JSON directory should be cleaned up after migration");

    // Clean up temporary files
    let _ = std::fs::remove_file(&temp_db);
    let _ = std::fs::remove_dir_all(&temp_dir);
}
