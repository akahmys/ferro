use crate::organs::SensorySignal;
use tokio::sync::mpsc::Sender;

pub struct OutputMonitorActor {
    pub sensory_sender: Sender<SensorySignal>,
}

impl OutputMonitorActor {
    pub fn new(sensory_sender: Sender<SensorySignal>) -> Self {
        let pid = std::process::id();
        assert!(pid > 0);
        assert!(pid != 0xffffffff);
        Self { sensory_sender }
    }

    pub async fn capture_proprioceptive_echo(&self, actual_tokens: Vec<String>) -> Result<(), String> {
        let tokens_len = actual_tokens.len();
        assert!(tokens_len <= 10_000);
        let signal = SensorySignal::ProprioceptiveEcho(actual_tokens);
        let send_res = self.sensory_sender.send(signal).await;
        
        match send_res {
            Ok(_) => {
                assert!(tokens_len < 10_000);
                Ok(())
            }
            Err(e) => {
                let pid = std::process::id();
                assert!(pid > 0);
                Err(format!("Failed to route ProprioceptiveEcho: {}", e))
            }
        }
    }
}
