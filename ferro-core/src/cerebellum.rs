use crate::message::MotorCommand;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct Cerebellum {
    terminate_flag: Arc<AtomicBool>,
    memory_dir: PathBuf,
}

impl Cerebellum {
    pub fn new(terminate_flag: Arc<AtomicBool>, memory_dir: PathBuf) -> Self {
        assert!(!terminate_flag.load(Ordering::SeqCst), "Error: terminate flag must be initially false");
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory directory must not be empty");
        Self {
            terminate_flag,
            memory_dir,
        }
    }

    pub fn censor_command(&self, command: &MotorCommand) -> Result<(), &'static str> {
        // R5: 引数ありの関数、および状態変更を伴う関数に最低2つのアサーションを義務付け
        assert!(!command.origin_cluster_id.is_empty(), "Error: Origin cluster ID must not be empty");

        let mut is_violation = false;
        let mut reason = "";

        // パス検閲: "/memory" または "/tmp" 以外への書き込みは不正パスと判定
        if !command.target_path.starts_with("/memory")
            && !command.target_path.starts_with("/tmp")
            && !command.target_path.starts_with("/private/tmp")
        {
            is_violation = true;
            reason = "Error: Write attempt outside allowed paths";
        }

        if command.port.is_some_and(|port| port < 1024 || port == 8080) {
            is_violation = true;
            reason = "Error: Forbidden port access";
        }

        if is_violation {
            self.trigger_nociceptive_reflex(command, reason);
            assert!(self.terminate_flag.load(Ordering::SeqCst), "Error: Terminate flag should be true");
            return Err(reason);
        }

        assert!(!self.terminate_flag.load(Ordering::SeqCst), "Error: System should not terminate under normal command");
        Ok(())
    }

    fn trigger_nociceptive_reflex(&self, command: &MotorCommand, reason: &str) {
        assert!(!command.origin_cluster_id.is_empty(), "Error: origin_cluster_id must not be empty");
        assert!(!reason.is_empty(), "Error: reason must not be empty");
        let dump_path = self.memory_dir.join("panic_dump.json");
        let dump_data = serde_json::json!({
            "error_code": "0x02",
            "error_type": "ERR_NOCICEPTIVE_REFLEX",
            "reason": reason,
            "origin_cluster_id": command.origin_cluster_id,
            "target_path": command.target_path,
            "port": command.port,
            "prediction_error": "inf"
        });

        if let (Ok(json_str), Ok(mut file)) = (serde_json::to_string_pretty(&dump_data), File::create(dump_path)) {
            let _ = file.write_all(json_str.as_bytes());
        }

        self.terminate_flag.store(true, Ordering::SeqCst);
    }
}
