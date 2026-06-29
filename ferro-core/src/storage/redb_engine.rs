use redb::{Database, TableDefinition, ReadableTableMetadata, ReadableTable};
use std::path::PathBuf;
use std::sync::Arc;

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("actors");

pub struct RedbEngine {
    db: Arc<Database>,
    read_only: bool,
}

impl RedbEngine {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        assert!(!path.as_os_str().is_empty(), "Error: path must not be empty");
        let db = Database::create(&path).map_err(|e| e.to_string())?;
        let engine = Self { db: Arc::new(db), read_only: false };
        assert!(engine.begin_read().is_ok(), "Error: failed to open read txn on initialization");
        Ok(engine)
    }

    pub fn new_readonly(path: PathBuf) -> Result<Self, String> {
        assert!(!path.as_os_str().is_empty(), "Error: path must not be empty");
        let db = Database::open(&path).map_err(|e| e.to_string())?;
        let engine = Self { db: Arc::new(db), read_only: true };
        assert!(engine.begin_read().is_ok(), "Error: failed to open read txn on initialization");
        Ok(engine)
    }

    pub fn is_readonly(&self) -> bool {
        self.read_only
    }

    fn begin_read(&self) -> Result<redb::ReadTransaction, String> {
        self.db.begin_read().map_err(|e| e.to_string())
    }

    pub fn put(&self, key: String, value: String) -> Result<(), String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(!value.is_empty(), "Error: value must not be empty");
        assert!(key.len() < 1000, "Error: key is too long");

        if self.is_readonly() {
            return Err("Cannot write to read-only database".to_string());
        }

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_txn.open_table(TABLE).map_err(|e| e.to_string())?;
            table.insert(key.as_str(), value.as_str()).map_err(|e| e.to_string())?;
        }
        write_txn.commit().map_err(|e| e.to_string())?;
        
        assert!(!key.is_empty(), "Error: post-condition check key empty");
        assert!(!value.is_empty(), "Error: post-condition check value empty");
        Ok(())
    }

    pub fn remove(&self, key: &str) -> Result<bool, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(key.len() < 1000, "Error: key is too long");

        if self.is_readonly() {
            return Err("Cannot write to read-only database".to_string());
        }

        let write_txn = self.db.begin_write().map_err(|e| e.to_string())?;
        let removed = {
            let mut table = write_txn.open_table(TABLE).map_err(|e| e.to_string())?;
            table.remove(key).map_err(|e| e.to_string())?.is_some()
        };
        write_txn.commit().map_err(|e| e.to_string())?;
        
        assert!(!key.is_empty(), "Error: post-condition check key empty after removal");
        Ok(removed)
    }

    pub fn get_all_entries(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let read_txn = self.begin_read()?;
        let table = read_txn.open_table(TABLE).map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        let mut limit = 0;
        for result in table.iter().map_err(|e| e.to_string())? {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in redb get_all_entries");
            if let Ok((k, v)) = result {
                map.insert(k.value().to_string(), v.value().to_string());
            }
        }
        assert!(limit <= 100_000, "Error: post-condition loop limit verification");
        Ok(map)
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(key.len() < 1000, "Error: key is too long");
        let read_txn = self.begin_read()?;
        let table = read_txn.open_table(TABLE).map_err(|e| e.to_string())?;
        let val = table.get(key).map_err(|e| e.to_string())?;
        let res = val.map(|v| v.value().to_string());
        
        assert!(!key.is_empty(), "Error: post-condition key checking");
        Ok(res)
    }

    pub fn len(&self) -> usize {
        let read_txn = match self.begin_read() {
            Ok(tx) => tx,
            Err(_) => return 0,
        };
        let table = match read_txn.open_table(TABLE) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        match table.len() {
            Ok(len) => len as usize,
            Err(_) => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
