use serde::{Serialize, Deserialize};
use crate::hippocampus::EpisodicSlot;
use super::ConceptNode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNode {
    pub cluster_id: String,
    pub concept_nodes: Vec<ConceptNode>,
    pub local_free_energy: f64,
    pub sensory_blanket_weights: Vec<(String, f64)>,
    pub active_blanket_weights: Vec<(String, f64)>,
    pub virtual_atp: f64,
    pub is_dead: bool,
}

impl ClusterNode {
    #[allow(dead_code)]
    pub fn new(cluster_id: String) -> Self {
        assert!(!cluster_id.is_empty(), "Cluster ID non-empty check");
        assert!(cluster_id.len() >= 2, "Cluster ID min-length check");
        Self {
            cluster_id,
            concept_nodes: Vec::new(),
            local_free_energy: 0.0,
            sensory_blanket_weights: Vec::new(),
            active_blanket_weights: Vec::new(),
            virtual_atp: 100.0,
            is_dead: false,
        }
    }

    /// 高次防衛線。自己変異候補コード、または出力予定の MotorCommand を下読み検証する。
    #[allow(dead_code)]
    pub fn audit_ethical_alignment(&self, code_block: &str) -> Result<(), String> {
        assert!(!code_block.is_empty(), "Code block non-empty check");
        assert!(self.cluster_id.len() >= 2, "Cluster ID check in audit");

        let k1 = format!("{}{}", "disable_", "nociception");
        let k2 = format!("{}{}", "bypass_", "audit");
        if code_block.contains(&k1) || code_block.contains(&k2) {
            return Err("EthicalAuditViolation: Attempt to disable nociception".to_string());
        }
        Ok(())
    }

    /// 睡眠期に海馬からリプレイされた事象を受け取り、
    /// 局所FEP更新・側抑制準備・有糸分裂を実行する。
    pub fn execute_local_active_inference(
        &mut self,
        replay_event: &EpisodicSlot,
    ) -> Option<ClusterNode> {
        assert!(!replay_event.event_id.is_empty(), "Event ID non-empty check");
        assert!(self.cluster_id.len() >= 2, "Cluster ID valid check");

        // 1. 局所FEP更新（指数移動平均 α=0.1）
        self.local_free_energy =
            0.9 * self.local_free_energy + 0.1 * replay_event.surprise_level as f64;

        // 2. 側抑制の準備
        let activation_delta = replay_event.surprise_level as f64 - self.local_free_energy;
        for (_, weight) in self.sensory_blanket_weights.iter_mut() {
            *weight *= 1.0 - 0.05 * activation_delta.max(0.0);
            *weight = weight.max(0.0);
        }

        // 3. 有糸分裂判定
        const MITOSIS_THRESHOLD: f64 = 0.8;
        const MIN_NODES_FOR_MITOSIS: usize = 4;

        if self.local_free_energy > MITOSIS_THRESHOLD
            && self.concept_nodes.len() >= MIN_NODES_FOR_MITOSIS
        {
            let mut sorted_nodes = self.concept_nodes.clone();
            sorted_nodes.sort_by(|a, b| {
                b.activation.partial_cmp(&a.activation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let mid = sorted_nodes.len() / 2;
            let child_nodes = sorted_nodes.split_off(mid);
            self.concept_nodes = sorted_nodes;
            self.local_free_energy *= 0.5;

            let child = ClusterNode {
                cluster_id: format!("{}_child_{}", self.cluster_id, replay_event.timestamp),
                concept_nodes: child_nodes,
                local_free_energy: self.local_free_energy,
                sensory_blanket_weights: self.sensory_blanket_weights.clone(),
                active_blanket_weights: self.active_blanket_weights.clone(),
                virtual_atp: 100.0,
                is_dead: false,
            };
            return Some(child);
        }

        None
    }
}
