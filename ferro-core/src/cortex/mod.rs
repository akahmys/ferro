use std::collections::HashMap;
use std::pin::Pin;

pub struct DynamicClusterNode {
    pub id: usize,
    pub weight: f64,
    pub atp: f64,
    pub activity: f64,
    pub prediction_error: f64,
}

pub struct NodeArena {
    nodes: HashMap<usize, Pin<Box<DynamicClusterNode>>>,
    next_id: usize,
}

impl NodeArena {
    pub fn new() -> Self {
        let arena = Self {
            nodes: HashMap::new(),
            next_id: 1,
        };
        assert!(arena.nodes.is_empty(), "Error: arena must be empty");
        assert!(arena.next_id == 1, "Error: starting ID must be 1");
        arena
    }

    pub fn create_node(&mut self, weight: f64, atp: f64) -> usize {
        assert!(weight.is_finite(), "Error: weight must be finite");
        assert!(atp >= 0.0, "Error: atp must be non-negative");

        let id = self.next_id;
        self.next_id += 1;

        let node = Box::pin(DynamicClusterNode {
            id,
            weight,
            atp,
            activity: 0.0,
            prediction_error: 0.0,
        });
        self.nodes.insert(id, node);

        assert!(self.nodes.contains_key(&id), "Error: insertion failed");
        assert!(id > 0, "Error: invalid node ID created");
        id
    }

    pub fn with_mut_node<F, R>(&mut self, id: usize, f: F) -> Option<R>
    where
        F: FnOnce(&mut DynamicClusterNode) -> R,
    {
        assert!(id > 0, "Error: invalid node ID requested");
        if let Some(node) = self.nodes.get_mut(&id) {
            let node_mut = unsafe { node.as_mut().get_unchecked_mut() };
            let res = f(node_mut);
            assert!(node_mut.atp.is_finite(), "Error: ATP must be finite after mutation");
            assert!(node_mut.weight.is_finite(), "Error: weight must be finite after mutation");
            Some(res)
        } else {
            None
        }
    }

    pub fn get_node(&self, id: usize) -> Option<&DynamicClusterNode> {
        assert!(id > 0, "Error: invalid node ID requested");
        let opt = self.nodes.get(&id).map(|pinned| pinned.as_ref().get_ref());
        assert!(opt.as_ref().map(|n| n.id == id).unwrap_or(true), "Error: node ID mismatch");
        opt
    }

    pub fn remove_node(&mut self, id: usize) -> Option<Pin<Box<DynamicClusterNode>>> {
        assert!(id > 0, "Error: invalid node ID requested");
        let removed = self.nodes.remove(&id);
        assert!(removed.as_ref().map(|n| n.id == id).unwrap_or(true), "Error: removed node ID mismatch");
        removed
    }

    pub fn ids(&self) -> Vec<usize> {
        let mut ids: Vec<usize> = self.nodes.keys().cloned().collect();
        ids.sort();
        assert!(ids.len() == self.nodes.len(), "Error: size mismatch in IDs");
        assert!(ids.is_empty() || ids[0] > 0, "Error: IDs must be positive");
        ids
    }

    pub fn len(&self) -> usize {
        let length = self.nodes.len();
        assert!(length <= self.nodes.len(), "Error: size inconsistent");
        length
    }

    pub fn is_empty(&self) -> bool {
        let empty = self.nodes.is_empty();
        assert!(empty == (self.nodes.len() == 0), "Error: inconsistency in empty check");
        empty
    }

}

impl Default for NodeArena {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Cortex {
    pub arena: NodeArena,
}

impl Cortex {
    pub fn new() -> Self {
        let cortex = Self {
            arena: NodeArena::new(),
        };
        assert!(cortex.arena.is_empty(), "Error: cortex arena must be empty");
        assert!(cortex.arena.ids().is_empty(), "Error: cortex arena IDs must be empty");
        cortex
    }

    /// 有糸分裂: 予測誤差が一定値（例：2.0）を超えたノードを分裂させる
    pub fn perform_mitosis(&mut self, error_threshold: f64) {
        assert!(error_threshold > 0.0, "Error: threshold must be positive");
        assert!(self.arena.len() < 100_000, "Error: too many nodes in cortex");

        let mut to_split = Vec::new();
        let ids = self.arena.ids();
        let mut limit = 0;
        for id in ids {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in mitosis scan");
            if let Some(node) = self.arena.get_node(id).filter(|n| n.prediction_error > error_threshold) {
                to_split.push((id, node.weight, node.atp));
            }
        }

        let mut mitosis_limit = 0;
        for (parent_id, weight, atp) in to_split {
            mitosis_limit += 1;
            assert!(mitosis_limit <= 100_000, "Error: Loop limit exceeded in mitosis execution");
            
            // 親ノードの重みを半分にする
            let parent_mutated = self.arena.with_mut_node(parent_id, |node| {
                node.weight /= 2.0;
                node.prediction_error = 0.0;
            });
            assert!(parent_mutated.is_some(), "Error: parent node not found for weight mutation");

            // 新しいノードを作成
            let new_node_id = self.arena.create_node(weight / 2.0, atp);
            assert!(new_node_id > 0, "Error: new node must have positive ID");
        }
    }

    /// 側抑制: 最大活性のノードが他を抑制する
    pub fn perform_lateral_inhibition(&mut self, inhibition_factor: f64) {
        assert!(inhibition_factor >= 0.0, "Error: factor must be non-negative");
        assert!(inhibition_factor <= 1.0, "Error: factor must be less than or equal to 1.0");

        let ids = self.arena.ids();
        let mut max_activity = 0.0;
        let mut scan_limit = 0;
        for &id in &ids {
            scan_limit += 1;
            assert!(scan_limit <= 100_000, "Error: Loop limit in inhibition scan");
            if let Some(node) = self.arena.get_node(id).filter(|n| n.activity > max_activity) {
                max_activity = node.activity;
            }
        }

        let mut apply_limit = 0;
        for id in ids {
            apply_limit += 1;
            assert!(apply_limit <= 100_000, "Error: Loop limit in inhibition apply");
            let _ = self.arena.with_mut_node(id, |node| {
                if (node.activity - max_activity).abs() > 1e-9 {
                    node.activity -= max_activity * inhibition_factor;
                    if node.activity < 0.0 {
                        node.activity = 0.0;
                    }
                }
            });
        }
    }

    /// 代謝と餓死: 各ノードのATPを減少し、0以下のノードを削除する
    pub fn perform_metabolism(&mut self, atp_base_consumption: f64) -> Vec<usize> {
        assert!(atp_base_consumption >= 0.0, "Error: base consumption must be non-negative");
        assert!(atp_base_consumption < 10.0, "Error: base consumption too high");

        let ids = self.arena.ids();
        let mut starved_nodes = Vec::new();
        let mut metab_limit = 0;

        for id in ids {
            metab_limit += 1;
            assert!(metab_limit <= 100_000, "Error: Loop limit in metabolism");
            let is_starved = self.arena.with_mut_node(id, |node| {
                let cost = atp_base_consumption + (node.activity * 0.1);
                node.atp -= cost;
                node.atp <= 0.0
            });

            if let Some(true) = is_starved {
                starved_nodes.push(id);
            }
        }

        let mut prune_limit = 0;
        for id in &starved_nodes {
            prune_limit += 1;
            assert!(prune_limit <= 100_000, "Error: Loop limit in pruning");
            let removed = self.arena.remove_node(*id);
            assert!(removed.is_some(), "Error: failed to remove starved node");
        }

        assert!(starved_nodes.len() <= metab_limit, "Error: starved nodes cannot exceed processed nodes");
        starved_nodes
    }
}

impl Default for Cortex {
    fn default() -> Self {
        Self::new()
    }
}
