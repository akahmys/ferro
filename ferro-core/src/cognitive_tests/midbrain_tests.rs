use crate::midbrain::Midbrain;
use crate::organs::{SensorySignal, EfferenceCopy, SensoryMuteCommand};
use tokio::sync::{mpsc, broadcast};

#[tokio::test]
async fn test_midbrain_efference_matching() {
    let (eff_tx, eff_rx) = mpsc::channel(10);
    let (_echo_tx, echo_rx) = mpsc::channel(10);
    let (mute_tx, mut mute_rx) = broadcast::channel(10);
    let (surprise_tx, mut surprise_rx) = mpsc::channel(10);

    let mut midbrain = Midbrain::new(eff_rx, echo_rx, mute_tx, surprise_tx, 2000, 5);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);
    
    let eff = EfferenceCopy {
        timestamp: now,
        command_hash: 12345,
        origin_cluster_id: "cortex_01".to_string(),
        expected_tokens: vec!["hello".to_string()],
    };
    let _ = eff_tx.send(eff.clone()).await;
    midbrain.pending_efference.push_back(eff);
    
    midbrain.handle_sensory_echo(SensorySignal::ProprioceptiveEcho(vec!["hello".to_string()])).await;
    
    let surprise = surprise_rx.recv().await;
    assert!(surprise.is_some());
    assert_eq!(surprise, Some(0.0));

    let mute_cmd = mute_rx.recv().await;
    assert!(mute_cmd.is_ok());
    assert_eq!(mute_cmd, Ok(SensoryMuteCommand { mute: false, attenuation_db: 0.0 }));
}

