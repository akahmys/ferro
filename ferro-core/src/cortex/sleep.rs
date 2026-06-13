use std::sync::Arc;
use crate::cortex::dynamic_cluster::ClusterNode;
use crate::hippocampus::EpisodicSlot;

pub const MITOSIS_COST: f64 = 30.0;

pub fn apply_lateral_inhibition(clusters: &mut Vec<ClusterNode>) {
    assert!(clusters.capacity() >= clusters.len(), "Clusters capacity safety check");
    if clusters.is_empty() { return; }

    let max_fep = clusters.iter()
        .map(|c| c.local_free_energy)
        .fold(f64::NEG_INFINITY, f64::max);

    for cluster in clusters.iter_mut() {
        if cluster.local_free_energy < max_fep * 0.8 {
            for (_, weight) in cluster.active_blanket_weights.iter_mut() {
                *weight *= 0.95;
            }
        }
    }
    assert!(!clusters.is_empty(), "Post-condition check");
}

pub fn run_sleep_consolidation(
    clusters: &mut Vec<ClusterNode>,
    replay_events: &[EpisodicSlot],
) {
    assert!(clusters.capacity() >= clusters.len(), "Clusters capacity check");
    assert!(replay_events.len() < 10000, "Events size check");
    let mut new_children: Vec<ClusterNode> = Vec::new();

    for cluster in clusters.iter_mut() {
        for event in replay_events {
            let can_divide = cluster.virtual_atp > MITOSIS_COST;

            if let Some(child) = cluster.execute_local_active_inference(event) {
                if can_divide {
                    cluster.virtual_atp -= MITOSIS_COST;
                    new_children.push(child);
                }
            }

            if cluster.virtual_atp <= 0.0 {
                cluster.is_dead = true;
            }
        }
    }

    clusters.retain(|c| !c.is_dead);
    clusters.extend(new_children);
    apply_lateral_inhibition(clusters);
    assert!(clusters.capacity() >= clusters.len(), "Capacity safety check");
}

pub async fn trigger_sleep_replay(cortex: Arc<super::Cortex>, path: &std::path::Path) -> Result<(), String> {
    assert!(path.is_absolute(), "Path absolute check");
    assert!(Arc::strong_count(&cortex) >= 1, "Cortex reference check");
    if !path.exists() { return Ok(()); }
    let content = tokio::fs::read_to_string(path).await.map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= 1 { return Ok(()); }

    let mut replay_events = Vec::new();
    for line in lines.iter().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 6 {
            let timestamp = parts[0].parse::<u64>().unwrap_or(0);
            let event_id = parts[1].to_string();
            let origin_cluster_id = parts[2].to_string();
            let sensory_summary = parts[3].to_string();
            let motor_summary = parts[4].to_string();
            let surprise_level = parts[5].parse::<f32>().unwrap_or(0.0);
            let slot = EpisodicSlot {
                timestamp, event_id, origin_cluster_id,
                sensory_summary, motor_summary, surprise_level,
            };
            replay_events.push(slot);
        }
    }

    let mut clusters = cortex.storage.read_all_clusters().await?;

    // Dynamically insert missing parent clusters before consolidation
    for event in &replay_events {
        if !clusters.iter().any(|c| c.cluster_id == event.origin_cluster_id) {
            let new_parent = ClusterNode::new(event.origin_cluster_id.clone());
            clusters.push(new_parent);
        }
    }

    run_sleep_consolidation(&mut clusters, &replay_events);

    for cluster in &clusters {
        cortex.storage.write_cluster(cluster).await?;
    }

    Ok(())
}
