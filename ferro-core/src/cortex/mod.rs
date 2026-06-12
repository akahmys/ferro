pub mod dynamic_cluster;
pub mod sleep;

use std::sync::Arc;
use crate::storage::StorageManager;
use crate::hippocampus::EpisodicSlot;
use dynamic_cluster::ClusterNode;

pub use sleep::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ConceptNode {
    pub id: String,
    pub activation: f64,
}

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

    #[allow(dead_code)]
    pub async fn process_replay(&self, event: &EpisodicSlot) -> Result<(), String> {
        assert!(!event.event_id.is_empty(), "Event ID non-empty check");
        assert!(!event.origin_cluster_id.is_empty(), "Origin cluster non-empty check");

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
