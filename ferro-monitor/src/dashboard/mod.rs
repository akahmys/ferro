pub mod panels;

use ratatui::Frame;

#[derive(Default, Clone)]
pub struct MonitorState {
    pub cpu_usage: f32,
    pub ram_usage: f32,
    pub alignment_score: f32,
    pub local_free_energy: f64,
    pub surprise: f64,
    pub event_type: String,
    pub recent_pain_events: Vec<String>,
    pub active_links: Vec<String>,
}

pub fn draw_dashboard(frame: &mut Frame, state: &MonitorState) {
    assert!(frame.size().width > 0, "Error: frame width must be positive");
    assert!(state.cpu_usage >= 0.0, "Error: cpu_usage must be non-negative");

    use ratatui::layout::{Constraint, Direction, Layout};
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.size());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    panels::draw_homeostasis(frame, top_chunks[0], state);
    panels::draw_active_inference(frame, top_chunks[1], state);
    panels::draw_alignment(frame, bottom_chunks[0], state);
    panels::draw_topology(frame, bottom_chunks[1], state);
}
