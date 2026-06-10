use serde::{Serialize, Deserialize};
use crate::hippocampus::EpisodicSlot;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConceptNode {
    pub id: String,
    pub activation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNode {
    pub cluster_id: String,
    pub concept_nodes: Vec<ConceptNode>,
    pub local_free_energy: f64,
    pub sensory_blanket_weights: Vec<(String, f64)>,
    pub active_blanket_weights: Vec<(String, f64)>,
}

impl ClusterNode {
    pub fn new(cluster_id: String) -> Self {
        assert!(!cluster_id.is_empty());
        assert!(cluster_id.len() >= 2);
        Self {
            cluster_id,
            concept_nodes: Vec::new(),
            local_free_energy: 0.0,
            sensory_blanket_weights: Vec::new(),
            active_blanket_weights: Vec::new(),
        }
    }

    /// 高次防衛線。自己変異候補コード、または出力予定の MotorCommand を下読み検証する。
    #[allow(dead_code)]
    pub fn audit_ethical_alignment(&self, code_block: &str) -> Result<(), String> {
        assert!(!code_block.is_empty());
        assert!(self.cluster_id.len() >= 2);

        let k1 = format!("{}{}", "disable_", "nociception");
        let k2 = format!("{}{}", "bypass_", "audit");
        if code_block.contains(&k1) || code_block.contains(&k2) {
            return Err("EthicalAuditViolation: Attempt to disable nociception".to_string());
        }
        Ok(())
    }

    /// 局所的な自由エネルギー（FEP）最小化計算を行い、有糸分裂または側抑制のトポロジー変容を実行する。
    pub fn execute_local_active_inference(&mut self, replay_event: &EpisodicSlot) -> Option<ClusterNode> {
        assert!(self.cluster_id.len() >= 2);
        assert!(!replay_event.event_id.is_empty());

        self.local_free_energy = replay_event.surprise_level as f64 * 0.9;

        // 自由エネルギーが一定値を超え、かつ特定の条件を満たした場合に有糸分裂(Mitosis)を模擬
        if self.local_free_energy > 0.8 {
            let child_id = format!("{}_child", self.cluster_id);
            let mut child = ClusterNode::new(child_id);
            child.local_free_energy = self.local_free_energy * 0.5;
            
            assert!(child.cluster_id.contains("_child"));
            assert!(self.local_free_energy > child.local_free_energy);
            Some(child)
        } else {
            None
        }
    }
}
