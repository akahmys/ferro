#!/bin/bash

# ==============================================================================
# FERRO Environmental Simulator Mock Script: mock_env_sync.sh
# Purpose: Detect sleep state transition of the core and suspend data supply
#          (from ferro-env to memory/stimulus/*.json) for 15 minutes.
# ==============================================================================

# Default configurations
DURATION=900 # 15 minutes in seconds
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CSV_PATH="${PROJECT_ROOT}/ferro-core/memory/surprise_history.csv"
ENV_PID=""
START_ENV=false
IS_SUSPENDED=false
CLEANED=false

# Help instructions
show_help() {
  echo "Usage: $0 [options]"
  echo "Options:"
  echo "  -d, --duration SEC   Suspension duration in seconds (default: 900)"
  echo "  -c, --csv PATH       Path to surprise_history.csv (default: ferro-core/memory/surprise_history.csv)"
  echo "  -s, --start-env      Automatically start ferro-env in the background"
  echo "  -p, --pid PID        PID of already running ferro-env"
  echo "  -h, --help           Show this help message"
  echo ""
  echo "Mock signal support:"
  echo "  Send SIGUSR1 to this script (kill -USR1 \$\$) to trigger simulated sleep state immediately."
}

# Parse options
while [[ $# -gt 0 ]]; do
  case "$1" in
    -d|--duration)
      DURATION="$2"
      shift 2
      ;;
    -c|--csv)
      CSV_PATH="$2"
      shift 2
      ;;
    -s|--start-env)
      START_ENV=true
      shift
      ;;
    -p|--pid)
      ENV_PID="$2"
      shift 2
      ;;
    -h|--help)
      show_help
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      show_help
      exit 1
      ;;
  esac
done

# Cleanup function
cleanup() {
  if [ "$CLEANED" = true ]; then
    return
  fi
  CLEANED=true
  echo ""
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cleaning up resources..."
  if [ -n "$ENV_PID" ]; then
    if ps -p "$ENV_PID" > /dev/null 2>&1; then
      if [ "$IS_SUSPENDED" = true ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Resuming ferro-env (PID: $ENV_PID) before exiting..."
        kill -CONT "$ENV_PID" > /dev/null 2>&1
      fi
      if [ "$START_ENV" = true ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Terminating started background ferro-env (PID: $ENV_PID)..."
        kill -TERM "$ENV_PID" > /dev/null 2>&1
      fi
    fi
  fi
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Cleanup finished."
  exit 0
}

# Set traps
trap cleanup EXIT
trap 'exit 0' SIGINT SIGTERM

# Suspend data supply
trigger_sleep_suspension() {
  if [ "$IS_SUSPENDED" = true ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Data supply is already suspended. Ignoring duplicate trigger."
    return
  fi
  
  IS_SUSPENDED=true
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] >>> Sleep state detected! <<<"
  
  if [ -n "$ENV_PID" ] && ps -p "$ENV_PID" > /dev/null 2>&1; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Sending SIGSTOP to ferro-env (PID: $ENV_PID)..."
    kill -STOP "$ENV_PID"
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Data supply suspended. Waiting for $DURATION seconds (monitoring for early wakeup)..."
    
    start_time=$(date +%s)
    end_time=$((start_time + DURATION))
    while [ $(date +%s) -lt $end_time ]; do
      if [ -f "$CSV_PATH" ]; then
        last_line=$(tail -n 1 "$CSV_PATH")
        current_phase=$(echo "$last_line" | awk -F',' '{print $3}' | tr -d '\r\n[:space:]')
        if [ "$current_phase" = "Wake" ]; then
          echo "[$(date '+%Y-%m-%d %H:%M:%S')] Core woke up early. Resuming environmental data supply."
          break
        fi
      fi
      sleep 1
    done
    
    if ps -p "$ENV_PID" > /dev/null 2>&1; then
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] Sending SIGCONT to ferro-env (PID: $ENV_PID)..."
      kill -CONT "$ENV_PID"
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] Data supply resumed successfully."
    else
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] Warning: ferro-env (PID: $ENV_PID) was terminated during suspension."
    fi
  else
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Warning: ferro-env (PID: $ENV_PID) is not running. Skipping SIGSTOP/SIGCONT."
    # Simulate suspension wait anyway in case of dummy test
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Waiting for $DURATION seconds in mock state (monitoring for early wakeup)..."
    start_time=$(date +%s)
    end_time=$((start_time + DURATION))
    while [ $(date +%s) -lt $end_time ]; do
      if [ -f "$CSV_PATH" ]; then
        last_line=$(tail -n 1 "$CSV_PATH")
        current_phase=$(echo "$last_line" | awk -F',' '{print $3}' | tr -d '\r\n[:space:]')
        if [ "$current_phase" = "Wake" ]; then
          echo "[$(date '+%Y-%m-%d %H:%M:%S')] Core woke up early. Breaking mock suspension."
          break
        fi
      fi
      sleep 1
    done
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Mock suspension completed."
  fi
  
  IS_SUSPENDED=false
}

# Setup simulated sleep signal trap
trap 'trigger_sleep_suspension' SIGUSR1

# Start or auto-detect ferro-env
if [ "$START_ENV" = true ]; then
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] Launching ferro-env in the background..."
  export FERRO_MEMORY_PATH=$(dirname "$(dirname "$CSV_PATH")")
  cd "${PROJECT_ROOT}/ferro-env" || exit 1
  cargo run --quiet &
  ENV_PID=$!
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] ferro-env started in background. PID: $ENV_PID, Memory Path: $FERRO_MEMORY_PATH"
  cd - > /dev/null || exit 1
else
  if [ -z "$ENV_PID" ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Searching for running ferro-env process..."
    # Attempt to find compiled binary execution or cargo runs
    ENV_PID=$(pgrep -f "target/debug/ferro-env" | head -n 1)
    if [ -z "$ENV_PID" ]; then
      ENV_PID=$(pgrep -f "ferro-env" | grep -v "$$" | grep -v "mock_env_sync.sh" | head -n 1)
    fi
    
    if [ -z "$ENV_PID" ]; then
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] No active ferro-env process found yet. Will auto-detect dynamically during run."
    else
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] Found ferro-env process PID: $ENV_PID"
    fi
  fi
fi

echo "[$(date '+%Y-%m-%d %H:%M:%S')] Monitoring started."
echo "  - PID: ${ENV_PID:-Auto-detecting...}"
echo "  - CSV Path: $CSV_PATH"
echo "  - Suspension Duration: $DURATION sec"
echo "  - Script PID: $$ (Send 'kill -USR1 $$' to trigger mock sleep state)"

# Monitoring loop
prev_phase=""
while true; do
  # Dynamic PID discovery if not yet bound or process died and restarted
  if [ -z "$ENV_PID" ] || ! ps -p "$ENV_PID" > /dev/null 2>&1; then
    DETECTED_PID=$(pgrep -f "target/debug/ferro-env" | head -n 1)
    if [ -z "$DETECTED_PID" ]; then
      DETECTED_PID=$(pgrep -f "ferro-env" | grep -v "$$" | grep -v "mock_env_sync.sh" | head -n 1)
    fi
    if [ -n "$DETECTED_PID" ] && [ "$DETECTED_PID" != "$ENV_PID" ]; then
      ENV_PID="$DETECTED_PID"
      echo "[$(date '+%Y-%m-%d %H:%M:%S')] Dynamic auto-detect: ferro-env PID bound to $ENV_PID"
    fi
  fi

  if [ -f "$CSV_PATH" ]; then
    # Read the final line of surprise_history.csv
    last_line=$(tail -n 1 "$CSV_PATH")
    # CSV columns: timestamp,global_free_energy,phase
    current_phase=$(echo "$last_line" | awk -F',' '{print $3}' | tr -d '\r\n[:space:]')
    
    if [ "$current_phase" = "Sleep" ] && [ "$prev_phase" != "Sleep" ]; then
      trigger_sleep_suspension
    fi
    prev_phase="$current_phase"
  fi
  sleep 1
done
