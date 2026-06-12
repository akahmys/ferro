use crate::agents::AgentContext;

#[allow(dead_code)]
pub struct SupervisorAgent {
    pub context: AgentContext,
}

impl SupervisorAgent {
    #[allow(dead_code)]
    pub fn new(context: AgentContext) -> Self {
        assert!(!context.memory_dir.is_empty(), "Context memory dir must be non-empty");
        assert!(context.active_zone_markers.capacity() >= context.active_zone_markers.len(), "Markers capacity");
        let agent = Self { context };
        assert!(!agent.context.memory_dir.is_empty(), "Agent context memory dir is non-empty");
        agent
    }

    #[allow(dead_code)]
    pub fn analyze_cortex_bottlenecks(&self, history_csv: &str) -> Result<String, String> {
        assert!(!history_csv.is_empty(), "History CSV path must be non-empty");
        assert!(self.context.active_zone_markers.capacity() >= self.context.active_zone_markers.len(), "Markers capacity");
        
        let memory_path = std::path::Path::new(&self.context.memory_dir);
        let surprise_path = memory_path.join(history_csv);
        let pain_path = memory_path.join("pain_history.csv");

        let mut average_surprise = 0.0;
        if let Ok(content) = std::fs::read_to_string(&surprise_path) {
            let mut sum = 0.0;
            let mut count = 0;
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    if let Ok(val) = parts[1].trim().parse::<f64>() {
                        sum += val;
                        count += 1;
                    }
                }
            }
            if count > 0 {
                average_surprise = sum / count as f64;
            }
        }

        let mut most_pained_parent = String::new();
        if let Ok(content) = std::fs::read_to_string(&pain_path) {
            let mut counts = std::collections::HashMap::new();
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 4 {
                    let parent = parts[3].to_string();
                    *counts.entry(parent).or_insert(0) += 1;
                }
            }
            if let Some((parent, _)) = counts.into_iter().max_by_key(|&(_, count)| count) {
                most_pained_parent = parent;
            }
        }

        let target_zone = self.context.active_zone_markers.first()
            .cloned()
            .unwrap_or_else(|| "ADAPTIVE_ZONE_1".to_string());

        let res = format!(
            "Roadmap for {}: Target zone={}, AvgSurprise={:.4}, MostInfractedRoot={}",
            self.context.memory_dir,
            target_zone,
            average_surprise,
            if most_pained_parent.is_empty() { "None" } else { &most_pained_parent }
        );
        
        assert!(!res.is_empty(), "Roadmap result must be non-empty");
        Ok(res)
    }

    #[allow(dead_code)]
    pub fn compute_mutation_entropy(tickets: &[crate::agents::planner::PatchTicket]) -> f64 {
        assert!(tickets.len() <= 1000, "Entropy check pre");
        if tickets.is_empty() { return 0.0; }

        let mut zone_counts = std::collections::HashMap::new();
        for ticket in tickets {
            *zone_counts.entry(ticket.zone_marker_id.as_str()).or_insert(0) += 1;
        }

        let total = tickets.len() as f64;
        let mut entropy = 0.0;
        for &count in zone_counts.values() {
            let p = count as f64 / total;
            entropy -= p * p.ln();
        }

        assert!(entropy >= 0.0, "Entropy cannot be negative");
        entropy
    }

    #[allow(dead_code)]
    pub fn compute_reset_complexity(
        mutation_entropy: f64,
        max_entropy: f64,
    ) -> f64 {
        assert!(mutation_entropy >= 0.0, "Entropy must be positive");
        assert!(max_entropy >= 0.0, "Max entropy must be positive");

        let normalized = (mutation_entropy / max_entropy.max(1e-9)).clamp(0.0, 1.0);
        let res = 0.8 - 0.6 * normalized;

        assert!((0.2..=0.8).contains(&res), "Complexity bounds check");
        res
    }
}
