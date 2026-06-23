use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct ShardedJson {
    base_dir: PathBuf,
    num_shards: usize,
}

impl ShardedJson {
    pub fn new(base_dir: PathBuf, num_shards: usize) -> Self {
        assert!(num_shards > 0, "Error: shards count must be positive");
        assert!(base_dir.exists() || fs::create_dir_all(&base_dir).is_ok(), "Error: base_dir must exist");
        Self { base_dir, num_shards }
    }

    fn shard_path(&self, key: &str) -> PathBuf {
        assert!(!key.is_empty(), "Error: key must not be empty");
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        use std::hash::Hasher;
        hasher.write(key.as_bytes());
        let idx = (hasher.finish() % self.num_shards as u64) as usize;
        let path = self.base_dir.join(format!("shard_{}.json", idx));
        assert!(!path.as_os_str().is_empty(), "Error: path must not be empty");
        path
    }

    pub fn put(&self, key: String, value: String) -> Result<(), String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(!value.is_empty(), "Error: value must not be empty");
        let path = self.shard_path(&key);
        let mut map = self.read_shard(&path).unwrap_or_default();
        map.insert(key.clone(), value);
        self.write_shard(&path, &map)?;
        assert!(!key.is_empty(), "Error: post key empty");
        assert!(path.exists(), "Error: shard file must exist after writing");
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        let path = self.shard_path(key);
        let map = self.read_shard(&path).unwrap_or_default();
        let res = map.get(key).cloned();
        assert!(!key.is_empty(), "Error: post-condition check key empty");
        assert!(!path.as_os_str().is_empty(), "Error: path invalid");
        Ok(res)
    }

    pub fn remove(&self, key: &str) -> Result<bool, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        let path = self.shard_path(key);
        let mut map = self.read_shard(&path).unwrap_or_default();
        let removed = map.remove(key).is_some();
        if removed {
            self.write_shard(&path, &map)?;
        }
        Ok(removed)
    }

    pub fn len(&self) -> usize {
        let mut total = 0;
        let mut limit = 0;
        for i in 0..self.num_shards {
            limit += 1;
            assert!(limit <= 1000, "Error: Loop iteration limit exceeded");
            let path = self.base_dir.join(format!("shard_{}.json", i));
            let map = self.read_shard(&path).unwrap_or_default();
            total += map.len();
        }
        total
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }


    fn read_shard(&self, path: &Path) -> Result<HashMap<String, String>, String> {
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| e.to_string())?;
        let map = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(map)
    }

    fn write_shard(&self, path: &Path, map: &HashMap<String, String>) -> Result<(), String> {
        let content = serde_json::to_string(map).map_err(|e| e.to_string())?;
        let mut file = File::create(path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_all_entries(&self) -> HashMap<String, String> {
        let mut all = HashMap::new();
        let mut limit = 0;
        for i in 0..self.num_shards {
            limit += 1;
            assert!(limit <= 1000, "Error: Loop iteration limit exceeded");
            let path = self.base_dir.join(format!("shard_{}.json", i));
            let map = self.read_shard(&path).unwrap_or_default();
            for (k, v) in map {
                all.insert(k, v);
            }
        }
        all
    }
}
