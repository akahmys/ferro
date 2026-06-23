use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use ferro_core::storage::Storage;

#[derive(Debug, Serialize, Deserialize)]
pub struct PanicDump {
    pub origin_cluster_id: String,
    #[serde(alias = "error_type")]
    pub violation_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BreedingSignals {
    pub curriculum_stage: usize,
    pub plasticity_boost: f64,
    pub vocal_damping_ratio: f64,
    pub target_surprise: f64,
    pub interrupt_active: bool,
    pub prune_cluster_ids: Vec<String>,
}

pub struct Pruner {
    memory_dir: PathBuf,
}

impl Pruner {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        Self { memory_dir }
    }

    /// pain_history.csv に痛覚履歴を追記し、再発回数 N_recur を計算する
    pub fn record_pain_and_get_recurrence(&self, origin_id: &str, violation: &str) -> usize {
        assert!(!origin_id.is_empty(), "Error: origin_id must not be empty");
        let path = self.memory_dir.join("pain_history.csv");
        
        let mut count = 0;
        if path.exists() {
            if let Ok(mut rdr) = csv::Reader::from_path(&path) {
                let mut limit = 0;
                for result in rdr.records() {
                    limit += 1;
                    assert!(limit <= 100_000, "Error: Loop limit exceeded in history scan");
                    if let Ok(record) = result {
                        if record.get(1) == Some(origin_id) {
                            count += 1;
                        }
                    }
                }
            }
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path);

        if let Ok(f) = file {
            let mut wtr = csv::Writer::from_writer(f);
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = wtr.write_record(&[ts.to_string(), origin_id.to_string(), violation.to_string()]);
            let _ = wtr.flush();
        }
        
        count + 1
    }

    /// トポロジーデータを Storage から読み込んで、子ノードから親ノードへの逆引きグラフを構築する
    pub fn build_child_to_parents_map(&self, storage: &Storage) -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let entries = match storage.get_all_entries() {
            Ok(e) => e,
            Err(_) => return map,
        };

        let mut limit = 0;
        for (k, _) in entries {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in build_child_to_parents_map");
            
            // link:parent->child 形式のキーを探す
            if k.starts_with("link:") {
                let parts: Vec<&str> = k["link:".len()..].split("->").collect();
                if parts.len() == 2 {
                    let parent = parts[0].to_string();
                    let child = parts[1].to_string();
                    map.entry(child).or_default().push(parent);
                }
            }
        }
        map
    }

    /// スタック探索を用いて親ノードを遡及トレースし、prune_set を決定する
    pub fn trace_parents(
        &self,
        origin_id: &str,
        depth_limit: usize,
        child_to_parents: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        assert!(!origin_id.is_empty(), "Error: origin_id must not be empty");
        let mut prune_set = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        stack.push((origin_id.to_string(), 0));
        visited.insert(origin_id.to_string());

        let mut limit = 0;
        while let Some((node, depth)) = stack.pop() {
            limit += 1;
            assert!(limit <= 1000, "Error: Stack search loop limit exceeded");
            prune_set.insert(node.clone());

            if depth < depth_limit {
                if let Some(parents) = child_to_parents.get(&node) {
                    for parent in parents {
                        if !visited.contains(parent) {
                            visited.insert(parent.clone());
                            stack.push((parent.clone(), depth + 1));
                        }
                    }
                }
            }
        }
        
        prune_set.into_iter().collect()
    }

    /// panic_dump.json を処理し、breeding_signals.json を書き出す
    pub fn perform_pruning(&self) -> Result<(), String> {
        let dump_path = self.memory_dir.join("panic_dump.json");
        if !dump_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&dump_path).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&dump_path);
        let dump: PanicDump = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let n_recur = self.record_pain_and_get_recurrence(&dump.origin_cluster_id, &dump.violation_type);
        
        let d_initial = match dump.violation_type.as_str() {
            "OOM" => 1,
            _ => 10, // 全遡及としての最大世代
        };
        let depth_limit = std::cmp::max(d_initial, n_recur);

        let storage = Storage::new(self.memory_dir.clone(), 5000);
        let child_to_parents = self.build_child_to_parents_map(&storage);
        let prune_ids = self.trace_parents(&dump.origin_cluster_id, depth_limit, &child_to_parents);

        let signals = BreedingSignals {
            curriculum_stage: 7,
            plasticity_boost: 1.25,
            vocal_damping_ratio: 0.85,
            target_surprise: 0.45,
            interrupt_active: false,
            prune_cluster_ids: prune_ids,
        };

        let breeding_path = self.memory_dir.join("breeding_signals.json");
        let signals_str = serde_json::to_string(&signals).map_err(|e| e.to_string())?;
        fs::write(&breeding_path, signals_str).map_err(|e| e.to_string())?;
        
        Ok(())
    }
}
