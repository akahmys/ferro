use std::fs;
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::storage::manager::{StorageManager, CLUSTERS_TABLE};
use crate::storage::backend::StorageBackend;

impl StorageManager {
    pub async fn trigger_automatic_migration(&self) -> Result<(), String> {
        let mut backend_guard = self.backend.write().await;
        
        let base_path = match &*backend_guard {
            StorageBackend::ShardedJson { base_path } => base_path.clone(),
            StorageBackend::RedbKvs { .. } => return Ok(()),
        };

        assert!(base_path.is_absolute());
        assert!(!self.redb_path.as_os_str().is_empty());

        let mut loaded_nodes = Vec::new();

        if let Ok(entries) = fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            if sub_entry.path().extension().is_some_and(|ext| ext == "json") {
                                let bytes = fs::read(sub_entry.path())
                                    .map_err(|e| e.to_string())?;
                                let node: ClusterNode = serde_json::from_slice(&bytes)
                                    .map_err(|e| e.to_string())?;
                                loaded_nodes.push(node);
                            }
                        }
                    }
                }
            }
        }

        if let Some(parent) = self.redb_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let database = redb::Database::create(&self.redb_path)
            .map_err(|e| e.to_string())?;

        let write_txn = database.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn.open_table(CLUSTERS_TABLE).map_err(|e| e.to_string())?;
            for node in &loaded_nodes {
                let serialized = serde_json::to_vec(node).map_err(|e| e.to_string())?;
                table.insert(node.cluster_id.as_str(), serialized.as_slice())
                    .map_err(|e| e.to_string())?;
            }
        }
        write_txn.commit().map_err(|e| e.to_string())?;

        *backend_guard = StorageBackend::RedbKvs {
            db_path: self.redb_path.clone(),
            database,
        };

        let _ = fs::remove_dir_all(&base_path);

        assert!(self.redb_path.exists());
        assert!(loaded_nodes.len() < 1_000_000);

        Ok(())
    }
}
