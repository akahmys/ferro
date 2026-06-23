use ferro_core::message::{EfferenceCopy, SensoryMuteCommand};
use ferro_core::midbrain::Midbrain;
use ferro_core::hippocampus::{Hippocampus, EpisodicSlot};
use ferro_core::storage::Storage;

use std::fs;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_self_ear_muting() -> Result<(), String> {
    let (mute_tx, mut mute_rx) = mpsc::channel::<SensoryMuteCommand>(100);
    let midbrain = Midbrain::new(mute_tx);

    let copy = EfferenceCopy {
        timestamp: 100,
        command_hash: 12345,
        origin_cluster_id: "cluster_0".to_string(),
        expected_tokens: vec!["いぬ".to_string()],
    };

    midbrain.handle_efference_copy(copy).await?;

    let cmd = mute_rx.recv().await.ok_or("No mute command received")?;
    assert!(cmd.mute, "Mute should be true");
    assert!((cmd.attenuation_db - 40.0).abs() < 1e-5);

    tokio::time::sleep(Duration::from_millis(250)).await;
    let cmd = mute_rx.recv().await.ok_or("No unmute command received")?;
    assert!(!cmd.mute, "Mute should be false");

    Ok(())
}

#[tokio::test]
async fn test_midbrain_echo_match() -> Result<(), String> {
    let (mute_tx, _mute_rx) = mpsc::channel::<SensoryMuteCommand>(100);
    let midbrain = Midbrain::new(mute_tx);

    let copy = EfferenceCopy {
        timestamp: 100,
        command_hash: 12345,
        origin_cluster_id: "cluster_0".to_string(),
        expected_tokens: vec!["いぬ".to_string()],
    };

    midbrain.handle_efference_copy(copy).await?;

    let surprise = midbrain.handle_proprioceptive_echo(vec!["いぬ".to_string()])?;
    assert!((surprise - 0.0).abs() < 1e-5);

    let surprise = midbrain.handle_proprioceptive_echo(vec!["ねこ".to_string()])?;
    assert!((surprise - 1.0).abs() < 1e-5);

    Ok(())
}

#[tokio::test]
async fn test_hippocampus_csv() -> Result<(), String> {
    let test_dir = std::env::temp_dir().join("ferro_test_hippocampus");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);
    let csv_path = test_dir.join("episodic_buffer.csv");

    let hippocampus = Hippocampus::new(csv_path.clone());

    let slot1 = EpisodicSlot {
        timestamp: 100,
        input: "いぬ".to_string(),
        output: "わんわん".to_string(),
        surprise: 0.0,
    };
    let slot2 = EpisodicSlot {
        timestamp: 101,
        input: "ねこ".to_string(),
        output: "にゃー".to_string(),
        surprise: 1.0,
    };

    hippocampus.record_episode(slot1)?;
    hippocampus.record_episode(slot2)?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(csv_path.exists(), "CSV file must be created");
    let content = fs::read_to_string(&csv_path).map_err(|e| e.to_string())?;
    assert!(content.contains("timestamp,input,output,surprise"), "CSV header missing");
    assert!(content.contains("いぬ"), "CSV content missing");
    assert!(content.contains("ねこ"), "CSV content missing");

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[tokio::test]
async fn test_zero_downtime_migration() -> Result<(), String> {
    let test_dir = std::env::temp_dir().join("ferro_test_storage");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);

    let storage = Storage::new(test_dir.clone(), 3);

    storage.put("key_1".to_string(), "val_1".to_string())?;
    storage.put("key_2".to_string(), "val_2".to_string())?;

    assert_eq!(storage.len(), 2);
    assert_eq!(storage.get("key_1")?, Some("val_1".to_string()));

    storage.put("key_3".to_string(), "val_3".to_string())?;

    storage.put("key_4".to_string(), "val_4".to_string())?;
    assert_eq!(storage.get("key_2")?, Some("val_2".to_string()));
    assert_eq!(storage.get("key_4")?, Some("val_4".to_string()));

    tokio::time::sleep(Duration::from_millis(500)).await;

    assert_eq!(storage.get("key_1")?, Some("val_1".to_string()));
    assert_eq!(storage.get("key_2")?, Some("val_2".to_string()));
    assert_eq!(storage.get("key_3")?, Some("val_3".to_string()));
    assert_eq!(storage.get("key_4")?, Some("val_4".to_string()));

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}

#[tokio::test]
async fn test_learning_stages_simulation() -> Result<(), String> {
    let test_dir = std::env::temp_dir().join("ferro_test_learning");
    let _ = fs::remove_dir_all(&test_dir);
    let _ = fs::create_dir_all(&test_dir);

    let storage = Storage::new(test_dir.clone(), 100);

    storage.put("noun:いぬ".to_string(), "0.8".to_string())?;
    storage.put("noun:ねこ".to_string(), "0.9".to_string())?;

    let val = storage.get("noun:いぬ")?.ok_or("noun not found")?;
    let weight: f64 = val.parse().map_err(|e| format!("{:?}", e))?;
    assert!(weight > 0.5);

    storage.put("link:いぬ->はしる".to_string(), "0.2".to_string())?;

    let current_link_str = storage.get("link:いぬ->はしる")?.unwrap_or_else(|| "0.0".to_string());
    let current_link: f64 = current_link_str.parse().map_err(|e| format!("{:?}", e))?;
    let new_weight = current_link + 0.3;
    storage.put("link:いぬ->はしる".to_string(), new_weight.to_string())?;

    let updated_link_str = storage.get("link:いぬ->はしる")?.ok_or("link not found")?;
    let updated_link: f64 = updated_link_str.parse().map_err(|e| format!("{:?}", e))?;
    assert!(updated_link > 0.4);

    let _ = fs::remove_dir_all(&test_dir);
    Ok(())
}
