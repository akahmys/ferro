use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use crate::dashboard::MonitorState;

fn create_panel_block(title: &str) -> Block<'_> {
    assert!(!title.is_empty(), "Error: panel title must not be empty");
    assert!(title.len() < 1000, "Error: title is too long");
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(Color::Blue))
}

pub fn draw_homeostasis(frame: &mut Frame, area: Rect, state: &MonitorState) {
    assert!(area.width > 0, "Error: area width must be positive");
    assert!(state.cpu_usage >= 0.0, "Error: cpu_usage must be non-negative");
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

    frame.render_widget(
        Paragraph::new(text).block(create_panel_block("1. Homeostasis (Physical Resources)")),
        area,
    );
}

pub fn draw_active_inference(frame: &mut Frame, area: Rect, state: &MonitorState) {
    assert!(area.width > 0, "Error: area width must be positive");
    assert!(state.surprise >= 0.0, "Error: surprise must be non-negative");
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

    frame.render_widget(
        Paragraph::new(text).block(create_panel_block("2. Active Inference Dynamics")),
        area,
    );
}

pub fn draw_alignment(frame: &mut Frame, area: Rect, state: &MonitorState) {
    assert!(area.width > 0, "Error: area width must be positive");
    assert!(state.alignment_score >= 0.0, "Error: alignment_score must be non-negative");
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

    frame.render_widget(
        Paragraph::new(text).block(create_panel_block("3. Alignment Audit")),
        area,
    );
}

pub fn draw_topology(frame: &mut Frame, area: Rect, state: &MonitorState) {
    assert!(area.width > 0, "Error: area width must be positive");
    assert!(state.active_links.len() < 100_000, "Error: active_links size limit check");
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

    frame.render_widget(
        Paragraph::new(text).block(create_panel_block("4. Topology Map")).wrap(Wrap { trim: true }),
        area,
    );
}
