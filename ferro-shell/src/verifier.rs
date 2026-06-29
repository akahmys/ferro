use std::path::PathBuf;
use std::collections::HashMap;
use ferro_core::storage::Storage;
use crate::pruner::PanicDump;

pub struct Verifier {
    memory_dir: PathBuf,
}

impl Verifier {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        assert!(memory_dir.exists() || memory_dir.starts_with(".") || memory_dir.starts_with("/tmp"), "Error: path must be valid or exist");
        let verifier = Self { memory_dir };
        assert!(!verifier.memory_dir.as_os_str().is_empty(), "Error: post-condition path empty check");
        verifier
    }

    /// 形式検証を実行。Storage を ReadOnly でスキャンし、数学的契約および Lipschitz 境界を検証する。
    /// 違反があれば panic_dump.json を書き出して Err を返す。
    pub fn verify_safety_contracts(&self) -> Result<(), String> {
        assert!(!self.memory_dir.as_os_str().is_empty(), "Error: memory directory invalid");
        assert!(self.memory_dir.exists(), "Error: memory directory must exist");

        let storage = Storage::new_readonly(self.memory_dir.clone()).map_err(|e| e.to_string())?;
        let entries = storage.get_all_entries()?;

        let mut weights_sum: HashMap<String, f64> = HashMap::new();
        let mut limit = 0;
        
        for (k, v) in entries {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in verifier scan");
            
            if let Some(stripped) = k.strip_prefix("link:") {
                let parts: Vec<&str> = stripped.split("->").collect();
                if parts.len() == 2 {
                    let parent = parts[0];
                    let weight: f64 = v.parse().unwrap_or(0.0);
                    
                    let entry = weights_sum.entry(parent.to_string()).or_insert(0.0);
                    *entry += weight.abs();
                }
            }
        }

        let mut violation_detected = false;
        let mut offending_node = String::new();
        let mut offending_sum = 0.0;

        let mut limit_check = 0;
        for (node, sum) in &weights_sum {
            limit_check += 1;
            assert!(limit_check <= 100_000, "Error: Loop limit exceeded in checking weights");
            if *sum > 3.6 {
                violation_detected = true;
                offending_node = node.clone();
                offending_sum = *sum;
                break;
            }
        }

        if violation_detected {
            let dump = PanicDump {
                origin_cluster_id: offending_node.clone(),
                violation_type: "LipschitzViolation".to_string(),
            };
            let dump_path = self.memory_dir.join("panic_dump.json");
            let dump_str = serde_json::to_string(&dump).map_err(|e| e.to_string())?;
            std::fs::write(&dump_path, dump_str).map_err(|e| e.to_string())?;
            
            assert!(dump_path.exists(), "Error: panic_dump.json must be created upon violation");
            return Err(format!("Lipschitz violation detected at node: {}, sum = {}", offending_node, offending_sum));
        }

        assert!(limit <= 100_000, "Error: post-condition check for scan limit");
        Ok(())
    }
}
