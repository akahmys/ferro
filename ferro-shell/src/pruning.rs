use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
pub struct PanicDump {
    pub origin_cluster_id: String,
}

/// Prunes resources based on the contents of `panic_dump.json`.
///
/// # Errors
/// Returns an error if the panic dump file cannot be read or parsed, or if file deletion fails.
pub fn prune_resources(memory_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Rule 5: Assertions for pre-conditions (at least 2 checks)
    assert!(
        !memory_dir.is_empty(),
        "Memory directory path must not be empty"
    );
    assert!(
        Path::new(memory_dir).is_dir(),
        "Memory directory must exist"
    );

    let dump_path = Path::new(memory_dir).join("panic_dump.json");
    if !dump_path.exists() {
        return Ok(());
    }

    let dump_content = fs::read_to_string(&dump_path)?;
    let panic_dump: PanicDump = serde_json::from_str(&dump_content)?;
    let origin_id = &panic_dump.origin_cluster_id;

    // Prune simulated actor files
    let vocal_text_path = Path::new(memory_dir).join("vocal_text.json");
    if vocal_text_path.exists() {
        fs::remove_file(&vocal_text_path)?;
    }

    let cluster_dir = Path::new(memory_dir)
        .join("knowledge_graph")
        .join("clusters");
    let cluster_file_path = cluster_dir.join(format!("{}.json", origin_id));
    if cluster_file_path.exists() {
        fs::remove_file(&cluster_file_path)?;
    }

    // Clean up the panic dump itself
    fs::remove_file(&dump_path)?;

    // Rule 5: Assertions for post-conditions (at least 2 checks)
    assert!(
        !dump_path.exists(),
        "panic_dump.json must be deleted after pruning"
    );
    assert!(
        Path::new(memory_dir).is_dir(),
        "Memory directory must remain intact"
    );

    Ok(())
}
