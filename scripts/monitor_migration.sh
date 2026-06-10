#!/bin/bash

# ==============================================================================
# FERRO Storage & Phase Monitor Script
# ==============================================================================
# This script monitors migration events (JSON to redb) and cognitive phase
# transitions (Wake/Sleep) in the FERRO system.
# It watches storage.redb and surprise_history.csv in the memory directory.
# ==============================================================================

# Default configurations
DEFAULT_MEMORY_DIR="/memory"
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FALLBACK_MEMORY_DIR="$PROJECT_ROOT/memory"
LOG_FILE="$PROJECT_ROOT/scripts/monitor_migration.log"

# Target memory directory can be passed as the first argument
MEMORY_DIR="${1:-$DEFAULT_MEMORY_DIR}"

# Resolve fallback if default directory does not exist
if [ "$MEMORY_DIR" = "$DEFAULT_MEMORY_DIR" ] && [ ! -d "$DEFAULT_MEMORY_DIR" ]; then
    MEMORY_DIR="$FALLBACK_MEMORY_DIR"
fi

# Print message and write to log file
log_message() {
    local level="$1"
    local message="$2"
    local timestamp=$(date "+%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")"

log_message "INFO" "========================================="
log_message "INFO" "Initializing FERRO System Monitor..."
log_message "INFO" "Target Memory Dir : $MEMORY_DIR"
log_message "INFO" "Log File          : $LOG_FILE"
log_message "INFO" "========================================="

# Helper function to get file modification time (portable for macOS/Linux)
get_mtime() {
    local filepath="$1"
    if [ -f "$filepath" ]; then
        if stat -f "%m" "$filepath" >/dev/null 2>&1; then
            stat -f "%m" "$filepath" # macOS (BSD)
        else
            stat -c "%Y" "$filepath" # Linux (GNU)
        fi
    else
        echo 0
    fi
}

# Wait for memory directory to be created if not exists
if [ ! -d "$MEMORY_DIR" ]; then
    log_message "WARN" "Memory directory '$MEMORY_DIR' not found. Awaiting creation by ferro-core..."
    while [ ! -d "$MEMORY_DIR" ]; do
        sleep 2
    done
    log_message "INFO" "Memory directory '$MEMORY_DIR' detected."
fi

# File paths
REDB_FILE="$MEMORY_DIR/storage.redb"
JSON_DIR="$MEMORY_DIR/knowledge_graph"
CSV_FILE="$MEMORY_DIR/surprise_history.csv"

# Initial state variables
storage_exists=false
json_dir_exists=false
current_phase="Unknown"

last_redb_mtime=$(get_mtime "$REDB_FILE")
last_csv_mtime=$(get_mtime "$CSV_FILE")

# Initialize REDB storage state
if [ -f "$REDB_FILE" ]; then
    storage_exists=true
    log_message "INFO" "storage.redb is present. Migration has already completed."
else
    log_message "INFO" "storage.redb not found. System is in JSON storage mode."
fi

# Initialize JSON shards directory state
if [ -d "$JSON_DIR" ]; then
    json_dir_exists=true
    log_message "INFO" "knowledge_graph directory is present (awaiting migration threshold)."
fi

# Initialize phase state from surprise_history.csv
if [ -f "$CSV_FILE" ]; then
    last_line=$(tail -n 1 "$CSV_FILE" 2>/dev/null)
    if [ -n "$last_line" ] && [[ "$last_line" != "timestamp"* ]]; then
        initial_phase=$(echo "$last_line" | cut -d',' -f3 | tr -d '\r\n[:space:]')
        if [ -n "$initial_phase" ]; then
            current_phase="$initial_phase"
            log_message "INFO" "Current active phase detected: $current_phase"
        fi
    fi
else
    log_message "INFO" "surprise_history.csv not found. Awaiting first phase logging..."
fi

log_message "INFO" "Monitoring started successfully. Watching for transitions/migrations..."

# Main polling loop (1 second interval)
while true; do
    # 1. Monitor Migration: storage.redb Creation
    if [ "$storage_exists" = false ]; then
        if [ -f "$REDB_FILE" ]; then
            storage_exists=true
            last_redb_mtime=$(get_mtime "$REDB_FILE")
            log_message "MIGRATION" "Migration Event: storage.redb has been created! Database initialized."
        fi
    else
        # If storage already exists, monitor updates to redb
        current_redb_mtime=$(get_mtime "$REDB_FILE")
        if [ "$current_redb_mtime" -ne "$last_redb_mtime" ] && [ "$current_redb_mtime" -ne 0 ]; then
            last_redb_mtime="$current_redb_mtime"
            log_message "STORAGE" "Storage Update: storage.redb modified."
        fi
    fi

    # 2. Monitor Migration: Cleanup of knowledge_graph directory
    if [ "$json_dir_exists" = true ]; then
        if [ ! -d "$JSON_DIR" ]; then
            json_dir_exists=false
            log_message "MIGRATION" "Migration Event: knowledge_graph directory has been removed (JSON shards cleaned up)."
        fi
    else
        # If directory somehow reappears (e.g. system reset/re-run in JSON mode)
        if [ -d "$JSON_DIR" ]; then
            json_dir_exists=true
            log_message "INFO" "knowledge_graph directory has reappeared. Running in JSON storage mode."
        fi
    fi

    # 3. Monitor Phase Transitions: surprise_history.csv Update
    if [ -f "$CSV_FILE" ]; then
        current_csv_mtime=$(get_mtime "$CSV_FILE")
        
        # If CSV was modified or created
        if [ "$current_csv_mtime" -ne "$last_csv_mtime" ] && [ "$current_csv_mtime" -ne 0 ]; then
            last_csv_mtime="$current_csv_mtime"
            
            # Read last record
            last_line=$(tail -n 1 "$CSV_FILE" 2>/dev/null)
            if [ -n "$last_line" ] && [[ "$last_line" != "timestamp"* ]]; then
                new_phase=$(echo "$last_line" | cut -d',' -f3 | tr -d '\r\n[:space:]')
                energy=$(echo "$last_line" | cut -d',' -f2 | tr -d '\r\n[:space:]')
                ts=$(echo "$last_line" | cut -d',' -f1 | tr -d '\r\n[:space:]')
                
                if [ -n "$new_phase" ] && [ "$new_phase" != "$current_phase" ]; then
                    if [ "$current_phase" = "Unknown" ]; then
                        log_message "PHASE" "Phase resolved: $new_phase (timestamp: $ts, free_energy: $energy)"
                    else
                        log_message "PHASE" "Phase Transition: $current_phase -> $new_phase (timestamp: $ts, free_energy: $energy)"
                    fi
                    current_phase="$new_phase"
                fi
            fi
        fi
    fi

    sleep 1
done
