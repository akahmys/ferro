use crate::hippocampus::{Hippocampus, EpisodicSlot};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_hippocampus_ring_buffer() {
    let (_surprise_tx, surprise_rx) = mpsc::channel(10);
    let temp_file = std::env::temp_dir().join("episodic_buffer.csv");
    let path_str = temp_file.to_str().map(|s| s.to_string()).unwrap_or_default();
    
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

    let res = hippo.persist_buffer().await;
    assert!(res.is_ok());
    assert!(temp_file.exists());
    let _ = std::fs::remove_file(temp_file);
}
