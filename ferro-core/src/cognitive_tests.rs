#[cfg(test)]
mod tests {
    use crate::storage::ShardedJsonStorage;
    use crate::midbrain::Midbrain;
    use crate::hippocampus::{Hippocampus, EpisodicSlot};
    use crate::organs::{SensorySignal, EfferenceCopy, SensoryMuteCommand};
    use tokio::sync::{mpsc, broadcast};

    #[tokio::test]
    async fn test_sharded_storage() {
        let temp_dir = std::env::temp_dir().join("ferro_test_storage");
        let _ = std::fs::create_dir_all(&temp_dir);
        let storage = ShardedJsonStorage::new(&temp_dir);
        let node_id = "test_node_id";
        let (shard_dir, file_path) = storage.resolve_paths(node_id);
        assert!(shard_dir.starts_with(&temp_dir));
        assert!(file_path.ends_with("test_node_id.json"));

        let data = "hello".to_string();
        storage.write_node(node_id, &data).await.unwrap();
        let read: String = storage.read_node(node_id).await.unwrap();
        assert_eq!(read, data);
        storage.delete_node(node_id).await.unwrap();
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_midbrain_efference_matching() {
        let (eff_tx, eff_rx) = mpsc::channel(10);
        let (_echo_tx, echo_rx) = mpsc::channel(10);
        let (mute_tx, mut mute_rx) = broadcast::channel(10);
        let (surprise_tx, mut surprise_rx) = mpsc::channel(10);

        let mut midbrain = Midbrain::new(eff_rx, echo_rx, mute_tx, surprise_tx, 2000, 5);
        let eff = EfferenceCopy {
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            command_hash: 12345,
            origin_cluster_id: "cortex_01".to_string(),
            expected_tokens: vec!["hello".to_string()],
        };
        let _ = eff_tx.send(eff.clone()).await;
        midbrain.pending_efference.push_back(eff);
        
        midbrain.handle_sensory_echo(SensorySignal::ProprioceptiveEcho(vec!["hello".to_string()])).await;
        let surprise = surprise_rx.recv().await.unwrap();
        assert_eq!(surprise, 0.0);

        let mute_cmd = mute_rx.recv().await.unwrap();
        assert_eq!(mute_cmd, SensoryMuteCommand { mute: false, attenuation_db: 0.0 });
    }

    #[tokio::test]
    async fn test_hippocampus_ring_buffer() {
        let (_surprise_tx, surprise_rx) = mpsc::channel(10);
        let temp_file = std::env::temp_dir().join("episodic_buffer.csv");
        let path_str = temp_file.to_str().unwrap().to_string();
        let mut hippo = Hippocampus::new(3, path_str, surprise_rx);

        let slot = EpisodicSlot {
            timestamp: 123,
            event_id: "evt_1".to_string(),
            origin_cluster_id: "test".to_string(),
            sensory_summary: "s".to_string(),
            motor_summary: "m".to_string(),
            surprise_level: 0.2,
        };
        hippo.push_slot(slot.clone());
        assert_eq!(hippo.count, 1);
        assert_eq!(hippo.buffer[0], Some(slot));

        let _ = hippo.persist_buffer().await;
        assert!(temp_file.exists());
        let _ = std::fs::remove_file(temp_file);
    }
}
