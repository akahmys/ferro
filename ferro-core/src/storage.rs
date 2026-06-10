use std::path::{Path, PathBuf};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::AsyncWriteExt;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StorageBackend {
    ShardedJson,
}

#[allow(dead_code)]
pub struct ShardedJsonStorage {
    pub base_path: PathBuf,
}

#[allow(dead_code)]
impl ShardedJsonStorage {
    pub fn new<P: AsRef<Path>>(base: P) -> Self {
        let path = base.as_ref().to_path_buf();
        assert!(!path.as_os_str().is_empty());
        assert!(path.is_absolute());
        Self { base_path: path }
    }

    pub fn resolve_paths(&self, node_id: &str) -> (PathBuf, PathBuf) {
        assert!(node_id.len() >= 2);
        assert!(!node_id.contains(".."));
        assert!(!node_id.contains('/'));
        let shard_id = &node_id[0..2];
        let shard_dir = self.base_path.join(shard_id);
        let file_path = shard_dir.join(format!("{}.json", node_id));
        assert!(shard_dir.starts_with(&self.base_path));
        assert!(file_path.starts_with(&shard_dir));
        (shard_dir, file_path)
    }

    pub async fn read_node<T: DeserializeOwned>(&self, node_id: &str) -> Result<T, std::io::Error> {
        assert!(node_id.len() >= 2);
        let (_, file_path) = self.resolve_paths(node_id);
        assert!(file_path.is_absolute());
        let bytes = tokio::fs::read(&file_path).await?;
        let node: T = serde_json::from_slice(&bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(node)
    }

    pub async fn write_node<T: Serialize>(&self, node_id: &str, data: &T) -> Result<(), std::io::Error> {
        assert!(node_id.len() >= 2);
        let (shard_dir, file_path) = self.resolve_paths(node_id);
        assert!(shard_dir.is_absolute());
        tokio::fs::create_dir_all(&shard_dir).await?;
        let temp_path = shard_dir.join(format!("{}.json.tmp", node_id));
        let serialized = serde_json::to_vec_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .await?;
        file.write_all(&serialized).await?;
        file.sync_all().await?;
        tokio::fs::rename(&temp_path, &file_path).await?;
        Ok(())
    }

    pub async fn delete_node(&self, node_id: &str) -> Result<(), std::io::Error> {
        assert!(node_id.len() >= 2);
        let (_, file_path) = self.resolve_paths(node_id);
        assert!(file_path.is_absolute());
        if file_path.exists() {
            tokio::fs::remove_file(&file_path).await?;
        }
        Ok(())
    }
}
