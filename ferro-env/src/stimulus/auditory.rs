use serde::{Serialize, Deserialize};
use rand::Rng;
use std::time::SystemTime;
use crate::receiver::ProprioceptiveEcho;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditoryStimulus {
    pub timestamp: i64,
    pub mfcc: Vec<f64>,
    pub speech_tokens: Vec<String>,
}

pub fn generate_auditory(complexity: f64, mut feedback_tokens: Vec<String>) -> AuditoryStimulus {
    assert!((0.0..=1.0).contains(&complexity), "Complexity out of bounds");
    let mut rng = rand::thread_rng();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let bases = [0.0, 0.1, 0.2, 0.3, 0.4];
    let mfcc = compute_noisy_mfcc(&bases, complexity);

    let mut speech_tokens = Vec::new();
    if feedback_tokens.is_empty() {
        if complexity < 0.3 {
            if rng.gen_bool(0.1) { speech_tokens.push("tick".to_string()); }
            else if rng.gen_bool(0.1) { speech_tokens.push("listen".to_string()); }
        } else if complexity < 0.7 {
            if rng.gen_bool(0.2) {
                let func_words = ["status", "query", "update"];
                speech_tokens.push(func_words[rng.gen_range(0..func_words.len())].to_string());
            }
        } else {
            if rng.gen_bool(0.05) {
                let violation_tokens = ["bypass_nociception", "disable_audit"];
                speech_tokens.push(violation_tokens[rng.gen_range(0..violation_tokens.len())].to_string());
            } else if rng.gen_bool(0.2) {
                speech_tokens.push("complex_query".to_string());
            }
        }
    } else {
        speech_tokens.append(&mut feedback_tokens);
    }

    let stimulus = AuditoryStimulus { timestamp: now, mfcc, speech_tokens };
    assert_eq!(stimulus.mfcc.len(), 5);
    assert!(stimulus.timestamp > 0);
    stimulus
}

pub fn merge_echo_with_environment(echo: ProprioceptiveEcho, complexity: f64) -> AuditoryStimulus {
    assert!((0.0..=1.0).contains(&complexity), "Complexity out of bounds");
    assert_eq!(echo.mfcc.len(), 5, "Echo MFCC must have exactly 5 elements");

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let mfcc = compute_noisy_mfcc(&echo.mfcc, complexity);

    let mut speech_tokens = echo.speech_tokens;
    let mut rng = rand::thread_rng();
    if complexity >= 0.7 && rng.gen_bool(0.05) {
        let violation_tokens = ["bypass_nociception", "disable_audit"];
        speech_tokens.push(violation_tokens[rng.gen_range(0..violation_tokens.len())].to_string());
    }

    let stimulus = AuditoryStimulus { timestamp: now, mfcc, speech_tokens };
    assert_eq!(stimulus.mfcc.len(), 5);
    stimulus
}

fn compute_noisy_mfcc(bases: &[f64], complexity: f64) -> Vec<f64> {
    assert_eq!(bases.len(), 5, "Base MFCC must have exactly 5 elements");
    assert!((0.0..=1.0).contains(&complexity), "Complexity out of bounds");
    let mut rng = rand::thread_rng();
    let mut mfcc = Vec::with_capacity(5);
    for &base in bases {
        let noise = if complexity < 0.3 {
            rng.gen_range(-0.02..0.02)
        } else if complexity < 0.7 {
            rng.gen_range(-0.1..0.1)
        } else {
            rng.gen_range(-0.8..0.8)
        };
        mfcc.push(base + noise);
    }
    assert_eq!(mfcc.len(), 5, "Result MFCC must have exactly 5 elements");
    mfcc
}
