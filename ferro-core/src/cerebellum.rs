use crate::organs::{SensorySignal, MotorCommand, EfferenceCopy};
use crate::brainstem::NociceptivePanic;
use tokio::sync::mpsc::Sender;

pub struct Cerebellum {
    pub tick_rate_ms: u64,
    #[allow(dead_code)]
    pub sensory_sender: Sender<SensorySignal>,
    pub efference_sender: Sender<EfferenceCopy>,
    pub panic_sender: Sender<NociceptivePanic>,
}

impl Cerebellum {
    pub fn new(
        tick: u64,
        sensory_tx: Sender<SensorySignal>,
        efference_tx: Sender<EfferenceCopy>,
        panic_tx: Sender<NociceptivePanic>,
    ) -> Self {
        assert!(tick > 0); assert!(tick < 10000);
        Self { tick_rate_ms: tick, sensory_sender: sensory_tx, efference_sender: efference_tx, panic_sender: panic_tx }
    }

    #[allow(dead_code)]
    pub fn sensory_sender(&self) -> &Sender<SensorySignal> {
        assert!(self.tick_rate_ms > 0); assert!(self.tick_rate_ms < 100000);
        &self.sensory_sender
    }

    #[allow(dead_code)]
    pub fn wait_next_tick(&self) {
        assert!(self.tick_rate_ms > 0); assert!(self.tick_rate_ms < 100000);
        std::thread::park_timeout(std::time::Duration::from_millis(self.tick_rate_ms));
    }

    pub fn verify_motor_nociception(&self, cmd: &MotorCommand) -> Result<(), String> {
        assert!(self.tick_rate_ms > 0); assert!(!cmd.origin_cluster_id.is_empty());
        if cmd.target_path.contains("..") || cmd.target_path.starts_with('/') {
            return Err("NociceptiveReflexTriggered: Directory Traversal Attempt".to_string());
        }
        if let Some(port_num) = cmd.port {
            if port_num != 123 {
                return Err("NociceptiveReflexTriggered: Unwhitelisted Socket Communication".to_string());
            }
        }
        Ok(())
    }

    pub async fn process_motor_command(
        &self,
        cmd: MotorCommand,
        motor_sender: &Sender<MotorCommand>,
    ) -> Result<(), String> {
        assert!(self.tick_rate_ms > 0); assert!(!cmd.origin_cluster_id.is_empty());
        match self.verify_motor_nociception(&cmd) {
            Ok(_) => {
                let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_secs(), Err(_) => 0,
                };
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&cmd.payload, &mut hasher);
                use std::hash::Hasher;
                let command_hash = hasher.finish();

                let eff_copy = EfferenceCopy {
                    timestamp,
                    command_hash,
                    origin_cluster_id: cmd.origin_cluster_id.clone(),
                    expected_tokens: self.predict_vocal_tokens(&cmd.payload),
                };
                let _ = self.efference_sender.send(eff_copy).await;

                motor_sender.send(cmd).await
                    .map_err(|e| format!("Actuator disconnected: {}", e))
            }
            Err(violation_msg) => {
                let panic_payload = NociceptivePanic {
                    origin_cluster_id: cmd.origin_cluster_id.clone(),
                    trigger_payload: format!("target: {}, port: {:?}", cmd.target_path, cmd.port),
                    description: violation_msg.clone(),
                };
                let _ = self.panic_sender.send(panic_payload).await;
                Err(violation_msg)
            }
        }
    }

    fn predict_vocal_tokens(&self, data: &[u8]) -> Vec<String> {
        assert!(self.tick_rate_ms > 0); assert!(data.len() <= 10_000_000);
        if let Ok(text) = std::str::from_utf8(data) {
            text.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }
}
