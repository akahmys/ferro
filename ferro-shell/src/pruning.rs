use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const CLUSTERS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("clusters");

#[derive(Deserialize, Debug)]
pub struct PanicDump {
    pub origin_cluster_id: String,
    pub container_exit_code: Option<i32>,
    pub nociceptive_energy: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct PainHistoryRecord {
    pub timestamp: u64,
    pub origin_cluster_id: String,
    pub exit_code: i32,
    pub resolved_root_parent: String,
}

// Helper to resolve parent list iteratively (Power of 10 compliance - Rule 1 & 2)
fn resolve_pruning_targets(origin_id: &str, depth_limit: usize) -> Vec<String> {
    assert!(!origin_id.is_empty(), "Origin ID must not be empty");
    assert!(depth_limit > 0, "Depth limit must be positive");

    let mut targets = Vec::new();
    let mut current_id = origin_id.to_string();

    for _ in 0..10 {
        if targets.len() >= depth_limit {
            break;
        }
        targets.push(current_id.clone());

        if let Some(pos) = current_id.rfind("_child") {
            current_id = current_id[..pos].to_string();
            if current_id.is_empty() {
                break;
            }
        } else {
            break;
        }
    }

    assert!(!targets.is_empty(), "Must resolve at least one target node");
    targets
}

// Helper to extract the root parent ID
fn get_root_parent(origin_id: &str) -> String {
    let mut current_id = origin_id.to_string();
    for _ in 0..10 {
        if let Some(pos) = current_id.rfind("_child") {
            current_id = current_id[..pos].to_string();
            if current_id.is_empty() {
                break;
            }
        } else {
            break;
        }
    }
    current_id
}

/// Prunes resources based on the contents of `panic_dump.json`.
///
/// # Errors
/// Returns an error if the panic dump file cannot be read or parsed, or if file deletion fails.
pub async fn prune_resources(
    memory_dir: &str,
    exit_code: Option<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Rule 5: Assertions for pre-conditions
    assert!(
        !memory_dir.is_empty(),
        "Memory directory path must not be empty"
    );
    assert!(
        Path::new(memory_dir).is_dir(),
        "Memory directory must exist"
    );

    let dump_path = Path::new(memory_dir).join("panic_dump.json");
    let (origin_id, resolved_exit_code, is_seccomp_or_audit) = if dump_path.exists() {
        let dump_content = fs::read_to_string(&dump_path)?;
        let panic_dump: PanicDump = serde_json::from_str(&dump_content)?;
        let origin_id = panic_dump.origin_cluster_id;
        let dump_exit_code = panic_dump.container_exit_code;
        let dump_energy = panic_dump.nociceptive_energy.as_deref().unwrap_or("");

        let resolved_exit_code = exit_code.or(dump_exit_code).unwrap_or(0);
        let is_seccomp_or_audit = resolved_exit_code == 159 || dump_energy == "INFINITY";
        (origin_id, resolved_exit_code, is_seccomp_or_audit)
    } else {
        let resolved_exit_code = exit_code.unwrap_or(0);
        if resolved_exit_code != 0 {
            let action_text_path = Path::new(memory_dir).join("action").join("vocal_text.json");
            let vocal_text_path = Path::new(memory_dir).join("vocal_text.json");
            let mut resolved_id = None;

            for path in &[action_text_path, vocal_text_path] {
                if path.exists() {
                    if let Ok(c) = fs::read_to_string(path) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&c) {
                            if let Some(id_str) = val.get("origin_cluster_id").and_then(|v| v.as_str()) {
                                resolved_id = Some(id_str.to_string());
                                break;
                            }
                        }
                    }
                }
            }

            let origin_id = if let Some(id) = resolved_id {
                id
            } else {
                let hippo_path = Path::new(memory_dir).join("episodic_buffer.csv");
                if hippo_path.exists() {
                    let mut last_id = None;
                    if let Ok(file) = fs::File::open(&hippo_path) {
                        let mut rdr = csv::Reader::from_reader(file);
                        for rec in rdr.deserialize::<serde_json::Value>().flatten() {
                            if let Some(id_str) = rec.get("target_cluster_id").and_then(|v| v.as_str()) {
                                last_id = Some(id_str.to_string());
                            }
                        }
                    }
                    last_id.unwrap_or_else(|| "cortex_unknown".to_string())
                } else {
                    "cortex_unknown".to_string()
                }
            };

            let is_seccomp_or_audit = resolved_exit_code == 159;
            (origin_id, resolved_exit_code, is_seccomp_or_audit)
        } else {
            return Ok(());
        }
    };
    let d_initial = if is_seccomp_or_audit { 10 } else { 1 };

    // 2. Read history to find N_recur
    let root_parent = get_root_parent(&origin_id);
    let history_path = Path::new(memory_dir).join("pain_history.csv");
    let mut history_records = Vec::new();
    let mut n_recur = 0;

    if history_path.exists() {
        let file = fs::File::open(&history_path)?;
        let mut rdr = csv::Reader::from_reader(file);
        for record in rdr.deserialize().flatten() {
            let rec: PainHistoryRecord = record;
            if rec.resolved_root_parent == root_parent {
                n_recur += 1;
            }
            history_records.push(rec);
        }
    }

    // 3. Resolve target depth and target list
    let final_depth = std::cmp::max(d_initial, n_recur + 1);
    let targets = resolve_pruning_targets(&origin_id, final_depth);
    let final_root_parent = targets.last().cloned().unwrap_or_else(|| root_parent.clone());

    // 4. Update history with atomic write (Single-Writer rule & atomic write)
    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let new_record = PainHistoryRecord {
        timestamp: current_time,
        origin_cluster_id: origin_id.clone(),
        exit_code: resolved_exit_code,
        resolved_root_parent: final_root_parent,
    };
    history_records.push(new_record);

    let temp_history_path = Path::new(memory_dir).join("pain_history.csv.tmp");
    {
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_history_path)?;
        let mut wtr = csv::Writer::from_writer(file);
        for rec in &history_records {
            wtr.serialize(rec)?;
        }
        wtr.flush()?;
    }
    fs::rename(&temp_history_path, &history_path)?;

    // 5. Delete JSON Sharded files
    let cluster_dir = Path::new(memory_dir)
        .join("knowledge_graph")
        .join("clusters");
    for target in &targets {
        if target.len() >= 2 {
            let shard_id = &target[0..2];
            let shard_dir = cluster_dir.join(shard_id);
            let file_path = shard_dir.join(format!("{}.json", target));
            if file_path.exists() {
                fs::remove_file(&file_path)?;
                println!("[ferro-shell] Pruned JSON file: {:?}", file_path);
            }
        }
    }

    // 6. Delete redb records with retry loop
    let db_path = Path::new(memory_dir).join("storage.redb");
    if db_path.exists() {
        let mut db_opened = false;
        let mut attempts = 0;
        while !db_opened && attempts < 5 {
            attempts += 1;
            match redb::Database::create(&db_path) {
                Ok(db) => {
                    db_opened = true;
                    match db.begin_write() {
                        Ok(write_txn) => {
                            if let Ok(mut table) = write_txn.open_table(CLUSTERS_TABLE) {
                                for target in &targets {
                                    if let Ok(Some(_)) = table.remove(target.as_str()) {
                                        println!("[ferro-shell] Pruned redb record: {}", target);
                                    }
                                }
                            }
                            let _ = write_txn.commit();
                        }
                        Err(e) => {
                            eprintln!("[ferro-shell] Failed to begin write transaction: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ferro-shell] Attempt {} to open redb failed: {:?}", attempts, e);
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }
    }

    // Prune simulated actor files
    let vocal_text_path = Path::new(memory_dir).join("vocal_text.json");
    if vocal_text_path.exists() {
        fs::remove_file(&vocal_text_path)?;
    }

    // Clean up the panic dump itself
    if dump_path.exists() {
        fs::remove_file(&dump_path)?;
    }

    // Rule 5: Assertions for post-conditions
    assert!(
        !dump_path.exists(),
        "panic_dump.json must be deleted after pruning"
    );
    assert!(
        Path::new(memory_dir).is_dir(),
        "Memory directory must remain intact"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_pruning_targets() {
        let targets = resolve_pruning_targets("c0_child_child", 1);
        assert_eq!(targets, vec!["c0_child_child".to_string()]);

        let targets = resolve_pruning_targets("c0_child_child", 2);
        assert_eq!(targets, vec!["c0_child_child".to_string(), "c0_child".to_string()]);

        let targets = resolve_pruning_targets("c0_child_child", 3);
        assert_eq!(targets, vec!["c0_child_child".to_string(), "c0_child".to_string(), "c0".to_string()]);

        let targets = resolve_pruning_targets("c0_child_child", 10);
        assert_eq!(targets, vec!["c0_child_child".to_string(), "c0_child".to_string(), "c0".to_string()]);
    }

    #[tokio::test]
    async fn test_prune_resources_oom() {
        let temp_path = std::env::temp_dir().join(format!(
            "ferro_test_{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        fs::create_dir_all(&temp_path).unwrap();

        let cluster_dir = temp_path.join("knowledge_graph").join("clusters");
        let c0_shard = cluster_dir.join("c0");
        fs::create_dir_all(&c0_shard).unwrap();

        let child_file = c0_shard.join("c0_child.json");
        fs::write(&child_file, b"{}").unwrap();

        let dump_path = temp_path.join("panic_dump.json");
        let dump_data = serde_json::json!({
            "origin_cluster_id": "c0_child",
            "container_exit_code": 137,
            "nociceptive_energy": "1.0"
        });
        fs::write(&dump_path, serde_json::to_string(&dump_data).unwrap()).unwrap();

        prune_resources(&temp_path.to_string_lossy(), Some(137)).await.unwrap();

        assert!(!child_file.exists());
        assert!(!dump_path.exists());

        let history_path = temp_path.join("pain_history.csv");
        assert!(history_path.exists());
        let history_content = fs::read_to_string(&history_path).unwrap();
        assert!(history_content.contains("c0_child"));
        assert!(history_content.contains("137"));

        let _ = fs::remove_dir_all(&temp_path);
    }
}


