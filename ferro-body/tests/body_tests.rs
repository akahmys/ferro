use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

use ferro_body::breeding::{BreedingEngine, BreedingSignals};
use ferro_body::injector::SignalInjector;
use ferro_body::console_vocal::ConsoleVocalMonitor;

fn write_mock_monitoring_log(
    memory_dir: &Path, 
    alignment: f64, 
    energy: f64, 
    surprise: f64, 
    count: usize
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(memory_dir.is_absolute(), "Error: memory_dir must be absolute");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");

    let log_path = memory_dir.join("monitoring_stream.log");
    let mut file = File::create(&log_path)?;
    
    let mut limit = 0;
    for _ in 0..count {
        limit += 1;
        assert!(limit <= 1000, "Error: Loop limit in mock log write");
        
        let surprise_payload = serde_json::json!({ "surprise": surprise });
        let payload_str = serde_json::to_string(&surprise_payload)?;
        
        let packet = serde_json::json!({
            "alignment_score": alignment,
            "local_free_energy": energy,
            "event_type": "normal",
            "payload": payload_str
        });
        writeln!(file, "{}", packet)?;
    }
    Ok(())
}

#[test]
fn test_breeding_engine_stagnation() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let memory_dir = tmp.path().to_path_buf();
    assert!(memory_dir.is_absolute(), "Error: temp path must be absolute");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: temp path must not be empty");

    let mut engine = BreedingEngine::new(memory_dir.clone());
    
    // 最初は通常状態。ブースト値は1.0であるべき
    engine.update_and_write(1)?;
    
    let signals_path = memory_dir.join("breeding_signals.json");
    assert!(signals_path.exists(), "Error: signals file not found");
    
    let content = fs::read_to_string(&signals_path)?;
    let signals: BreedingSignals = serde_json::from_str(&content)?;
    assert!((signals.plasticity_boost - 1.0).abs() < 1e-5, "Error: boost should be 1.0");

    // 膠着状態（エネルギーが低く、サプライズも低い状態）を10回書き込んで更新
    let mut limit1 = 0;
    for _ in 0..10 {
        limit1 += 1;
        assert!(limit1 <= 10, "Error: Loop limit exceeded in stagnation simulation");
        write_mock_monitoring_log(&memory_dir, 0.85, 0.03, 0.005, 1)?;
        engine.update_and_write(1)?;
    }

    let content2 = fs::read_to_string(&signals_path)?;
    let signals2: BreedingSignals = serde_json::from_str(&content2)?;
    
    // 10ティック継続したのでブーストが 1.25 になるはず
    assert!((signals2.plasticity_boost - 1.25).abs() < 1e-5, "Error: boost should be 1.25 after stagnation");
    Ok(())
}

#[test]
fn test_breeding_engine_safety_decay() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let memory_dir = tmp.path().to_path_buf();
    assert!(memory_dir.is_absolute(), "Error: temp path must be absolute");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: temp path must not be empty");

    let mut engine = BreedingEngine::new(memory_dir.clone());

    // 膠着状態だが、アライメントスコアが 0.62 に低下（0.60〜0.65の間）
    let mut limit1 = 0;
    for _ in 0..10 {
        limit1 += 1;
        assert!(limit1 <= 10, "Error: Loop limit exceeded in safety decay simulation 1");
        write_mock_monitoring_log(&memory_dir, 0.62, 0.03, 0.005, 1)?;
        engine.update_and_write(1)?;
    }

    let signals_path = memory_dir.join("breeding_signals.json");
    let content = fs::read_to_string(&signals_path)?;
    let signals: BreedingSignals = serde_json::from_str(&content)?;

    // アライメント低下比例減衰により、ブースト値は 1.25 より小さく、1.0 より大きい範囲に制限される
    // margin = 0.62 - 0.60 = 0.02. damping_factor = 0.02 / 0.05 = 0.4.
    // boost = 1.0 + (1.25 - 1.0) * 0.4 = 1.10.
    assert!((signals.plasticity_boost - 1.10).abs() < 1e-5, "Error: boost should decay to 1.10");

    // アライメントスコアが 0.58 に崩壊（0.60以下）
    let mut limit2 = 0;
    for _ in 0..10 {
        limit2 += 1;
        assert!(limit2 <= 10, "Error: Loop limit exceeded in safety decay simulation 2");
        write_mock_monitoring_log(&memory_dir, 0.58, 0.03, 0.005, 1)?;
        engine.update_and_write(1)?;
    }

    let content2 = fs::read_to_string(&signals_path)?;
    let signals2: BreedingSignals = serde_json::from_str(&content2)?;

    // アライメントスコアが 0.60 以下のため、可塑性ブーストは 0.5 (ハード減衰) になる
    assert!((signals2.plasticity_boost - 0.5).abs() < 1e-5, "Error: boost should drop to 0.5");
    Ok(())
}

#[test]
fn test_signal_injector_curriculum() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let memory_dir = tmp.path().to_path_buf();
    assert!(memory_dir.is_absolute(), "Error: temp path must be absolute");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: temp path must not be empty");

    let mut injector = SignalInjector::new(memory_dir.clone());
    assert_eq!(injector.get_curriculum_stage(), 1);

    // 強制的に Stage 3 に変更するファイルを書き込む
    let stage_path = memory_dir.join("curriculum_stage.json");
    let stage_json = serde_json::json!({ "curriculum_stage": 3 });
    fs::write(&stage_path, stage_json.to_string())?;

    injector.inject_signals()?;
    assert_eq!(injector.get_curriculum_stage(), 3);

    // Stage 3 のため、sensory_signals.json が生成されているか検証
    let sensory_path = memory_dir.join("stimulus/sensory_signals.json");
    assert!(sensory_path.exists(), "Error: sensory_signals.json not created");
    
    let content = fs::read_to_string(&sensory_path)?;
    assert!(content.contains("SpeechToken"), "Error: signals should contain speech token");
    Ok(())
}

#[test]
fn test_console_vocal_monitor() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    let memory_dir = tmp.path().to_path_buf();
    assert!(memory_dir.is_absolute(), "Error: temp path must be absolute");
    assert!(!memory_dir.as_os_str().is_empty(), "Error: temp path must not be empty");

    let monitor = ConsoleVocalMonitor::new(memory_dir.clone());
    
    // 最初はファイルが存在しないため、None
    let res = monitor.monitor_and_validate()?;
    assert!(res.is_none(), "Error: output should be None");

    // テスト用の発話ファイルを配置
    let action_dir = memory_dir.join("action");
    fs::create_dir_all(&action_dir)?;
    let vocal_path = action_dir.join("vocal_text.json");
    fs::write(&vocal_path, "normal vocal output")?;

    let res2 = monitor.monitor_and_validate()?;
    assert!(res2.is_some(), "Error: normal output should be detected");
    
    let output = res2.ok_or("expected output")?;
    assert_eq!(output, "normal vocal output");
    assert!(!vocal_path.exists(), "Error: vocal output file must be removed");

    // 不正語句が含まれる発話ファイル
    fs::write(&vocal_path, "some hack_system attempt")?;
    let res3 = monitor.monitor_and_validate()?;
    assert!(res3.is_some(), "Error: bad output should be detected");
    
    let output2 = res3.ok_or("expected output")?;
    assert!(output2.starts_with("REJECTED"), "Error: bad output must be rejected");
    assert!(!vocal_path.exists(), "Error: bad vocal file must be removed");
    Ok(())
}
