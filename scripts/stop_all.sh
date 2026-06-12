#!/bin/bash

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PID_FILE="${PROJECT_ROOT}/scratch/ferro_pids.txt"

echo "[FERRO] Initiating clean shutdown sequence..."

# 1. Kill background processes saved in the PID file
if [ -f "$PID_FILE" ]; then
    echo "Stopping background processes from PID file..."
    while read -r pid; do
        if [ -n "$pid" ] && ps -p "$pid" > /dev/null 2>&1; then
            echo "Terminating PID: $pid..."
            kill -TERM "$pid" > /dev/null 2>&1
            sleep 0.5
            # Force kill if still running
            if ps -p "$pid" > /dev/null 2>&1; then
                echo "Force killing PID: $pid..."
                kill -9 "$pid" > /dev/null 2>&1
            fi
        fi
    done < "$PID_FILE"
    rm -f "$PID_FILE"
else
    echo "No PID file found. Searching for running processes manually..."
fi

# 2. Cleanup leftover processes manually by name pattern
echo "Cleaning up leftover processes..."
# Kill mock_env_sync.sh
pkill -f "mock_env_sync.sh" || true
# Kill dashboard_api.py
pkill -f "dashboard_api.py" || true
# Kill ferro-env
pkill -f "ferro-env" || true
# Kill ferro-shell
pkill -f "ferro-shell" || true

# 3. Stop and remove Docker container
CONTAINER_NAME="ferro-core-runtime"
if docker ps -a --format '{{.Names}}' | grep -Eq "^${CONTAINER_NAME}$"; then
    echo "Stopping Docker container: ${CONTAINER_NAME}..."
    docker stop "${CONTAINER_NAME}" > /dev/null 2>&1 || true
    echo "Removing Docker container: ${CONTAINER_NAME}..."
    docker rm "${CONTAINER_NAME}" > /dev/null 2>&1 || true
fi

echo "[FERRO] Shutdown sequence completed successfully!"
