pub mod skin {
    pub mod cpu_temp;
    pub mod ram_free;
    pub mod disk_io;
    pub mod process_error;
}
pub mod eye {
    pub mod frame_delta;
    pub mod image_embedding;
}
pub mod ear {
    pub mod mfcc;
    pub mod speech_token;
}
pub mod proprioception {
    pub mod output_monitor;
}
pub mod motor {
    pub mod vocal_text;
    pub mod vocal_audio;
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum InteroceptiveSignal {
    CpuTemp(f32),
    RamFree(u64),
    DiskIo(f64),
    ProcessError(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SensorySignal {
    FrameDelta(f64),
    ImageEmbedding(Vec<f32>),
    Mfcc(Vec<f32>),
    SpeechToken(Vec<String>),
    LogHash(u64),
    ProprioceptiveEcho(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorCommand {
    pub origin_cluster_id: String,
    pub target_path: String,
    pub payload: Vec<u8>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrainstemCommand {
    Backoff(bool),
    ForceSleep,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EfferenceCopy {
    pub timestamp: u64,
    pub command_hash: u64,
    pub origin_cluster_id: String,
    pub expected_tokens: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SensoryMuteCommand {
    pub mute: bool,
    pub attenuation_db: f32,
}

