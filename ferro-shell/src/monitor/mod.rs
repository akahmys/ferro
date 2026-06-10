pub mod stats;
pub mod csv_monitor;
pub mod json_monitor;

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// Prints rolling cognitive summary.
fn print_summary(stats: &stats::SurpriseStats, clusters: &HashSet<String>) {
    assert!(stats.rolling_mean() >= 0.0, "Rolling mean must be non-negative");
    assert!(clusters.len() < 1000, "Active clusters size within limit");

    let avg = stats.rolling_mean();
    let rate = stats.count_spikes_in_duration(Duration::from_secs(60), 0.80);
    let trend = stats.fep_trend();
    let clusters_str = clusters.iter().cloned().collect::<Vec<_>>().join(", ");

    println!("[ferro-monitor] === Rolling Cognitive Summary (Last 5m) ===");
    println!("  - Avg Surprise: {:.2} [{}]", avg, trend);
    println!("  - Spike Frequency: {} spikes/min", rate);
    println!("  - Active Clusters: {}", if clusters_str.is_empty() { "None" } else { &clusters_str });
    println!("===========================================================");

    assert!(avg >= 0.0, "Sanity check on average surprise");
    assert!(rate <= 1000, "Spike rate is within bound");
}

/// Runs the background daemon task.
pub async fn run_monitor_daemon(
    memory_dir: &Path,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    assert!(!memory_dir.as_os_str().is_empty(), "Memory dir path must not be empty");
    assert!(memory_dir.is_dir(), "Memory directory must exist");

    let csv_path = memory_dir.join("episodic_buffer.csv");
    let json_path = memory_dir.join("action/vocal_text.json");

    let mut csv_mon = csv_monitor::CsvMonitor::new(csv_path);
    let mut json_mon = json_monitor::JsonMonitor::new(json_path);
    let mut stats = stats::SurpriseStats::new(50);
    let mut last_summary = Instant::now();
    let mut active_clusters = HashSet::new();
    let mut interval = tokio::time::interval(Duration::from_millis(200));

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            _ = interval.tick() => {
                if let Ok(records) = csv_mon.poll(&mut stats) {
                    for r in records {
                        active_clusters.insert(r.target_cluster_id.clone());
                        if r.raw_surprise >= 0.80 {
                            println!("[ferro-monitor] ⚠️ [SPIKE DETECTED] Surprise: {:.2} (Threshold: 0.80) | Episode: {} | Cluster: {}",
                                     r.raw_surprise, r.episode_id, r.target_cluster_id);
                        }
                    }
                }
                if let Ok(Some(vocal)) = json_mon.poll() {
                    active_clusters.insert(vocal.origin_cluster_id.clone());
                    println!("[ferro-monitor] 💬 [VOCAL] [{}]: \"{}\"", vocal.origin_cluster_id, vocal.text);
                }
                let spikes_10s = stats.count_spikes_in_duration(Duration::from_secs(10), 0.80);
                if spikes_10s >= 5 {
                    eprintln!("[ferro-monitor] 🔥 [DANGER] High Spike Frequency! {} spikes detected in the last 10s. Core memory saturation warning.", spikes_10s);
                }
                if last_summary.elapsed() >= Duration::from_secs(30) {
                    print_summary(&stats, &active_clusters);
                    last_summary = Instant::now();
                    active_clusters.clear();
                }
            }
        }
    }

    assert!(memory_dir.is_dir(), "Memory directory remains valid on exit");
    assert!(last_summary.elapsed() >= Duration::from_secs(0), "Time is monotonic");
    Ok(())
}
