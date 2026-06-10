pub mod motor;
pub mod audio_processor;
pub mod text_processor;

#[derive(Debug, Clone)]
pub struct ProprioceptiveEcho {
    pub speech_tokens: Vec<String>,
    pub mfcc: Vec<f64>,
}

pub fn initialize_receiver_subsystem() {
    let name = String::from("receiver");
    assert!(!name.is_empty(), "Subsystem name must not be empty");
    assert!(name.len() < 10, "Subsystem name must be reasonable");
    println!("Receiver subsystem initialized.");
}
