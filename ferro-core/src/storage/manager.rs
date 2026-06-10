use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::storage::backend::{
    StorageBackend, count_json_files, write_json_cluster, read_json_cluster,
};

pub const CLUSTERS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("clusters");

pub use crate::storage::backend::get_safe_path;

pub struct StorageManager {
    pub backend: Arc<RwLock<StorageBackend>>,
    pub migration_threshold: usize,
    pub redb_path: PathBuf,
}

impl StorageManager {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        json_dir: P1,
        redb_path: P2,
        threshold: usize,
    ) -> Self {
        let json_path = json_dir.as_ref().to_path_buf();
        let rdb_path = redb_path.as_ref().to_path_buf();
        assert!(!json_path.as_os_str().is_empty());
        assert!(!rdb_path.as_os_str().is_empty());
        assert!(threshold > 0);

        Self {
            backend: Arc::new(RwLock::new(StorageBackend::ShardedJson { base_path: json_path })),
            migration_threshold: threshold,
            redb_path: rdb_path,
        }
    }

    pub async fn write_cluster(&self, node: &ClusterNode) -> Result<(), String> {
        assert!(!node.cluster_id.is_empty());
        assert!(node.cluster_id.len() >= 2);

        let mut should_migrate = false;

        {
            let backend_guard = self.backend.read().await;
            match &*backend_guard {
                StorageBackend::ShardedJson { base_path } => {
                    write_json_cluster(base_path, node).await?;
                    let count = count_json_files(base_path);
                    should_migrate = count >= self.migration_threshold;
                }
                StorageBackend::RedbKvs { database, .. } => {
                    let serialized = serde_json::to_vec(node)
                        .map_err(|e| e.to_string())?;
                    let write_txn = database.begin_write()
                        .map_err(|e| e.to_string())?;
                    {
                        let mut table = write_txn.open_table(CLUSTERS_TABLE)
                            .map_err(|e| e.to_string())?;
                        table.insert(node.cluster_id.as_str(), serialized.as_slice())
                            .map_err(|e| e.to_string())?;
                    }
                    write_txn.commit().map_err(|e| e.to_string())?;
                }
            }
        }

        if should_migrate {
            self.trigger_automatic_migration().await?;
        }
        Ok(())
    }
    pub async fn read_cluster(&self, cluster_id: &str) -> Result<ClusterNode, String> {
        assert!(!cluster_id.is_empty());
        assert!(cluster_id.len() >= 2);
        let backend_guard = self.backend.read().await;
        match &*backend_guard {
            StorageBackend::ShardedJson { base_path } => {
                read_json_cluster(base_path, cluster_id).await
            }
            StorageBackend::RedbKvs { database, .. } => {
                let read_txn = database.begin_read().map_err(|e| e.to_string())?;
                let table = read_txn.open_table(CLUSTERS_TABLE).map_err(|e| e.to_string())?;
                let value_opt = table.get(cluster_id).map_err(|e| e.to_string())?;
                if let Some(guard) = value_opt {
                    let node: ClusterNode = serde_json::from_slice(guard.value()).map_err(|e| e.to_string())?;
                    Ok(node)
                } else {
                    Err(format!("Cluster {} not found in KVS", cluster_id))
                }
            }
        }
    }
}
