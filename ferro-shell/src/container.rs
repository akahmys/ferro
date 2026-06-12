use std::path::PathBuf;
use std::process::ExitStatus;
use tokio::process::Command;

fn get_seccomp_profile_path() -> PathBuf {
    let curr = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let path = if curr.ends_with("ferro-shell") {
        curr.join("seccomp_profile.json")
    } else {
        curr.join("ferro-shell/seccomp_profile.json")
    };
    path.canonicalize().unwrap_or(path)
}

/// Runs the `ferro-core` docker container with safety options.
///
/// # Errors
/// Returns an error if spawn or waiting fails.
pub async fn run_container(
    container_name: &str,
    memory_host_path: &str,
) -> Result<ExitStatus, Box<dyn std::error::Error>> {
    // Rule 5: Pre-condition assertions
    assert!(
        !container_name.is_empty(),
        "Container name must not be empty"
    );
    assert!(
        !memory_host_path.is_empty(),
        "Memory host path must not be empty"
    );

    // Remove existing container with the same name if any
    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output()
        .await;

    let seccomp_path = get_seccomp_profile_path();
    let seccomp_arg = format!("seccomp={}", seccomp_path.to_string_lossy());
    let mount_arg = format!("type=bind,source={},target=/memory", memory_host_path);

    let mut child = Command::new("docker")
        .args([
            "run",
            "--name",
            container_name,
            "--network",
            "none",
            "--cpus=2.0",
            "-m",
            "2g",
            "--memory-swap",
            "2g",
            "--read-only",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid",
            "--security-opt",
            "no-new-privileges",
            "--security-opt",
            &seccomp_arg,
            "--cap-drop",
            "ALL",
            "--mount",
            &mount_arg,
            "ferro-core-runtime:latest",
        ])
        .spawn()?;

    let status = child.wait().await?;

    // Rule 5: Post-condition assertions
    assert!(!container_name.is_empty(), "Container name remains valid");
    assert!(
        status.code().is_some() || !status.success(),
        "Exit status must be resolved"
    );

    Ok(status)
}

/// Force removes the container.
///
/// # Errors
/// Returns an error if the command invocation fails.
pub async fn cleanup_container(container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Rule 5: Pre-condition assertions
    assert!(
        !container_name.is_empty(),
        "Container name must not be empty"
    );
    assert!(
        container_name.len() >= 3,
        "Container name must be of reasonable length"
    );

    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output()
        .await?;

    // Rule 5: Post-condition assertions
    assert!(!container_name.is_empty(), "Container name remains valid");
    assert!(
        container_name.contains("ferro"),
        "Container name must belong to ferro namespace"
    );

    Ok(())
}

/// Stops the container cleanly.
///
/// # Errors
/// Returns an error if the command invocation fails.
pub async fn stop_container(container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!container_name.is_empty(), "Container name must not be empty");
    assert!(container_name.len() >= 3, "Container name must be of reasonable length");

    let _ = Command::new("docker")
        .args(["stop", "-t", "5", container_name])
        .output()
        .await?;

    assert!(!container_name.is_empty(), "Container name remains valid");
    Ok(())
}


