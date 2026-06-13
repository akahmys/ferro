use crate::agents;
use std::path::Path;

/// Runs the 4-agent natural selection loop on the host when sleep phase is detected.
pub async fn run_evolution_cycle(memory_host_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!memory_host_path.is_empty(), "Memory path must be non-empty");
    assert!(Path::new(memory_host_path).is_dir(), "Memory directory must exist");
    
    let ctx = agents::AgentContext::new(
        memory_host_path.to_string(),
        vec!["ADAPTIVE_ZONE_1".to_string()],
    );

    let supervisor = agents::supervisor::SupervisorAgent::new(ctx.clone());
    let planner = agents::planner::PlannerAgent::new(ctx.clone());
    let executor = agents::executor::ExecutorAgent::new(ctx.clone());
    let verifier = agents::verifier::VerifierAgent::new(ctx.clone(), "ferro-sandbox".to_string());

    let roadmap = supervisor.analyze_cortex_bottlenecks("surprise_history.csv").map_err(|e| e.to_string())?;
    
    let kg_path = Path::new(memory_host_path).join("knowledge_graph");
    let kg_json = if kg_path.is_file() {
        std::fs::read_to_string(&kg_path)?
    } else {
        r#"{"nodes": []}"#.to_string()
    };

    let ticket = planner.generate_patch_ticket(&roadmap, &kg_json).map_err(|e| e.to_string())?;
    
    let curr = std::env::current_dir()?;
    let core_dir = if curr.ends_with("ferro-shell") {
        curr.parent().ok_or("No parent directory found")?.join("ferro-core")
    } else {
        curr.join("ferro-core")
    };
    let target_file_path = core_dir.join(&ticket.file_path);
    if !target_file_path.exists() {
        return Ok(());
    }

    let original_content = std::fs::read_to_string(&target_file_path)?;
    let backup_path = target_file_path.with_extension("bak");
    std::fs::write(&backup_path, &original_content)?;

    let modified = executor.apply_patch_to_file(&original_content, &ticket).map_err(|e| e.to_string())?;
    std::fs::write(&target_file_path, &modified)?;

    let surprise_history_file = Path::new(memory_host_path).join("surprise_history.csv");
    let episodic_buffer_file = Path::new(memory_host_path).join("episodic_buffer.csv");

    let surprise_before = load_surprise_history(&surprise_history_file);
    let surprise_after = load_episodic_surprise(&episodic_buffer_file);

    let mut verified = false;
    if verifier.build_sandbox_image().await.is_ok() {
        let (exit_code, sandbox_output) = verifier.execute_secure_sandbox_run("ferro-sandbox-run", &core_dir.to_string_lossy()).await.unwrap_or((-1, String::new()));
        
        let surprise_after_sim = parse_surprise_from_output(&sandbox_output);
        let final_surprise_after = if !surprise_after_sim.is_empty() {
            surprise_after_sim
        } else {
            surprise_after
        };

        let report_res = verifier.compile_and_test_report(
            exit_code,
            &sandbox_output,
            &sandbox_output,
            "", "",
            std::slice::from_ref(&ticket),
            1,
            &surprise_before,
            &final_surprise_after,
            true
        );
        if let Ok(report) = report_res {
            if let Some(fitness) = report.get("fitness").and_then(|f| f.as_f64()) {
                if fitness > 0.0 {
                    verified = true;
                }
            }
        }
    }

    if verified {
        let _ = std::fs::remove_file(backup_path);
        let entropy = agents::supervisor::SupervisorAgent::compute_mutation_entropy(&[ticket]);
        let reset_complexity = agents::supervisor::SupervisorAgent::compute_reset_complexity(entropy, 2.0);
        
        let zpd_path = Path::new(memory_host_path).join("zpd_control.json");
        let payload = serde_json::json!({ "complexity_level": reset_complexity });
        let bytes = serde_json::to_vec(&payload)?;
        
        let tmp_path = zpd_path.with_extension("tmp");
        std::fs::write(&tmp_path, &bytes)?;
        std::fs::rename(&tmp_path, &zpd_path)?;
    } else {
        std::fs::write(&target_file_path, &original_content)?;
        let _ = std::fs::remove_file(backup_path);
    }

    assert!(Path::new(memory_host_path).is_dir(), "Memory dir still valid after evolution");
    Ok(())
}

fn load_surprise_history(path: &Path) -> Vec<f64> {
    let mut history = Vec::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                if let Ok(val) = parts[1].trim().parse::<f64>() {
                    history.push(val);
                }
            }
        }
    }
    history
}

fn load_episodic_surprise(path: &Path) -> Vec<f64> {
    let mut surprises = Vec::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 6 {
                if let Ok(val) = parts[5].trim().parse::<f64>() {
                    surprises.push(val);
                }
            }
        }
    }
    surprises
}

fn parse_surprise_from_output(output: &str) -> Vec<f64> {
    let mut surprises = Vec::new();
    let start_marker = "--- TEST_SURPRISE_HISTORY_START ---";
    let end_marker = "--- TEST_SURPRISE_HISTORY_END ---";

    if let Some(start_pos) = output.find(start_marker) {
        if let Some(end_pos) = output.find(end_marker) {
            let content = &output[start_pos + start_marker.len()..end_pos];
            for line in content.lines() {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 2 {
                    if let Ok(val) = parts[1].trim().parse::<f64>() {
                        surprises.push(val);
                    }
                }
            }
        }
    }
    surprises
}
