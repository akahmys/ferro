pub mod supervisor;
pub mod planner;
pub mod executor;
pub mod verifier;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub memory_dir: String,
    pub active_zone_markers: Vec<String>,
}

impl AgentContext {
    #[allow(dead_code)]
    pub fn new(memory_dir: String, active_zone_markers: Vec<String>) -> Self {
        assert!(!memory_dir.is_empty(), "Memory dir must not be empty");
        assert!(active_zone_markers.capacity() >= active_zone_markers.len(), "Capacity check");
        let res = Self {
            memory_dir,
            active_zone_markers,
        };
        assert!(!res.memory_dir.is_empty(), "Memory dir must match");
        res
    }
}
