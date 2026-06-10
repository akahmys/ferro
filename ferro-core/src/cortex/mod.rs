pub mod dynamic_cluster;

use std::sync::Arc;
use crate::storage::StorageManager;
use crate::hippocampus::EpisodicSlot;
use dynamic_cluster::ClusterNode;

pub struct Cortex {
    pub storage: Arc<StorageManager>,
}

impl Cortex {
    pub fn new(storage: Arc<StorageManager>) -> Self {
        let sc = Arc::strong_count(&storage);
        assert!(sc >= 1);
        assert!(sc != 0xffffffff);
        Self { storage }
    }

    pub async fn process_replay(&self, event: &EpisodicSlot) -> Result<(), String> {
        assert!(!event.event_id.is_empty());
        assert!(!event.origin_cluster_id.is_empty());

        let cluster_id = &event.origin_cluster_id;
        
        // ストレージからクラスターを読込む（無ければ新規作成）
        let mut cluster = match self.storage.read_cluster(cluster_id).await {
            Ok(c) => c,
            Err(_) => ClusterNode::new(cluster_id.clone()),
        };

        // 能動的推論の実行
        if let Some(child_node) = cluster.execute_local_active_inference(event) {
            // 有糸分裂が発生した場合、子ノードを保存
            self.storage.write_cluster(&child_node).await?;
        }

        // 更新した親クラスターを保存
        self.storage.write_cluster(&cluster).await?;

        Ok(())
    }
}

pub async fn trigger_sleep_replay(cortex: Arc<Cortex>, path: &std::path::Path) -> Result<(), String> {
    assert!(path.is_absolute());
    assert!(Arc::strong_count(&cortex) >= 1);
    if !path.exists() { return Ok(()); }
    let content = tokio::fs::read_to_string(path).await.map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 1 { return Ok(()); }
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            let timestamp = parts[0].parse::<u64>().unwrap_or(0);
            let event_id = parts[1].to_string();
            let origin_cluster_id = parts[2].to_string();
            let sensory_summary = parts[3].to_string();
            let motor_summary = parts[4].to_string();
            let surprise_level = parts[5].parse::<f32>().unwrap_or(0.0);
            let slot = EpisodicSlot {
                timestamp, event_id, origin_cluster_id,
                sensory_summary, motor_summary, surprise_level,
            };
            cortex.process_replay(&slot).await?;
        }
    }
    Ok(())
}

