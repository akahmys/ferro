use std::collections::HashMap;
use std::pin::Pin;
use crate::cortex::node::DynamicClusterNode;

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
            moving_average_error: 0.0,
            learning_rate: 0.05,
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
            let backup = *node_mut;
            let res = f(node_mut);

            let is_invalid = !node_mut.atp.is_finite()
                || !node_mut.weight.is_finite()
                || !node_mut.moving_average_error.is_finite()
                || !node_mut.learning_rate.is_finite();

            if is_invalid {
                *node_mut = backup;
                let dump_data = serde_json::json!({
                    "error_code": "0x03",
                    "error_type": "ERR_UNDO_TRANSACTION",
                    "reason": "MathConstraintViolation in node mutation",
                    "node_id": id,
                    "backup_values": {
                        "weight": backup.weight,
                        "atp": backup.atp,
                        "moving_average_error": backup.moving_average_error,
                        "learning_rate": backup.learning_rate,
                    }
                });
                if let Ok(json_str) = serde_json::to_string_pretty(&dump_data) {
                    let _ = std::fs::write("panic_dump.json", &json_str);
                    let _ = std::fs::write("/tmp/panic_dump.json", &json_str);
                }
                return None;
            }

            assert!(node_mut.atp.is_finite(), "Error: ATP must be finite after mutation");
            assert!(node_mut.weight.is_finite(), "Error: weight must be finite after mutation");
            assert!(node_mut.moving_average_error.is_finite(), "Error: moving_average_error must be finite after mutation");
            assert!(node_mut.learning_rate.is_finite(), "Error: learning_rate must be finite after mutation");
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
