use crate::storage::manager::{StorageManager, CLUSTERS_TABLE};
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::storage::backend::StorageBackend;
use redb::ReadableTable;

impl StorageManager {
    pub async fn read_all_clusters(&self) -> Result<Vec<ClusterNode>, String> {
        assert!(self.migration_threshold > 0, "Threshold constraint check");
        let backend_guard = self.backend.read().await;
        let res = match &*backend_guard {
            StorageBackend::ShardedJson { base_path } => {
                let mut loaded_nodes = Vec::new();
                if let Ok(entries) = std::fs::read_dir(base_path) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                                for sub_entry in sub_entries.flatten() {
                                    if sub_entry.path().extension().is_some_and(|ext| ext == "json") {
                                        let bytes = std::fs::read(sub_entry.path()).map_err(|e| e.to_string())?;
                                        let node: ClusterNode = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                                        loaded_nodes.push(node);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(loaded_nodes)
            }
            StorageBackend::RedbKvs { database, .. } => {
                let read_txn = database.begin_read().map_err(|e| e.to_string())?;
                let table = read_txn.open_table(CLUSTERS_TABLE).map_err(|e| e.to_string())?;
                let mut nodes = Vec::new();
                for (_key, val) in table.iter().map_err(|e| e.to_string())?.flatten() {
                    let node: ClusterNode = serde_json::from_slice(val.value()).map_err(|e| e.to_string())?;
                    nodes.push(node);
                }
                Ok(nodes)
            }
        };
        assert!(res.is_ok(), "Read all clusters success check");
        res
    }
}
