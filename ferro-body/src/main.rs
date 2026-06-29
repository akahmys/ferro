#![deny(warnings)]
#![deny(clippy::all)]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;

use ferro_body::console_vocal::ConsoleVocalMonitor;
use ferro_body::sensory_generator::SensoryGenerator;
use ferro_body::system_metrics::SystemMetricsSampler;
use ferro_body::breeding::BreedingEngine;
use ferro_body::injector::SignalInjector;
use ferro_body::regularizer::Regularizer;

fn setup_memory_dir() -> PathBuf {
    let dir_str = env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    assert!(!dir_str.is_empty(), "Error: memory directory string is empty");
    let path = PathBuf::from(dir_str);
    if !path.exists() {
        let create_res = fs::create_dir_all(&path);
        if let Err(e) = create_res {
            eprintln!("Warning: failed to create memory directory: {:?}", e);
        }
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
    let mut breeding = BreedingEngine::new(memory_dir.clone());
    let mut injector = SignalInjector::new(memory_dir.clone());

    let mut interval = time::interval(Duration::from_millis(100));

    let mut ticks = 0;
    while ticks < 1000 {
        let _ = interval.tick().await;
        ticks += 1;

        let _ = sampler.sample_and_write();
        let _ = generator.generate_and_write();
        
        // カリキュラム進行と信号インジェクション
        let _ = injector.inject_signals();
        let stage = injector.get_curriculum_stage();
        
        // 可塑性・アライメント調整
        let _ = breeding.update_and_write(stage);

        // 代謝ペナルティの模擬計算とフィードバック出力
        let mock_atp = vec![1.0, 1.5, 0.8];
        let mock_errors = vec![0.02, 0.05, 0.01];
        let cost = Regularizer::calculate_metabolic_cost(&mock_atp);
        let penalty = Regularizer::calculate_dissonance_penalty(&mock_errors);

        let reg_signals = serde_json::json!({
            "metabolic_cost": cost,
            "dissonance_penalty": penalty,
        });
        if let Ok(json_str) = serde_json::to_string_pretty(&reg_signals) {
            let reg_path = memory_dir.join("regularizer_signals.json");
            let temp_path = memory_dir.join("regularizer_signals.tmp");
            if fs::write(&temp_path, json_str).is_ok() {
                let _ = fs::rename(&temp_path, &reg_path);
            }
        }

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
