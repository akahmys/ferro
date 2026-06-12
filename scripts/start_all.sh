#!/bin/bash
set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PID_FILE="${PROJECT_ROOT}/scratch/ferro_pids.txt"

# Ensure scratch directory exists
mkdir -p "${PROJECT_ROOT}/scratch"

# Clean up any leftover PID file
if [ -f "$PID_FILE" ]; then
    echo "Warning: Leftover PID file found. Cleaning up first..."
    "${PROJECT_ROOT}/scripts/stop_all.sh" || true
fi

echo "[FERRO] Starting all subsystems..."

# 1. Start Python Dashboard API Server
echo "[1/4] Starting Dashboard API Server on port 18080..."
python3 "${PROJECT_ROOT}/scripts/dashboard_api.py" > "${PROJECT_ROOT}/scratch/dashboard_api.log" 2>&1 &
API_PID=$!
sleep 1
if ! ps -p $API_PID > /dev/null; then
    echo "Error: Failed to start Dashboard API Server. Check scratch/dashboard_api.log"
    exit 1
fi
echo $API_PID > "$PID_FILE"

# 2. Start mock_env_sync.sh which launches ferro-env in the background
echo "[2/4] Starting Environmental Simulator Sync..."
# We run mock_env_sync.sh with -s (start env) and -d 30 (30s sleep duration)
"${PROJECT_ROOT}/scripts/mock_env_sync.sh" -s -d 30 > "${PROJECT_ROOT}/scratch/mock_env_sync.log" 2>&1 &
SYNC_PID=$!
sleep 1
if ! ps -p $SYNC_PID > /dev/null; then
    echo "Error: Failed to start Environmental Sync. Check scratch/mock_env_sync.log"
    # Clean up what we started
    kill -TERM $API_PID || true
    rm -f "$PID_FILE"
    exit 1
fi
echo $SYNC_PID >> "$PID_FILE"

# 3. Start ferro-shell (which compiles and controls the core docker container)
echo "[3/4] Starting ferro-shell controller daemon..."
cargo run --manifest-path="${PROJECT_ROOT}/ferro-shell/Cargo.toml" > "${PROJECT_ROOT}/scratch/ferro_shell.log" 2>&1 &
SHELL_PID=$!
sleep 1
if ! ps -p $SHELL_PID > /dev/null; then
    echo "Error: Failed to start ferro-shell. Check scratch/ferro_shell.log"
    # Clean up what we started
    kill -TERM $API_PID $SYNC_PID || true
    rm -f "$PID_FILE"
    exit 1
fi
echo $SHELL_PID >> "$PID_FILE"

# 4. Start Vite UI Frontend Server
echo "[4/4] Starting Vite UI Frontend Server on port 5173..."
if [ -d "${PROJECT_ROOT}/ferro-dashboard" ]; then
    cd "${PROJECT_ROOT}/ferro-dashboard"
    npm run dev > "${PROJECT_ROOT}/scratch/vite_dev.log" 2>&1 &
    VITE_PID=$!
    echo $VITE_PID >> "$PID_FILE"
    cd - > /dev/null
else
    echo "Warning: ferro-dashboard directory not initialized yet. Skipping UI startup."
fi

echo ""
echo "================================================================="
echo "  FERRO System successfully launched in the background!"
echo "  - API Server PID: $API_PID"
echo "  - Sync/Env PID: $SYNC_PID"
echo "  - Shell Controller PID: $SHELL_PID"
if [ -n "$VITE_PID" ]; then
echo "  - Vite UI PID: $VITE_PID"
fi
echo "================================================================="
echo "  Please open: http://localhost:5173"
echo "  To stop the entire system, execute: ./scripts/stop_all.sh"
echo "================================================================="
