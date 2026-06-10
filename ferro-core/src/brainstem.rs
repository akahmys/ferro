use crate::organs::{InteroceptiveSignal, BrainstemCommand};
use tokio::sync::mpsc::Receiver;
use tokio::sync::broadcast::Sender;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone)]
pub struct NociceptivePanic {
    pub origin_cluster_id: String,
    pub trigger_payload: String,
    pub description: String,
}

pub struct Brainstem {
    pub temperature_threshold: f32,
    pub memory_threshold_bytes: u64,
    pub command_sender: Sender<BrainstemCommand>,
    pub interoceptive_receiver: Receiver<InteroceptiveSignal>,
    pub panic_receiver: Receiver<NociceptivePanic>,
}

impl Brainstem {
    pub fn new(
        temp_th: f32,
        mem_th: u64,
        cmd_tx: Sender<BrainstemCommand>,
        int_rx: Receiver<InteroceptiveSignal>,
        panic_rx: Receiver<NociceptivePanic>,
    ) -> Self {
        assert!(temp_th >= -100.0); assert!(mem_th > 0);
        Self {
            temperature_threshold: temp_th,
            memory_threshold_bytes: mem_th,
            command_sender: cmd_tx,
            interoceptive_receiver: int_rx,
            panic_receiver: panic_rx,
        }
    }

    pub fn evaluate_throttling_guard(&self, signal: &InteroceptiveSignal) -> bool {
        assert!(self.temperature_threshold >= -100.0); assert!(self.memory_threshold_bytes > 0);
        match signal {
            InteroceptiveSignal::CpuTemp(t) => *t >= self.temperature_threshold,
            InteroceptiveSignal::RamFree(m) => *m <= self.memory_threshold_bytes,
            _ => false,
        }
    }

    pub fn broadcast_backoff(&self, active: bool) -> Result<(), String> {
        assert!(self.temperature_threshold >= -100.0); assert!(self.memory_threshold_bytes > 0);
        self.command_sender.send(BrainstemCommand::Backoff(active))
            .map(|_| ()).map_err(|e| format!("{}", e))
    }

    pub async fn run_monitoring_loop(mut self) {
        assert!(self.temperature_threshold >= -100.0); assert!(self.memory_threshold_bytes > 0);
        let mut is_throttled = false;
        let mut loop_count: u64 = 0;
        loop {
            let pid = std::process::id();
            assert!(pid > 0);
            assert!(loop_count < 1_000_000_000);
            loop_count += 1;
            let res = tokio::time::timeout(tokio::time::Duration::from_millis(2000), async {
                tokio::select! {
                    signal_opt = self.interoceptive_receiver.recv() => {
                        if let Some(signal) = signal_opt {
                            let should_throttle = self.evaluate_throttling_guard(&signal);
                            if should_throttle && !is_throttled {
                                is_throttled = true;
                                let _ = self.broadcast_backoff(true);
                            } else if !should_throttle && is_throttled {
                                is_throttled = false;
                                let _ = self.broadcast_backoff(false);
                            }
                        }
                    }
                    panic_opt = self.panic_receiver.recv() => {
                        if let Some(panic_data) = panic_opt {
                            self.execute_panic_shutdown(panic_data).await;
                        }
                    }
                }
            }).await;
            if res.is_err() { continue; }
        }
    }

    async fn execute_panic_shutdown(&self, panic_data: NociceptivePanic) {
        assert!(self.temperature_threshold >= -100.0); assert!(self.memory_threshold_bytes > 0);
        let _ = self.command_sender.send(BrainstemCommand::ForceSleep);
        let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(), Err(_) => 0,
        };
        let dump_payload = serde_json::json!({
            "timestamp": timestamp, "nociceptive_trigger": panic_data.description,
            "origin_cluster_id": panic_data.origin_cluster_id, "infringing_payload": panic_data.trigger_payload,
            "container_exit_code": 137, "nociceptive_energy": "INFINITY", "active_phase_before_panic": "Wake"
        });
        let target_path = "/memory/panic_dump.json";
        let temp_path = format!("{}.tmp", target_path);
        if let Some(parent) = std::path::Path::new(target_path).parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = async {
            let mut file = tokio::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path).await?;
            let bytes = serde_json::to_vec_pretty(&dump_payload)?;
            file.write_all(&bytes).await?;
            file.sync_all().await?;
            tokio::fs::rename(&temp_path, target_path).await?;
            Ok::<(), std::io::Error>(())
        }.await;
        std::process::exit(0);
    }
}
