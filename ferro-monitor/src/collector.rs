use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::Instant;

use ferro_core::storage::Storage;
use crate::dashboard::MonitorState;

#[derive(serde::Deserialize)]
struct MonitoringPacket {
    alignment_score: f32,
    local_free_energy: f64,
    event_type: String,
    payload: String,
}

pub struct Collector {
    memory_dir: PathBuf,
    stream_offset: u64,
    pain_offset: u64,
    last_topology_read: Option<Instant>,
}

impl Collector {
    pub fn new(memory_dir: PathBuf) -> Self {
        assert!(!memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        assert!(memory_dir.starts_with(".") || memory_dir.starts_with("/") || memory_dir.starts_with("/tmp"), "Error: path must be valid");
        Self {
            memory_dir,
            stream_offset: 0,
            pain_offset: 0,
            last_topology_read: None,
        }
    }

    pub fn update(&mut self, state: &mut MonitorState) {
        assert!(state.cpu_usage >= 0.0, "Error: monitor state must be initialized");
        assert!(state.recent_pain_events.len() <= 1000, "Error: pain events history limits");
        self.load_monitoring_stream(state);
        self.load_pain_history(state);
        self.load_topology(state);
    }

    fn load_monitoring_stream(&mut self, state: &mut MonitorState) {
        assert!(!self.memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        assert!(state.cpu_usage >= 0.0, "Error: state cpu usage must be initialized");

        let path = self.memory_dir.join("monitoring_stream.log");
        if !path.exists() {
            return;
        }
        let file_res = File::open(&path);
        if let Ok(mut file) = file_res {
            if file.metadata().is_ok_and(|meta| meta.len() < self.stream_offset) {
                self.stream_offset = 0;
            }
            if file.seek(SeekFrom::Start(self.stream_offset)).is_err() {
                return;
            }
            let rdr = BufReader::new(&mut file);
            let mut limit = 0;
            for line in rdr.lines().map_while(Result::ok) {
                limit += 1;
                assert!(limit <= 10_000, "Error: Loop limit exceeded in load_monitoring_stream");
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
            if let Ok(pos) = file.stream_position() {
                self.stream_offset = pos;
            }
        }
    }

    fn load_pain_history(&mut self, state: &mut MonitorState) {
        assert!(!self.memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        assert!(state.recent_pain_events.len() <= 1000, "Error: pain events history bounds");

        let path = self.memory_dir.join("pain_history.csv");
        if !path.exists() {
            return;
        }
        let file_res = File::open(&path);
        if let Ok(mut file) = file_res {
            if file.metadata().is_ok_and(|meta| meta.len() < self.pain_offset) {
                self.pain_offset = 0;
            }
            if file.seek(SeekFrom::Start(self.pain_offset)).is_err() {
                return;
            }
            let rdr = BufReader::new(&mut file);
            let mut limit = 0;
            for line in rdr.lines().map_while(Result::ok) {
                limit += 1;
                assert!(limit <= 10_000, "Error: Loop limit exceeded in load_pain_history");
                state.recent_pain_events.push(line);
            }
            if let Ok(pos) = file.stream_position() {
                self.pain_offset = pos;
            }
        }
        if state.recent_pain_events.len() > 5 {
            let start = state.recent_pain_events.len() - 5;
            state.recent_pain_events = state.recent_pain_events[start..].to_vec();
        }
    }

    fn load_topology(&mut self, state: &mut MonitorState) {
        assert!(!self.memory_dir.as_os_str().is_empty(), "Error: memory_dir must not be empty");
        assert!(state.active_links.len() <= 1000, "Error: active links bounds");

        if self.last_topology_read.is_some_and(|last| last.elapsed() < std::time::Duration::from_secs(1)) {
            return;
        }
        self.last_topology_read = Some(Instant::now());

        let storage_res = Storage::new_readonly(self.memory_dir.clone());
        if let Ok(storage) = storage_res {
            let mut links = Vec::new();
            if let Ok(entries) = storage.get_all_entries() {
                let mut limit = 0;
                for (k, _) in entries {
                    limit += 1;
                    assert!(limit <= 100_000, "Error: Loop limit exceeded in load_topology");
                    if let Some(stripped) = k.strip_prefix("link:") {
                        links.push(stripped.to_string());
                    }
                }
            }
            links.sort();
            if links.len() > 10 {
                links = links[0..10].to_vec();
            }
            state.active_links = links;
        }
    }
}
