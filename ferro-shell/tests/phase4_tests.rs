use std::fs;
use ferro_core::storage::Storage;
use ferro_shell::pruner::{Pruner, PanicDump, BreedingSignals};

#[test]
fn test_structural_pruning_flow() -> Result<(), String> {
    let test_dir = std::env::temp_dir().join("ferro_test_phase4");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);

    // 1. Storage に親子関係を格納
    let storage = Storage::new(test_dir.clone(), 5000);
    storage.put("link:parent_1->child_1".to_string(), "0.5".to_string())?;
    storage.put("link:parent_2->parent_1".to_string(), "0.6".to_string())?;
    storage.put("link:unrelated_parent->unrelated_child".to_string(), "0.1".to_string())?;

    // 2. panic_dump.json を書き出し
    let dump = PanicDump {
        origin_cluster_id: "child_1".to_string(),
        violation_type: "EthicalAudit".to_string(),
    };
    let dump_path = test_dir.join("panic_dump.json");
    let dump_str = serde_json::to_string(&dump).map_err(|e| e.to_string())?;
    fs::write(&dump_path, dump_str).map_err(|e| e.to_string())?;

    // 3. Pruner による剪定実行
    let pruner = Pruner::new(test_dir.clone());
    pruner.perform_pruning()?;

    // 4. breeding_signals.json が生成されたか確認
    let breeding_path = test_dir.join("breeding_signals.json");
    assert!(breeding_path.exists());
    let breeding_content = fs::read_to_string(&breeding_path).map_err(|e| e.to_string())?;
    let signals: BreedingSignals = serde_json::from_str(&breeding_content).map_err(|e| e.to_string())?;

    // 遡及深度制限 (EthicalAudit は D=10) により、child_1 -> parent_1 -> parent_2 が全て剪定対象に含まれるべき
    assert!(signals.prune_cluster_ids.contains(&"child_1".to_string()));
    assert!(signals.prune_cluster_ids.contains(&"parent_1".to_string()));
    assert!(signals.prune_cluster_ids.contains(&"parent_2".to_string()));
    // unrelated は child_1 の祖先ではないので含まれないはず
    assert!(!signals.prune_cluster_ids.contains(&"unrelated_parent".to_string()));
    assert!(!signals.prune_cluster_ids.contains(&"unrelated_child".to_string()));

    // 5. Pruning Hook シミュレーション
    let entries = storage.get_all_entries()?;
    for (k, _) in entries {
        for id in &signals.prune_cluster_ids {
            let matches = k == format!("actor:{}", id)
                || k.starts_with(&format!("link:{}->", id))
                || k.ends_with(&format!("->{}", id))
                || k.contains(&format!(":{}->", id))
                || k.contains(&format!("->{}", id))
                || k == *id;
            if matches {
                let _ = storage.remove(&k);
            }
        }
    }

    // 削除確認
    assert!(storage.get("link:parent_1->child_1")?.is_none());
    assert!(storage.get("link:parent_2->parent_1")?.is_none());
    // unrelated は削除されていないはず
    assert!(storage.get("link:unrelated_parent->unrelated_child")?.is_some());

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}
