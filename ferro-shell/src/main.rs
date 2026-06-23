use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time;

use ferro_shell::pruner::Pruner;

fn get_binary_path(name: &str) -> PathBuf {
    let workspace_root = env::var("FERRO_WORKSPACE_ROOT")
        .unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(workspace_root).join("target").join("debug").join(name);
    assert!(path.exists() || PathBuf::from(name).exists(), "Error: binary must exist");
    path
}

fn spawn_child(binary: &Path, memory_dir: &Path) -> Result<std::process::Child, String> {
    assert!(binary.exists(), "Error: child binary path must exist");
    let child = Command::new(binary)
        .env("FERRO_MEMORY_DIR", memory_dir.to_string_lossy().to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", binary.display(), e))?;
    Ok(child)
}

fn handle_panic_and_pruning(memory_dir: &Path) -> Result<(), String> {
    assert!(memory_dir.exists(), "Error: memory directory must exist");
    let pruner = Pruner::new(memory_dir.to_path_buf());
    
    // 構造剪定を実行 (panic_dump.json があれば breeding_signals.json が作られる)
    pruner.perform_pruning()?;
    Ok(())
}

async fn run_governance_loop(
    core_bin: &Path,
    body_bin: &Path,
    memory_dir: &Path,
) -> Result<(), String> {
    let mut core_child = spawn_child(core_bin, memory_dir)?;
    let mut body_child = spawn_child(body_bin, memory_dir)?;

    let mut interval = time::interval(Duration::from_millis(200));
    let mut loop_count = 0;
    
    // 静的上限: 1000 サイクル (約200秒) でクリーン終了またはテスト用タイムアウト
    while loop_count < 1000 {
        loop_count += 1;
        let _ = interval.tick().await;

        let core_status = core_child.try_wait().map_err(|e| e.to_string())?;
        let body_status = body_child.try_wait().map_err(|e| e.to_string())?;

        let panic_dump_exists = memory_dir.join("panic_dump.json").exists();
        let child_died = core_status.is_some() || body_status.is_some();

        if child_died || panic_dump_exists {
            println!("Outer governance detected anomaly. PanicDump: {}, ChildDied: {}", panic_dump_exists, child_died);
            
            // 子プロセスをクリーン終了
            let _ = core_child.kill();
            let _ = body_child.kill();

            // 剪定の実行
            handle_panic_and_pruning(memory_dir)?;

            // 1秒待って再起動
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Rebuilding and restarting FERRO cluster...");
            core_child = spawn_child(core_bin, memory_dir)?;
            body_child = spawn_child(body_bin, memory_dir)?;
        }
    }
    
    let _ = core_child.kill();
    let _ = body_child.kill();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_str = env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    let memory_dir = PathBuf::from(dir_str);
    if !memory_dir.exists() {
        let _ = std::fs::create_dir_all(&memory_dir);
    }

    let core_bin = get_binary_path("ferro-core");
    let body_bin = get_binary_path("ferro-body");

    println!("FERRO Hypervisor starting up...");
    println!("Core path: {}, Body path: {}", core_bin.display(), body_bin.display());

    run_governance_loop(&core_bin, &body_bin, &memory_dir).await?;

    println!("FERRO Hypervisor exiting.");
    Ok(())
}
