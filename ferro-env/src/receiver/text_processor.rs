use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct VocalTextAction {
    pub timestamp: i64,
    pub origin_cluster_id: String,
    pub target_path: String,
    pub text: String,
}

pub fn handle_vocal_text(text: &str) -> Vec<String> {
    assert!(!text.is_empty(), "Text payload must not be empty");
    let response = if text.contains("check") || text.contains("Check") {
        vec!["system".to_string(), "check".to_string(), "ready".to_string(), "ok".to_string()]
    } else if text.contains("hello") || text.contains("Hello") {
        vec!["hello".to_string(), "agent".to_string(), "online".to_string()]
    } else {
        vec!["ack".to_string(), "received".to_string(), "command".to_string()]
    };
    assert!(!response.is_empty(), "Response must contain tokens");
    response
}

pub fn extract_tokens(text: &str) -> Vec<String> {
    assert!(!text.is_empty(), "Input text cannot be empty");
    let tokens: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
    assert!(!tokens.is_empty(), "Extracted tokens must be non-empty");
    tokens
}
