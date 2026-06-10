use serde::Deserialize;
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct VocalAudioAction {
    pub timestamp: i64,
    pub origin_cluster_id: String,
    pub pcm_payload_base64: String,
    pub sample_rate: u32,
    pub channels: u32,
}

pub fn decode_and_compute_mfcc(action: &VocalAudioAction) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    assert!(action.sample_rate > 0, "Sample rate must be positive");
    assert!(action.channels > 0, "Channels must be positive");

    let bytes = STANDARD.decode(&action.pcm_payload_base64)?;
    assert!(bytes.len() % 2 == 0, "PCM bytes length must be even");

    let samples: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let mfcc = compute_mfcc(&samples, action.sample_rate, action.channels);
    Ok(mfcc)
}

fn compute_mfcc(samples: &[i16], sample_rate: u32, channels: u32) -> Vec<f64> {
    assert!(sample_rate > 0, "Sample rate must be positive");
    assert!(channels > 0, "Channels must be positive");

    let n = samples.len();
    if n == 0 {
        return vec![0.0; 5];
    }

    let x: Vec<f64> = samples.iter().map(|&s| s as f64 / 32768.0).collect();

    // 1. RMS
    let sum_sq: f64 = x.iter().map(|&val| val * val).sum();
    let rms = (sum_sq / n as f64).sqrt();
    assert!((0.0..=1.0).contains(&rms), "RMS out of bounds");

    // 2. ZCR
    let mut zcr = 0.0;
    if n > 1 {
        let mut zero_crossings = 0;
        for i in 1..n {
            let s1 = if x[i] >= 0.0 { 1 } else { -1 };
            let s2 = if x[i - 1] >= 0.0 { 1 } else { -1 };
            if s1 != s2 {
                zero_crossings += 1;
            }
        }
        zcr = zero_crossings as f64 / (n - 1) as f64;
    }
    assert!((0.0..=1.0).contains(&zcr), "ZCR out of bounds");

    // 3. HFD
    let mut hfd = 0.0;
    if n > 1 {
        let mut diff_sum = 0.0;
        for i in 1..n {
            diff_sum += (x[i] - x[i - 1]).abs();
        }
        hfd = diff_sum / (n - 1) as f64;
    }
    assert!((0.0..=2.0).contains(&hfd), "HFD out of bounds");

    // 4. ASYM
    let sum_cube: f64 = x.iter().map(|&val| val * val * val).sum();
    let asym = sum_cube / n as f64;
    assert!((-1.0..=1.0).contains(&asym), "ASYM out of bounds");

    // 5. DUR
    let pcm_len = n as f64 / channels as f64;
    let dur = (pcm_len / sample_rate as f64).clamp(0.0, 3.0);
    assert!((0.0..=3.0).contains(&dur), "DUR out of bounds");

    let mfcc = vec![rms, zcr, hfd, asym, dur];
    assert_eq!(mfcc.len(), 5, "MFCC dimension must be 5");
    mfcc
}
