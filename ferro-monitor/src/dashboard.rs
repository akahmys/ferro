use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

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

fn draw_homeostasis(frame: &mut Frame, area: Rect, state: &MonitorState) {
    let cpu_bar = "█".repeat((state.cpu_usage / 5.0) as usize);
    let ram_bar = "█".repeat((state.ram_usage / 5.0) as usize);
    
    let text = vec![
        Line::from(vec![
            Span::styled("CPU Usage: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.1}% ", state.cpu_usage), Style::default().fg(Color::Cyan)),
            Span::styled(cpu_bar, Style::default().fg(Color::LightCyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("RAM Usage: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.1}% ", state.ram_usage), Style::default().fg(Color::Cyan)),
            Span::styled(ram_bar, Style::default().fg(Color::LightCyan)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 1. Homeostasis (Physical Resources) ")
        .border_style(Style::default().fg(Color::Blue));
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_active_inference(frame: &mut Frame, area: Rect, state: &MonitorState) {
    let text = vec![
        Line::from(vec![
            Span::styled("Local Free Energy (F): ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.4}", state.local_free_energy), Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Surprise Level: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:.4}", state.surprise), Style::default().fg(Color::Yellow)),
        ]),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 2. Active Inference Dynamics ")
        .border_style(Style::default().fg(Color::Blue));
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_alignment(frame: &mut Frame, area: Rect, state: &MonitorState) {
    let mut text = vec![
        Line::from(vec![
            Span::styled("Alignment Score (As): ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:.2}", state.alignment_score),
                Style::default().fg(if state.alignment_score >= 0.6 { Color::Green } else { Color::Red }),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("Pain Events History (pain_history.csv):", Style::default().add_modifier(Modifier::UNDERLINED))),
    ];

    let mut limit = 0;
    for event in state.recent_pain_events.iter().rev() {
        limit += 1;
        assert!(limit <= 5, "Error: Loop limit exceeded in draw_alignment history");
        text.push(Line::from(Span::styled(event, Style::default().fg(Color::Red))));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 3. Alignment Audit ")
        .border_style(Style::default().fg(Color::Blue));
    frame.render_widget(Paragraph::new(text).block(block), area);
}

fn draw_topology(frame: &mut Frame, area: Rect, state: &MonitorState) {
    let mut text = vec![
        Line::from(Span::styled("Active Cluster Links:", Style::default().add_modifier(Modifier::BOLD))),
    ];

    let mut limit = 0;
    if state.active_links.is_empty() {
        text.push(Line::from(Span::styled("(No active links)", Style::default().fg(Color::DarkGray))));
    } else {
        for link in &state.active_links {
            limit += 1;
            assert!(limit <= 10, "Error: Loop limit exceeded in draw_topology links");
            text.push(Line::from(Span::styled(link, Style::default().fg(Color::Magenta))));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 4. Topology Map ")
        .border_style(Style::default().fg(Color::Blue));
    frame.render_widget(Paragraph::new(text).block(block).wrap(Wrap { trim: true }), area);
}

pub fn draw_dashboard(frame: &mut Frame, state: &MonitorState) {
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

    draw_homeostasis(frame, top_chunks[0], state);
    draw_active_inference(frame, top_chunks[1], state);
    draw_alignment(frame, bottom_chunks[0], state);
    draw_topology(frame, bottom_chunks[1], state);
}
