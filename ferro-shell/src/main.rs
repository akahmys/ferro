#![deny(warnings)]
#![deny(clippy::all)]

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time;

use ferro_shell::pruner::Pruner;

fn get_binary_path(name: &str) -> PathBuf {
    assert!(!name.is_empty(), "Error: binary name must not be empty");
    assert!(name.len() < 100, "Error: binary name too long");
    let workspace_root = env::var("FERRO_WORKSPACE_ROOT")
        .unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(workspace_root).join("target").join("debug").join(name);
    assert!(path.exists() || PathBuf::from(name).exists(), "Error: binary must exist");
    path
}

fn spawn_child(name: &str, binary: &Path, memory_dir: &Path) -> Result<std::process::Child, String> {
    assert!(binary.exists(), "Error: child binary path must exist");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory directory must not be empty");

    let use_docker = std::env::var("FERRO_USE_DOCKER").unwrap_or_default() == "1";
    
    if use_docker {
        let memory_dir_str = memory_dir.to_string_lossy().to_string();
        
        let mut args = vec![
            "run".to_string(),
            "--name".to_string(),
            format!("ferro-{}", name),
            "--rm".to_string(),
            "--network".to_string(),
            "none".to_string(),
            "-v".to_string(),
            format!("{}:/memory", memory_dir_str),
            "-e".to_string(),
            "FERRO_MEMORY_DIR=/memory".to_string(),
        ];

        let seccomp_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("seccomp_profile.json");
        if seccomp_path.exists() {
            args.push("--security-opt".to_string());
            args.push(format!("seccomp={}", seccomp_path.to_string_lossy()));
        }

        let is_prod = std::env::var("FERRO_ENV").unwrap_or_default() == "production";
        if is_prod {
            args.push("--cap-add".to_string());
            args.push("IPC_LOCK".to_string());
        }

        let docker_image = std::env::var("FERRO_DOCKER_IMAGE").unwrap_or_else(|_| "ferro-sandbox".to_string());
        args.push(docker_image);
        args.push(format!("/usr/local/bin/{}", name));

        println!("Spawning child in Docker container: docker {}", args.join(" "));

        let child = Command::new("docker")
            .args(&args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn {} inside Docker: {}", name, e))?;

        assert!(child.id() > 0, "Error: spawned Docker child process must have a positive ID");
        Ok(child)
    } else {
        let child = Command::new(binary)
            .env("FERRO_MEMORY_DIR", memory_dir.to_string_lossy().to_string())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", binary.display(), e))?;
        
        assert!(child.id() > 0, "Error: spawned child process must have a positive ID");
        Ok(child)
    }
}

fn handle_panic_and_pruning(memory_dir: &Path) -> Result<(), String> {
    assert!(memory_dir.exists(), "Error: memory directory must exist");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory directory must not be empty");
    let pruner = Pruner::new(memory_dir.to_path_buf());
    
    // 構造剪定を実行 (panic_dump.json があれば breeding_signals.json が作られる)
    pruner.perform_pruning()?;
    
    assert!(!memory_dir.join("panic_dump.json").exists(), "Error: post-condition panic dump file should be removed");
    Ok(())
}

async fn run_governance_loop(
    core_bin: &Path,
    body_bin: &Path,
    memory_dir: &Path,
) -> Result<(), String> {
    assert!(core_bin.exists(), "Error: core binary must exist");
    assert!(body_bin.exists(), "Error: body binary must exist");
    assert!(memory_dir.exists(), "Error: memory directory must exist");

    let mut core_child = spawn_child("ferro-core", core_bin, memory_dir)?;
    let mut body_child = spawn_child("ferro-body", body_bin, memory_dir)?;

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
            
            // 子プロセスをクリーン終了 (DBロックの解放)
            let _ = core_child.kill();
            let _ = body_child.kill();

            // 100ms 待ってOSレベルで確実にロックを解放させる
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Verifier による事後形式検証 (Lipschitz 境界条件のチェック)
            let verifier = ferro_shell::verifier::Verifier::new(memory_dir.to_path_buf());
            if let Err(ref e) = verifier.verify_safety_contracts() {
                println!("Verifier post-mortem warning: {}", e);
            }

            // 剪定の実行
            handle_panic_and_pruning(memory_dir)?;

            // 1秒待って再起動
            tokio::time::sleep(Duration::from_secs(1)).await;
            println!("Rebuilding and restarting FERRO cluster...");
            core_child = spawn_child("ferro-core", core_bin, memory_dir)?;
            body_child = spawn_child("ferro-body", body_bin, memory_dir)?;
        }
    }
    
    let _ = core_child.kill();
    let _ = body_child.kill();

    assert!(loop_count <= 1000, "Error: loop count limit exceeded");
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
