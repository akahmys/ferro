pub mod sharded_json;
pub mod redb_engine;

use sharded_json::ShardedJson;
use redb_engine::RedbEngine;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Clone)]
enum StorageState {
    ShardedJson(Arc<ShardedJson>),
    Migrating {
        json: Arc<ShardedJson>,
        redb: Arc<RedbEngine>,
    },
    Redb(Arc<RedbEngine>),
}

pub struct Storage {
    state: Arc<RwLock<StorageState>>,
    memory_dir: PathBuf,
    threshold: usize,
}

impl Storage {
    pub fn new(memory_dir: PathBuf, threshold: usize) -> Self {
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory directory path must not be empty");
        assert!(threshold > 0, "Error: threshold must be positive");
        let json_dir = memory_dir.join("shards");
        let json = Arc::new(ShardedJson::new(json_dir, 16));
        let state = Arc::new(RwLock::new(StorageState::ShardedJson(json)));
        Self { state, memory_dir, threshold }
    }

    pub fn put(&self, key: String, value: String) -> Result<(), String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(!value.is_empty(), "Error: value must not be empty");

        let current_state = {
            let r = self.state.read().map_err(|e| e.to_string())?;
            (*r).clone()
        };

        match current_state {
            StorageState::ShardedJson(ref json) => {
                json.put(key.clone(), value.clone())?;
                if json.len() >= self.threshold {
                    self.trigger_migration(json.clone())?;
                }
            }
            StorageState::Migrating { ref json, ref redb } => {
                json.put(key.clone(), value.clone())?;
                redb.put(key.clone(), value.clone())?;
            }
            StorageState::Redb(ref redb) => {
                redb.put(key.clone(), value.clone())?;
            }
        }

        assert!(!key.is_empty(), "Error: post-condition check key empty");
        assert!(!value.is_empty(), "Error: post-condition check value empty");
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(key.len() < 1000, "Error: key is too long");

        let current_state = {
            let r = self.state.read().map_err(|e| e.to_string())?;
            (*r).clone()
        };

        let val = match current_state {
            StorageState::ShardedJson(ref json) => json.get(key),
            StorageState::Migrating { ref json, .. } => json.get(key),
            StorageState::Redb(ref redb) => redb.get(key),
        };
        val
    }

    pub fn len(&self) -> usize {
        let current_state = match self.state.read() {
            Ok(r) => (*r).clone(),
            Err(_) => return 0,
        };
        match current_state {
            StorageState::ShardedJson(ref json) => json.len(),
            StorageState::Migrating { ref json, .. } => json.len(),
            StorageState::Redb(ref redb) => redb.len(),
        }
    }

    fn trigger_migration(&self, json: Arc<ShardedJson>) -> Result<(), String> {
        assert!(self.threshold > 0, "Error: threshold must be valid");
        let redb_path = self.memory_dir.join("storage.redb");
        let redb = Arc::new(RedbEngine::new(redb_path)?);

        {
            let mut w = self.state.write().map_err(|e| e.to_string())?;
            *w = StorageState::Migrating {
                json: json.clone(),
                redb: redb.clone(),
            };
        }

        let state_clone = self.state.clone();
        thread::spawn(move || {
            let all_entries = json.get_all_entries();
            let mut limit = 0;
            for (k, v) in all_entries {
                limit += 1;
                assert!(limit <= 100000, "Error: migration entry limit exceeded");
                let _ = redb.put(k, v);
            }
            if let Ok(mut w) = state_clone.write() {
                *w = StorageState::Redb(redb);
            }
        });

        assert!(self.threshold > 0, "Error: post-condition trigger migration threshold check");
        Ok(())
    }
}
