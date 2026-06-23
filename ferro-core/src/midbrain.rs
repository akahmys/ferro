use crate::message::{EfferenceCopy, SensoryMuteCommand};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct Midbrain {
    mute_tx: mpsc::Sender<SensoryMuteCommand>,
    expected_echoes: Arc<RwLock<Vec<(Instant, EfferenceCopy)>>>,
}

impl Midbrain {
    pub fn new(mute_tx: mpsc::Sender<SensoryMuteCommand>) -> Self {
        assert!(mute_tx.capacity() > 0, "Error: mute channel must not be full");
        let new_midbrain = Self {
            mute_tx,
            expected_echoes: Arc::new(RwLock::new(Vec::new())),
        };
        assert!(new_midbrain.expected_echoes.read().is_ok(), "Error: lock must be available");
        new_midbrain
    }

    pub async fn handle_efference_copy(&self, copy: EfferenceCopy) -> Result<(), String> {
        assert!(copy.timestamp > 0, "Error: timestamp must be positive");
        assert!(!copy.expected_tokens.is_empty(), "Error: expected tokens must not be empty");

        {
            let mut echoes = self.expected_echoes.write().map_err(|e| e.to_string())?;
            echoes.push((Instant::now(), copy));
        }

        self.mute_tx.send(SensoryMuteCommand {
            mute: true,
            attenuation_db: 40.0,
        }).await.map_err(|e| e.to_string())?;

        let mute_tx_clone = self.mute_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            let _ = mute_tx_clone.send(SensoryMuteCommand {
                mute: false,
                attenuation_db: 0.0,
            }).await;
        });

        assert!(!self.mute_tx.is_closed(), "Error: mute channel must not be closed");
        assert!(self.mute_tx.capacity() < 2000, "Error: capacity check");
        Ok(())
    }

    pub fn handle_proprioceptive_echo(&self, echo_tokens: Vec<String>) -> Result<f64, String> {
        assert!(!echo_tokens.is_empty(), "Error: echo tokens must not be empty");
        assert!(echo_tokens.len() < 100, "Error: echo tokens length limit exceeded");

        let mut echoes = self.expected_echoes.write().map_err(|e| e.to_string())?;
        let now = Instant::now();
        let mut found_idx = None;
        let mut limit = 0;

        for (i, (timestamp, copy)) in echoes.iter().enumerate() {
            limit += 1;
            assert!(limit <= 1000, "Error: Loop iteration limit exceeded");
            if now.duration_since(*timestamp) < Duration::from_millis(500)
                && copy.expected_tokens == echo_tokens
            {
                found_idx = Some(i);
                break;
            }
        }

        let surprise = if let Some(idx) = found_idx {
            echoes.remove(idx);
            0.0
        } else {
            1.0
        };

        let mut clean_limit = 0;
        echoes.retain(|(timestamp, _)| {
            clean_limit += 1;
            assert!(clean_limit <= 1000, "Error: Loop iteration limit exceeded in cleanup");
            now.duration_since(*timestamp) < Duration::from_secs(1)
        });

        assert!(surprise == 0.0 || surprise == 1.0, "Error: surprise value must be binary 0 or 1");
        Ok(surprise)
    }
}
