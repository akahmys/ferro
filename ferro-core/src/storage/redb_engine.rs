use redb::{Database, TableDefinition, ReadableTableMetadata};
use std::path::PathBuf;
use std::sync::Arc;

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("actors");

pub struct RedbEngine {
    db: Arc<Database>,
}

impl RedbEngine {
    pub fn new(path: PathBuf) -> Result<Self, String> {
        let db = Database::create(path).map_err(|e| e.to_string())?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn put(&self, key: String, value: String) -> Result<(), String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(!value.is_empty(), "Error: value must not be empty");
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

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        assert!(!key.is_empty(), "Error: key must not be empty");
        assert!(key.len() < 1000, "Error: key is too long");
        let read_txn = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_txn.open_table(TABLE).map_err(|e| e.to_string())?;
        let val = table.get(key).map_err(|e| e.to_string())?;
        let res = val.map(|v| v.value().to_string());
        Ok(res)
    }

    pub fn len(&self) -> usize {
        let read_txn = match self.db.begin_read() {
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
}
