#![deny(warnings)]
#![deny(clippy::all)]

mod container;
mod pruning;
mod monitor;
mod cli;
mod injector;
mod agents;
mod evolution;

use std::path::Path;
use tokio::sync::oneshot;

const MAX_RECOVERY_ATTEMPTS: usize = 5;
const CONTAINER_NAME: &str = "ferro-core-runtime";

fn get_memory_host_path() -> String {
    std::env::var("FERRO_MEMORY_PATH").unwrap_or_else(|_| {
        let curr = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let path = if curr.ends_with("ferro-shell") {
            curr.parent().unwrap().join("ferro-core/memory")
        } else {
            curr.join("ferro-core/memory")
        };
        path.canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::parse_args();
    assert!(args.surprise.is_none() || args.surprise.unwrap() >= 0.0, "Surprise arg assertion");
    assert!(CONTAINER_NAME.starts_with("ferro"), "Container namespace assertion");

    let memory_host_path = get_memory_host_path();

    if let Some(mode) = args.mode {
        match mode.as_str() {
            "monitor-debug" => {
                let mem_dir = args.memory_dir.unwrap_or_else(|| memory_host_path.clone());
                run_monitor_debug(&mem_dir).await?;
            }
            "inject-mock-episode" => {
                let mem_dir = args.memory_dir.unwrap_or_else(|| memory_host_path.clone());
                let surprise = args.surprise.unwrap_or(0.92);
                injector::inject_mock(&mem_dir, surprise)?;
            }
            _ => eprintln!("Unknown mode: {}", mode),
        }
    } else {
        run_lifecycle_loop(&memory_host_path).await?;
    }

    assert!(!CONTAINER_NAME.is_empty(), "Container name check on exit");
    assert!(!memory_host_path.is_empty(), "Memory path check on exit");
    Ok(())
}

async fn run_monitor_debug(mem_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!mem_dir.is_empty(), "Memory dir must not be empty");
    assert!(Path::new(mem_dir).is_dir(), "Memory dir must exist");

    println!("[ferro-shell] Starting monitor-debug mode on {}", mem_dir);
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        let _ = tx.send(());
    });

    monitor::run_monitor_daemon(Path::new(mem_dir), rx, None).await?;

    assert!(Path::new(mem_dir).is_dir(), "Memory dir remains valid after monitor");
    assert!(!mem_dir.is_empty(), "Memory dir non-empty");
    Ok(())
}

async fn rebuild_core_runtime() -> Result<(), Box<dyn std::error::Error>> {
    let curr = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let (dockerfile_path, context_path) = if curr.ends_with("ferro-shell") {
        (curr.join("Dockerfile.core"), curr.parent().unwrap().to_path_buf())
    } else {
        (curr.join("ferro-shell/Dockerfile.core"), curr.clone())
    };

    println!("[ferro-shell] Rebuilding ferro-core-runtime using Dockerfile {:?}", dockerfile_path);
    let status = tokio::process::Command::new("docker")
        .args([
            "build",
            "-f",
            &dockerfile_path.to_string_lossy(),
            "-t",
            "ferro-core-runtime:latest",
            &context_path.to_string_lossy(),
        ])
        .status()
        .await?;

    if !status.success() {
        return Err("Failed to build ferro-core-runtime image".into());
    }
    Ok(())
}

async fn run_lifecycle_loop(memory_host_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let max_rec = MAX_RECOVERY_ATTEMPTS;
    assert!(max_rec > 0, "Max recovery attempts must be positive");
    assert!(!memory_host_path.is_empty(), "Memory host path must not be empty");

    let mut attempt = 0;
    while attempt < MAX_RECOVERY_ATTEMPTS {
        attempt += 1;
        println!("[ferro-shell] Start cycle {}/{}...", attempt, MAX_RECOVERY_ATTEMPTS);
        
        pruning::prune_resources(memory_host_path, None).await?;

        if let Err(e) = rebuild_core_runtime().await {
            eprintln!("[ferro-shell] Error rebuilding runtime: {:?}", e);
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (sleep_tx, sleep_rx) = oneshot::channel();
        let path_clone = memory_host_path.to_string();
        let daemon_handle = tokio::spawn(async move {
            let path = Path::new(&path_clone);
            let _ = monitor::run_monitor_daemon(path, shutdown_rx, Some(sleep_tx)).await;
        });

        let path_clone_2 = memory_host_path.to_string();
        let mut container_join = tokio::spawn(async move {
            container::run_container(CONTAINER_NAME, &path_clone_2).await.map_err(|e| e.to_string())
        });

        let mut sleep_triggered = false;
        let exit_status = tokio::select! {
            res = &mut container_join => {
                match res {
                    Ok(Ok(status)) => {
                        println!("[ferro-shell] Container exited naturally: {:?}", status);
                        Some(status)
                    }
                    _ => {
                        println!("[ferro-shell] Container execution failed");
                        None
                    }
                }
            }
            _ = sleep_rx => {
                println!("[ferro-shell] Sleep phase transition detected. Stopping container...");
                sleep_triggered = true;
                let _ = container::stop_container(CONTAINER_NAME).await;
                match container_join.await {
                    Ok(Ok(status)) => Some(status),
                    _ => None
                }
            }
        };

        let _ = shutdown_tx.send(());
        let _ = daemon_handle.await;

        let exit_code = exit_status.and_then(|s| s.code());
        container::cleanup_container(CONTAINER_NAME).await?;

        if sleep_triggered {
            println!("[ferro-shell] Starting evolutionary adaptation loop...");
            if let Err(e) = evolution::run_evolution_cycle(memory_host_path).await {
                eprintln!("[ferro-shell] Evolutionary cycle error: {:?}", e);
            }
        } else {
            pruning::prune_resources(memory_host_path, exit_code).await?;
        }
    }

    assert!(attempt == MAX_RECOVERY_ATTEMPTS, "Lifecycle loop executed fully");
    assert!(!Path::new(memory_host_path).join("panic_dump.json").exists(), "Panic dump cleanup");
    Ok(())
}
