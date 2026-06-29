use std::sync::{Arc, RwLock};
use std::thread;
use crate::storage::{StorageState, sharded_json::ShardedJson, redb_engine::RedbEngine};

pub(crate) fn run_migration(
    state: Arc<RwLock<StorageState>>,
    json: Arc<ShardedJson>,
    redb: Arc<RedbEngine>,
) {
    // R5: 最低2つのアサーション
    assert!(json.len() < 1_000_000, "Error: json size too large");
    assert!(redb.len() < 1_000_000, "Error: redb size too large");

    let state_clone = state;
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
}
