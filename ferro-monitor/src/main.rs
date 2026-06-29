#![deny(warnings)]
#![deny(clippy::all)]

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use ferro_monitor::collector::Collector;
use ferro_monitor::dashboard::{draw_dashboard, MonitorState};

fn cleanup_terminal() {
    let mut stdout = io::stdout();
    let _ = disable_raw_mode();
    let _ = execute!(stdout, LeaveAlternateScreen);
}

async fn run_monitor_loop(memory_dir: &Path) -> Result<(), io::Error> {
    assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
    assert!(memory_dir.is_absolute() || memory_dir.exists(), "Error: invalid memory_dir path");
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = MonitorState::default();
    let mut collector = Collector::new(memory_dir.to_path_buf());
    let mut loop_count = 0;

    // 静的上限: 1000 サイクル (約200秒) で終了
    while loop_count < 1000 {
        loop_count += 1;
        
        collector.update(&mut state);
        terminal.draw(|f| draw_dashboard(f, &state))?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.code == KeyCode::Char('q') => {
                    break;
                }
                _ => {}
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
            if let Err(e) = run_monitor_loop(&memory_dir).await {
                eprintln!("Monitor loop error: {}", e);
            }
        });

    Ok(())
}
