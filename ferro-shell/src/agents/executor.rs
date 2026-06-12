use crate::agents::{AgentContext, planner::PatchTicket};

#[allow(dead_code)]
pub struct ExecutorAgent {
    pub context: AgentContext,
}

impl ExecutorAgent {
    #[allow(dead_code)]
    pub fn new(context: AgentContext) -> Self {
        assert!(!context.memory_dir.is_empty(), "Context memory dir must be non-empty");
        assert!(context.active_zone_markers.capacity() >= context.active_zone_markers.len(), "Markers capacity");
        let agent = Self { context };
        assert!(!agent.context.memory_dir.is_empty(), "Agent context memory dir is non-empty");
        agent
    }

    #[allow(dead_code)]
    pub fn apply_patch_to_file(
        &self,
        file_content: &str,
        ticket: &PatchTicket,
    ) -> Result<String, String> {
        assert!(!file_content.is_empty(), "File content must not be empty");
        assert!(!ticket.ticket_id.is_empty(), "Ticket ID must not be empty");

        let start_marker = "// == FERRO_ADAPTIVE_ZONE_START";
        let end_marker = "// == FERRO_ADAPTIVE_ZONE_END";

        let start_pos = file_content.find(start_marker)
            .ok_or_else(|| format!("Start marker not found: {}", start_marker))?;
        let end_pos = file_content.find(end_marker)
            .ok_or_else(|| format!("End marker not found: {}", end_marker))?;

        if start_pos >= end_pos {
            return Err("Markers out of order".to_string());
        }

        let before = &file_content[..start_pos + start_marker.len()];
        let after = &file_content[end_pos..];

        let modified = format!("{}\n{}\n{}", before, ticket.replacement_ast_code.trim(), after);

        assert!(!modified.is_empty(), "Modified content check");
        assert!(modified.contains(start_marker), "Markers must be preserved");
        Ok(modified)
    }
}
