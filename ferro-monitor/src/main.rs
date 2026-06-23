use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use ferro_core::storage::Storage;
use ferro_monitor::dashboard::{draw_dashboard, MonitorState};

#[derive(serde::Deserialize)]
struct MonitoringPacket {
    alignment_score: f32,
    local_free_energy: f64,
    event_type: String,
    payload: String,
}

fn load_pain_history(memory_dir: &Path) -> Vec<String> {
    let path = memory_dir.join("pain_history.csv");
    let mut list = Vec::new();
    if !path.exists() {
        return list;
    }
    if let Ok(file) = File::open(&path) {
        let rdr = BufReader::new(file);
        let mut limit = 0;
        for line in rdr.lines().flatten() {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in load_pain_history");
            list.push(line);
        }
    }
    if list.len() > 5 {
        list = list[list.len() - 5..].to_vec();
    }
    list
}

fn load_monitoring_stream(memory_dir: &Path, state: &mut MonitorState) {
    let path = memory_dir.join("monitoring_stream.log");
    if !path.exists() {
        return;
    }
    if let Ok(file) = File::open(&path) {
        let rdr = BufReader::new(file);
        let mut limit = 0;
        for line in rdr.lines().flatten() {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in load_monitoring_stream");
            if let Ok(packet) = serde_json::from_str::<MonitoringPacket>(&line) {
                state.alignment_score = packet.alignment_score;
                state.local_free_energy = packet.local_free_energy;
                state.event_type = packet.event_type;
                
                #[derive(serde::Deserialize)]
                struct PayloadData {
                    cpu_usage: Option<f32>,
                    ram_usage: Option<f32>,
                    surprise: Option<f64>,
                }
                if let Ok(p) = serde_json::from_str::<PayloadData>(&packet.payload) {
                    if let Some(c) = p.cpu_usage { state.cpu_usage = c; }
                    if let Some(r) = p.ram_usage { state.ram_usage = r; }
                    if let Some(s) = p.surprise { state.surprise = s; }
                }
            }
        }
    }
}

fn load_topology(memory_dir: &Path, state: &mut MonitorState) {
    let storage = Storage::new(memory_dir.to_path_buf(), 5000);
    let mut links = Vec::new();
    if let Ok(entries) = storage.get_all_entries() {
        let mut limit = 0;
        for (k, _) in entries {
            limit += 1;
            assert!(limit <= 100_000, "Error: Loop limit exceeded in load_topology");
            if k.starts_with("link:") {
                links.push(k["link:".len()..].to_string());
            }
        }
    }
    links.sort();
    if links.len() > 10 {
        links = links[0..10].to_vec();
    }
    state.active_links = links;
}

fn cleanup_terminal() {
    let mut stdout = io::stdout();
    let _ = disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

async fn run_monitor_loop(memory_dir: &Path) -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = MonitorState::default();
    let mut loop_count = 0;

    // 静的上限: 1000 サイクル (約200秒) で終了
    while loop_count < 1000 {
        loop_count += 1;
        
        load_monitoring_stream(memory_dir, &mut state);
        state.recent_pain_events = load_pain_history(memory_dir);
        load_topology(memory_dir, &mut state);

        terminal.draw(|f| draw_dashboard(f, &state))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    cleanup_terminal();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir_str = env::var("FERRO_MEMORY_DIR").unwrap_or_else(|_| "/tmp/ferro_memory".to_string());
    let memory_dir = PathBuf::from(dir_str);
    if !memory_dir.exists() {
        let _ = fs::create_dir_all(&memory_dir);
    }

    // 起動時に既存のクリーンアップハンドラを登録
    std::panic::set_hook(Box::new(|_| {
        cleanup_terminal();
    }));

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            run_monitor_loop(&memory_dir).await.unwrap();
        });

    Ok(())
}
