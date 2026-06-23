use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::SystemTime;
use sysinfo::System;

pub struct SystemMetricsSampler {
    sys: System,
    memory_dir: PathBuf,
}

impl SystemMetricsSampler {
    pub fn new(memory_dir: PathBuf) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys, memory_dir }
    }

    pub fn sample_and_write(&mut self) -> Result<(), std::io::Error> {
        // R5: アサーション最低2つを義務付け
        assert!(self.memory_dir.exists(), "Error: memory directory must exist");

        self.sys.refresh_cpu();
        self.sys.refresh_memory();

        let cpu_usage = self.sys.global_cpu_info().cpu_usage();
        let free_mem = self.sys.free_memory();

        let csv_path = self.memory_dir.join("brainstem_metrics.csv");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(csv_path)?;

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let line = format!("{},{:.2},{}\n", now, cpu_usage, free_mem);
        file.write_all(line.as_bytes())?;

        let signal_path = self.memory_dir.join("interoceptive_signals.json");
        let signal_data = serde_json::json!([
            { "CpuTemp": cpu_usage },
            { "RamFree": free_mem }
        ]);

        if let Ok(json_str) = serde_json::to_string_pretty(&signal_data) {
            let temp_path = self.memory_dir.join("interoceptive_signals.tmp");
            fs::write(&temp_path, json_str)?;
            fs::rename(&temp_path, &signal_path)?;
        }

        assert!(signal_path.exists(), "Error: signal file must exist after atomic write");
        Ok(())
    }
}
