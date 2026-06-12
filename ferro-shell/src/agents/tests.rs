use crate::agents::{AgentContext, supervisor::SupervisorAgent, planner::PlannerAgent, executor::ExecutorAgent, verifier::VerifierAgent};

#[test]
fn test_agent_context_creation() {
    let context = AgentContext::new("/tmp".to_string(), vec!["ZONE_1".to_string()]);
    assert!(!context.memory_dir.is_empty(), "Pre-condition check");
    assert_eq!(context.memory_dir, "/tmp");
    assert_eq!(context.active_zone_markers.len(), 1);
}

#[test]
fn test_supervisor_agent() {
    let context = AgentContext::new("/tmp".to_string(), vec!["ZONE_1".to_string()]);
    assert!(!context.memory_dir.is_empty(), "Pre-condition check");
    let supervisor = SupervisorAgent::new(context);
    let res = supervisor.analyze_cortex_bottlenecks("surprise_history.csv");
    assert!(res.is_ok());
    assert!(res.unwrap().contains("/tmp"));
}

#[test]
fn test_planner_agent() {
    let context = AgentContext::new("/tmp".to_string(), vec![]);
    assert!(!context.memory_dir.is_empty(), "Pre-condition check");
    let planner = PlannerAgent::new(context);
    let ticket_res = planner.generate_patch_ticket("roadmap", "graph_json");
    assert!(ticket_res.is_ok());
    let ticket = ticket_res.unwrap();
    assert!(ticket.ticket_id.starts_with("ticket_"));
}

#[test]
fn test_executor_agent() {
    let context = AgentContext::new("/tmp".to_string(), vec![]);
    assert!(!context.memory_dir.is_empty(), "Pre-condition check");
    let executor = ExecutorAgent::new(context);
    let ticket = crate::agents::planner::PatchTicket {
        ticket_id: "ticket_123".to_string(),
        file_path: "src/cortex/mod.rs".to_string(),
        zone_marker_id: "ZONE_1".to_string(),
        replacement_ast_code: "fn test() {}".to_string(),
    };
    let input = "before\n// == FERRO_ADAPTIVE_ZONE_START\nold code\n// == FERRO_ADAPTIVE_ZONE_END\nafter";
    let res = executor.apply_patch_to_file(input, &ticket);
    assert!(res.is_ok());
    let output = res.unwrap();
    assert!(output.contains("fn test() {}"));
    assert!(output.contains("before"));
    assert!(output.contains("after"));
}

#[test]
fn test_verifier_agent() {
    let context = AgentContext::new("/tmp".to_string(), vec![]);
    assert!(!context.memory_dir.is_empty(), "Pre-condition check");
    let verifier = VerifierAgent::new(context, "ferro-sandbox:latest".to_string());
    let report_res = verifier.compile_and_test_report(
        0, "Finished build", "test ok", "metrics", "buf",
        &[], 0, &[], &[], false
    );
    assert!(report_res.is_ok());
    let report = report_res.unwrap();
    assert_eq!(report["static_score"], 1.0);
    assert_eq!(report["homeostasis_score"], 1.0);
    assert_eq!(report["epistemic_score"], 1.0);
    assert_eq!(report["fep_trend_score"], 1.0);
    assert_eq!(report["fitness"], 1.0);
}

#[tokio::test]
async fn test_evolution_cycle() {
    let old_dir = std::env::current_dir().unwrap();
    let rand_id = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos()).unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("ferro_test_evo_{}", rand_id));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let original_shell_dir = if old_dir.ends_with("ferro-shell") {
        old_dir.clone()
    } else {
        old_dir.join("ferro-shell")
    };

    let mock_shell_dir = temp_dir.join("ferro-shell");
    std::fs::create_dir_all(&mock_shell_dir).unwrap();
    std::fs::copy(original_shell_dir.join("Dockerfile.sandbox"), mock_shell_dir.join("Dockerfile.sandbox")).unwrap();
    std::fs::copy(original_shell_dir.join("seccomp_profile.json"), mock_shell_dir.join("seccomp_profile.json")).unwrap();

    let surprise_history_path = temp_dir.join("surprise_history.csv");
    std::fs::write(&surprise_history_path, "timestamp,global_free_energy,phase\n1620000000,0.5,Sleep\n").unwrap();

    let core_dir = temp_dir.join("ferro-core");
    let cortex_dir = core_dir.join("src/cortex");
    std::fs::create_dir_all(&cortex_dir).unwrap();

    let cargo_toml_path = core_dir.join("Cargo.toml");
    std::fs::write(&cargo_toml_path, "[package]\nname = \"ferro-core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[dependencies]\n").unwrap();

    let main_rs_path = core_dir.join("src/main.rs");
    std::fs::write(&main_rs_path, "fn main() {}\n").unwrap();

    let cargo_lock_path = core_dir.join("Cargo.lock");
    std::fs::write(&cargo_lock_path, "").unwrap();

    let target_file = cortex_dir.join("mod.rs");
    std::fs::write(&target_file, "before\n// == FERRO_ADAPTIVE_ZONE_START\nold code\n// == FERRO_ADAPTIVE_ZONE_END\nafter").unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();

    let res = crate::evolution::run_evolution_cycle(&temp_dir.to_string_lossy()).await;
    assert!(res.is_ok());

    std::env::set_current_dir(&old_dir).unwrap();

    assert!(target_file.exists(), "Target file must still exist after rollback");
    let content = std::fs::read_to_string(&target_file).unwrap();
    assert!(content.contains("old code"), "Content must be rolled back to original");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
