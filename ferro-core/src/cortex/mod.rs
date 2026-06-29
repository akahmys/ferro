pub mod node;
pub mod arena;

pub use node::DynamicClusterNode;
pub use arena::NodeArena;

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

    /// 有糸分裂: 予測誤差が一定値を超えたノードを分裂させる
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
            
            let parent_mutated = self.arena.with_mut_node(parent_id, |node| {
                node.weight /= 2.0;
                node.prediction_error = 0.0;
            });
            assert!(parent_mutated.is_some(), "Error: parent node not found for weight mutation");

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

    /// 学習率の自己組織化的更新（決定論的 libm::exp 採用）
    pub fn update_learning_rates(&mut self, lambda: f64, eta_base: f64, alpha_e: f64) {
        assert!(lambda >= 0.0, "Error: lambda must be non-negative");
        assert!(eta_base > 0.0, "Error: eta_base must be positive");
        assert!(alpha_e > 0.0 && alpha_e < 1.0, "Error: alpha_e must be between 0 and 1");

        let ids = self.arena.ids();
        let mut limit = 0;
        let epsilon = 1e-8;

        for id in ids {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in learning rate update");

            let _ = self.arena.with_mut_node(id, |node| {
                let new_ema = (1.0 - alpha_e) * node.moving_average_error + alpha_e * node.prediction_error;
                node.moving_average_error = new_ema.clamp(0.0, 100.0);

                let diff = node.prediction_error - node.moving_average_error;
                let denom = node.moving_average_error + epsilon;
                let x = lambda * (diff / denom);
                
                let new_eta = eta_base * libm::exp(x);
                node.learning_rate = new_eta.clamp(0.001, 1.0);
            });
        }
        assert!(self.arena.len() < 100_000, "Error: post-condition check for arena size failed");
    }
}

impl Default for Cortex {
    fn default() -> Self {
        Self::new()
    }
}
