use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;

use ferro_body::console_vocal::ConsoleVocalMonitor;
use ferro_body::sensory_generator::SensoryGenerator;
use ferro_body::system_metrics::SystemMetricsSampler;

fn setup_memory_dir() -> PathBuf {
    let dir_str = env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    assert!(!dir_str.is_empty(), "Error: memory directory string is empty");
    let path = PathBuf::from(dir_str);
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    assert!(path.exists(), "Error: failed to create memory directory");
    path
}

#[tokio::main]
async fn main() {
    let memory_dir = setup_memory_dir();

    let mut sampler = SystemMetricsSampler::new(memory_dir.clone());
    let mut generator = SensoryGenerator::new(memory_dir.clone());
    let monitor = ConsoleVocalMonitor::new(memory_dir.clone());

    let mut interval = time::interval(Duration::from_millis(100));

    let mut ticks = 0;
    while ticks < 1000 {
        let _ = interval.tick().await;
        ticks += 1;

        let _ = sampler.sample_and_write();
        let _ = generator.generate_and_write();

        if let Ok(Some(output)) = monitor.monitor_and_validate() {
            println!("Vocal Output Detected: {}", output);
        }

        let panic_path = memory_dir.join("panic_dump.json");
        if panic_path.exists() {
            println!("FERRO Body: Detected panic_dump.json. Terminating body loop.");
            break;
        }
    }

    println!("FERRO Body clean exit completed.");
}
