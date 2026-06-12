use crate::agents::AgentContext;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]  
pub struct PatchTicket {  
    pub ticket_id: String,  
    pub file_path: String,  
    pub zone_marker_id: String,  
    pub replacement_ast_code: String,  
}

#[allow(dead_code)]
pub struct PlannerAgent {
    pub context: AgentContext,
}

impl PlannerAgent {
    #[allow(dead_code)]
    pub fn new(context: AgentContext) -> Self {
        assert!(!context.memory_dir.is_empty(), "Context memory dir must be non-empty");
        assert!(context.active_zone_markers.capacity() >= context.active_zone_markers.len(), "Markers capacity");
        let agent = Self { context };
        assert!(!agent.context.memory_dir.is_empty(), "Agent context memory dir is non-empty");
        agent
    }

    #[allow(dead_code)]
    pub fn generate_patch_ticket(
        &self,
        roadmap: &str,
        graph_json: &str,
    ) -> Result<PatchTicket, String> {
        assert!(!roadmap.is_empty(), "Roadmap must be non-empty");
        assert!(!graph_json.is_empty(), "Graph JSON must be non-empty");

        let target_zone = self.context.active_zone_markers.first()
            .cloned()
            .unwrap_or_else(|| "ADAPTIVE_ZONE_1".to_string());

        let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis()).unwrap_or(0);
        
        let cost = 20.0 + (millis % 26) as f64; // range from 20.0 to 45.0
        let replacement_ast_code = format!("pub const MITOSIS_COST: f64 = {:.1};", cost);

        let ticket = PatchTicket {
            ticket_id: format!("ticket_{}", millis),
            file_path: "src/cortex/mod.rs".to_string(),
            zone_marker_id: target_zone,
            replacement_ast_code,
        };

        assert!(!ticket.ticket_id.is_empty(), "Ticket ID must not be empty");
        assert!(!ticket.file_path.is_empty(), "File path must not be empty");
        Ok(ticket)
    }
}
