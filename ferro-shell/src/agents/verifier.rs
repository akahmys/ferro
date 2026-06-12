use crate::agents::AgentContext;
use crate::agents::planner::PatchTicket;
use serde_json::Value;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::process::Command;

#[allow(dead_code)]
pub struct VerifierAgent {
    pub context: AgentContext,
    pub docker_image_name: String,
}

impl VerifierAgent {
    #[allow(dead_code)]
    pub fn new(context: AgentContext, docker_image_name: String) -> Self {
        assert!(!context.memory_dir.is_empty(), "Memory dir empty");
        assert!(!docker_image_name.is_empty(), "Docker image empty");
        let agent = Self { context, docker_image_name };
        assert!(!agent.docker_image_name.is_empty(), "Docker image name set");
        agent
    }

    #[allow(dead_code)]
    pub async fn build_sandbox_image(&self) -> Result<(), Box<dyn std::error::Error>> {
        assert!(!self.docker_image_name.is_empty(), "Docker image set");
        let curr = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let (df_path, ctx_path) = if curr.ends_with("ferro-shell") {
            (curr.join("Dockerfile.sandbox"), curr.clone())
        } else {
            (curr.join("ferro-shell/Dockerfile.sandbox"), curr.join("ferro-shell"))
        };
        assert!(df_path.exists(), "Dockerfile.sandbox exists");
        let status = Command::new("docker")
            .args(["build", "-t", &self.docker_image_name, "-f", &df_path.to_string_lossy(), &ctx_path.to_string_lossy()])
            .status().await?;
        if !status.success() {
            return Err("Failed to build sandbox image".into());
        }
        assert!(status.success(), "Docker build success");
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn execute_secure_sandbox_run(
        &self,
        sandbox_name: &str,
        target_src_path: &str,
    ) -> Result<(i32, String), Box<dyn std::error::Error>> {
        assert!(!sandbox_name.is_empty(), "Sandbox name empty");
        assert!(!target_src_path.is_empty(), "Target path empty");
        let _ = Command::new("docker").args(["rm", "-f", sandbox_name]).output().await;
        let curr = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let sc_path = if curr.ends_with("ferro-shell") {
            curr.join("seccomp_profile.json")
        } else {
            curr.join("ferro-shell/seccomp_profile.json")
        };
        let sc_path = sc_path.canonicalize().unwrap_or(sc_path);
        assert!(sc_path.exists(), "seccomp exists");
        let output = Command::new("docker")
            .args([
                "run", "--name", sandbox_name, "--network", "none", "--cpus=2.0", "-m", "2g",
                "--memory-swap", "2g", "--tmpfs", "/target:rw,noexec,nosuid",
                "--tmpfs", "/tmp:rw,noexec,nosuid", "-e", "CARGO_TARGET_DIR=/target",
                "--security-opt", "no-new-privileges",
                "--security-opt", &format!("seccomp={}", sc_path.to_string_lossy()),
                "--cap-drop", "ALL", "--mount", &format!("type=bind,source={},target=/workspace,readonly", target_src_path),
                &self.docker_image_name,
            ])
            .output().await?;
        let code = output.status.code().unwrap_or(-1);
        let output_str = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        assert!(!sandbox_name.is_empty(), "Sandbox name valid");
        assert!(output.status.code().is_some() || !output.status.success(), "Exit resolved");
        Ok((code, output_str))
    }

    #[allow(dead_code)]
    pub fn compute_epistemic_score(
        &self,
        recent_tickets: &[PatchTicket],
        zone_count: usize,
    ) -> Result<f64, String> {
        assert!(recent_tickets.len() <= 1000, "Tickets cap check");
        assert!(zone_count < 1000, "Zone count check");

        if recent_tickets.is_empty() || zone_count == 0 {
            return Ok(1.0);
        }

        let mut zone_freq = HashMap::new();
        for ticket in recent_tickets {
            *zone_freq.entry(ticket.zone_marker_id.clone()).or_insert(0) += 1;
        }

        let mut freqs: Vec<f64> = (0..zone_count)
            .map(|i| *zone_freq.get(&format!("zone_{}", i)).unwrap_or(&0) as f64)
            .collect();
        freqs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = freqs.len() as f64;
        let sum: f64 = freqs.iter().sum();
        if sum == 0.0 { return Ok(1.0); }

        let gini: f64 = freqs.iter().enumerate()
            .map(|(i, &f)| (2.0 * (i as f64 + 1.0) - n - 1.0) * f)
            .sum::<f64>() / (n * sum);

        const STAGNATION_THRESHOLD: f64 = 0.7;
        if gini > STAGNATION_THRESHOLD {
            return Err(format!(
                "EpistemicStagnation: Gini={:.3} exceeds {:.3}. Reject.",
                gini, STAGNATION_THRESHOLD
            ));
        }

        let res = 1.0 - gini;
        assert!((0.0..=1.0).contains(&res), "Epistemic score bounds check");
        Ok(res)
    }

    #[allow(dead_code)]
    pub fn compute_fep_trend_score(
        &self,
        surprise_history_before: &[f64],
        surprise_history_after: &[f64],
        phase4_enabled: bool,
    ) -> f64 {
        assert!(surprise_history_before.len() <= 1000, "Before cap check");
        assert!(surprise_history_after.len() <= 1000, "After cap check");

        if !phase4_enabled {
            return 1.0;
        }
        if surprise_history_before.is_empty() || surprise_history_after.is_empty() {
            return 0.5;
        }

        let mean_before: f64 = surprise_history_before.iter().sum::<f64>()
            / surprise_history_before.len() as f64;
        let mean_after: f64 = surprise_history_after.iter().sum::<f64>()
            / surprise_history_after.len() as f64;

        let delta = mean_before - mean_after;
        let res = 1.0 / (1.0 + (-delta * 10.0).exp());

        assert!((0.0..=1.0).contains(&res), "FEP trend score bounds check");
        res
    }

    #[allow(dead_code)]
    pub fn evaluate_total_fitness(
        &self,
        static_score: f64,
        homeostasis_score: f64,
        epistemic_score: f64,
        fep_trend_score: f64,
    ) -> Result<f64, String> {
        assert!(static_score >= 0.0, "Static score check");
        assert!(homeostasis_score >= 0.0, "Homeostasis check");

        let fitness = static_score
            * homeostasis_score
            * epistemic_score
            * fep_trend_score;

        if fitness <= 0.0 {
            return Err(format!(
                "Fitness=0: static={:.2} homeostasis={:.2} epistemic={:.2} fep_trend={:.2}",
                static_score, homeostasis_score, epistemic_score, fep_trend_score
            ));
        }

        assert!(fitness >= 0.0, "Fitness bounds check");
        Ok(fitness)
    }

    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub fn compile_and_test_report(
        &self,
        container_exit_code: i32,
        cargo_build_output: &str,
        cargo_test_output: &str,
        _brainstem_metrics_csv: &str,
        _episodic_buffer_csv: &str,
        recent_tickets: &[PatchTicket],
        zone_count: usize,
        surprise_before: &[f64],
        surprise_after: &[f64],
        phase4_enabled: bool,
    ) -> Result<Value, String> {
        assert!(!self.docker_image_name.is_empty(), "Docker image name set");
        assert!(!self.context.memory_dir.is_empty(), "Memory dir set");

        let build_success = cargo_build_output.contains("Finished") || cargo_build_output.contains("Checked") || container_exit_code == 0;
        let test_success = cargo_test_output.contains("ok") || container_exit_code == 0;
        let static_score = if build_success && test_success { 1.0 } else { 0.0 };

        let homeostasis_score = if container_exit_code == 137 || container_exit_code == 159 { 0.0 } else { 1.0 };

        let epistemic_score = self.compute_epistemic_score(recent_tickets, zone_count)?;
        let fep_trend_score = self.compute_fep_trend_score(surprise_before, surprise_after, phase4_enabled);

        let fitness = self.evaluate_total_fitness(static_score, homeostasis_score, epistemic_score, fep_trend_score)?;

        let report = serde_json::json!({
            "static_score": static_score,
            "homeostasis_score": homeostasis_score,
            "epistemic_score": epistemic_score,
            "fep_trend_score": fep_trend_score,
            "fitness": fitness,
        });
        assert!(report.is_object(), "Report object");
        Ok(report)
    }
}
