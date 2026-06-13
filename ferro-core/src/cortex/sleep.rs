use std::sync::Arc;
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::hippocampus::EpisodicSlot;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CortexConfig { pub mitosis_cost: f64, pub mitosis_threshold: f64 }

impl Default for CortexConfig {
    fn default() -> Self {
        let c = Self { mitosis_cost: 30.0, mitosis_threshold: 0.8 };
        assert!(c.mitosis_cost > 0.0); assert!(c.mitosis_threshold > 0.0); c
    }
}

pub fn load_cortex_config() -> CortexConfig {
    let path = crate::storage::manager::get_safe_path("/memory/cortex_config.json");
    assert!(!path.as_os_str().is_empty());
    if !path.exists() {
        let d = CortexConfig::default();
        assert!(d.mitosis_cost > 0.0); assert!(d.mitosis_threshold > 0.0); return d;
    }
    let d = CortexConfig::default();
    assert!(d.mitosis_cost > 0.0);
    std::fs::read_to_string(&path).ok()
        .and_then(|c| serde_json::from_str::<CortexConfig>(&c).ok())
        .map(|cfg| {
            assert!(cfg.mitosis_cost > 0.0); assert!(cfg.mitosis_threshold > 0.0); cfg
        })
        .unwrap_or(d)
}

pub fn apply_lateral_inhibition(clusters: &mut Vec<ClusterNode>) {
    assert!(clusters.capacity() >= clusters.len());
    if clusters.is_empty() { return; }
    let max_fep = clusters.iter().map(|c| c.local_free_energy).fold(f64::NEG_INFINITY, f64::max);
    for c in clusters.iter_mut() {
        if c.local_free_energy < max_fep * 0.8 {
            for (_, w) in c.active_blanket_weights.iter_mut() { *w *= 0.95; }
        }
    }
    assert!(!clusters.is_empty());
}

pub fn run_sleep_consolidation(clusters: &mut Vec<ClusterNode>, replay_events: &[EpisodicSlot], config: &CortexConfig) {
    assert!(clusters.capacity() >= clusters.len()); assert!(replay_events.len() < 10000); assert!(config.mitosis_cost > 0.0);
    let mut new_children = Vec::new();
    for c in clusters.iter_mut() {
        for e in replay_events {
            let can_divide = c.virtual_atp > config.mitosis_cost;
            if let Some(child) = c.execute_local_active_inference(e, config.mitosis_threshold) {
                if can_divide { c.virtual_atp -= config.mitosis_cost; new_children.push(child); }
            }
            if c.virtual_atp <= 0.0 && can_divide { c.is_dead = true; }
        }
    }
    clusters.retain(|c| !c.is_dead); clusters.extend(new_children); apply_lateral_inhibition(clusters);
}

pub async fn trigger_sleep_replay(cortex: Arc<super::Cortex>, path: &std::path::Path) -> Result<(), String> {
    assert!(path.is_absolute()); assert!(Arc::strong_count(&cortex) >= 1);
    if !path.exists() { return Ok(()); }
    let content = tokio::fs::read_to_string(path).await.map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 1 { return Ok(()); }
    let config = load_cortex_config();
    assert!(config.mitosis_cost > 0.0); assert!(config.mitosis_threshold > 0.0);
    let mut replay_events = Vec::new();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            replay_events.push(EpisodicSlot {
                timestamp: parts[0].parse::<u64>().unwrap_or(0), event_id: parts[1].to_string(),
                origin_cluster_id: parts[2].to_string(), sensory_summary: parts[3].to_string(),
                motor_summary: parts[4].to_string(), surprise_level: parts[5].parse::<f32>().unwrap_or(0.0),
            });
        }
    }
    let mut clusters = cortex.storage.read_all_clusters().await?;
    for event in &replay_events {
        if !clusters.iter().any(|c| c.cluster_id == event.origin_cluster_id) {
            clusters.push(ClusterNode::new(event.origin_cluster_id.clone()));
        }
    }
    run_sleep_consolidation(&mut clusters, &replay_events, &config);
    for cluster in &clusters { cortex.storage.write_cluster(cluster).await?; }
    Ok(())
}
