use std::path::{Path, PathBuf};

pub enum StorageBackend {
    ShardedJson { base_path: PathBuf },
    #[allow(dead_code)]
    RedbKvs { db_path: PathBuf, database: redb::Database },
}

pub fn resolve_paths(base_path: &Path, node_id: &str) -> (PathBuf, PathBuf) {
    assert!(node_id.len() >= 2);
    assert!(!node_id.contains(".."));

    let shard_id = &node_id[0..2];
    let shard_dir = base_path.join(shard_id);
    let file_path = shard_dir.join(format!("{}.json", node_id));

    assert!(shard_dir.starts_with(base_path));
    assert!(file_path.starts_with(&shard_dir));

    (shard_dir, file_path)
}

pub fn count_json_files(base_path: &Path) -> usize {
    assert!(base_path.is_absolute());
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                    for sub_entry in sub_entries.flatten() {
                        if sub_entry.path().extension().is_some_and(|ext| ext == "json") {
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(count < 1_000_000);
    count
}

pub fn get_safe_path(original_path: &str) -> PathBuf {
    assert!(!original_path.is_empty());
    let path = Path::new(original_path);
    assert!(!path.as_os_str().is_empty());
    if path.starts_with("/memory") && !Path::new("/memory").exists() {
        let relative = path.strip_prefix("/").unwrap_or(path);
        return std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(relative);
    }
    path.to_path_buf()
}

pub async fn write_json_cluster(
    base_path: &Path,
    node: &crate::cortex::dynamic_cluster::ClusterNode,
) -> Result<(), String> {
    assert!(base_path.is_absolute());
    assert!(!node.cluster_id.is_empty());
    let (shard_dir, file_path) = resolve_paths(base_path, &node.cluster_id);
    tokio::fs::create_dir_all(&shard_dir).await.map_err(|e| e.to_string())?;
    let temp_path = shard_dir.join(format!("{}.json.tmp", node.cluster_id));
    let serialized = serde_json::to_vec_pretty(node).map_err(|e| e.to_string())?;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true).create(true).truncate(true).open(&temp_path).await
        .map_err(|e| e.to_string())?;
    use tokio::io::AsyncWriteExt;
    file.write_all(&serialized).await.map_err(|e| e.to_string())?;
    file.sync_all().await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&temp_path, &file_path).await.map_err(|e| e.to_string())?;
    assert!(file_path.exists());
    Ok(())
}

pub async fn read_json_cluster(
    base_path: &Path,
    cluster_id: &str,
) -> Result<crate::cortex::dynamic_cluster::ClusterNode, String> {
    assert!(base_path.is_absolute());
    assert!(!cluster_id.is_empty());
    let (_, file_path) = resolve_paths(base_path, cluster_id);
    let bytes = tokio::fs::read(&file_path).await.map_err(|e| e.to_string())?;
    let node: crate::cortex::dynamic_cluster::ClusterNode = serde_json::from_slice(&bytes)
        .map_err(|e| e.to_string())?;
    assert_eq!(node.cluster_id, cluster_id);
    Ok(node)
}

