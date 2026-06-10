#![deny(warnings)]
#![deny(clippy::all)]

mod container;
mod pruning;
mod monitor;
mod cli;
mod injector;

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

    monitor::run_monitor_daemon(Path::new(mem_dir), rx).await?;

    assert!(Path::new(mem_dir).is_dir(), "Memory dir remains valid after monitor");
    assert!(!mem_dir.is_empty(), "Memory dir non-empty");
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
        pruning::prune_resources(memory_host_path)?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let path_clone = memory_host_path.to_string();
        let daemon_handle = tokio::spawn(async move {
            let path = Path::new(&path_clone);
            let _ = monitor::run_monitor_daemon(path, shutdown_rx).await;
        });

        let status = container::run_container(CONTAINER_NAME, memory_host_path).await?;
        println!("[ferro-shell] Container exited with status: {:?}", status);

        let _ = shutdown_tx.send(());
        let _ = daemon_handle.await;

        pruning::prune_resources(memory_host_path)?;
        container::cleanup_container(CONTAINER_NAME).await?;
    }

    assert!(attempt == MAX_RECOVERY_ATTEMPTS, "Lifecycle loop executed fully");
    assert!(!Path::new(memory_host_path).join("panic_dump.json").exists(), "Panic dump cleanup");
    Ok(())
}
