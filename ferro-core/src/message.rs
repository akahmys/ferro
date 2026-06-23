use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteroceptiveSignal {
    CpuTemp(f32),
    RamFree(u64),
    DiskIo(f64),
    ProcessError(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorySignal {
    FrameDelta(f64),
    ImageEmbedding(Vec<f32>),
    Mfcc(Vec<f32>),
    SpeechToken(Vec<String>),
    LogHash(u64),
    ProprioceptiveEcho(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorCommand {
    pub origin_cluster_id: String,
    pub target_path: String,
    pub payload: Vec<u8>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfferenceCopy {
    pub timestamp: u64,
    pub command_hash: u64,
    pub origin_cluster_id: String,
    pub expected_tokens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryMuteCommand {
    pub mute: bool,
    pub attenuation_db: f32,
}
