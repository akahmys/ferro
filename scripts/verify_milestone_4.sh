#!/bin/bash
set -e

echo "=== Milestone 4 Verification Started ==="

# 1. Setup testing directory
export FERRO_MEMORY_DIR="/tmp/ferro_memory"
export FERRO_WORKSPACE_ROOT="."
echo "Setting up memory directory at ${FERRO_MEMORY_DIR}"
rm -rf "${FERRO_MEMORY_DIR}"
mkdir -p "${FERRO_MEMORY_DIR}"

# 2. Build workspace binaries
echo "Building project workspace..."
cargo build --workspace

# 3. Start ferro-shell in background
echo "Launching ferro-shell (outer governance) in background..."
./target/debug/ferro-shell > /tmp/ferro_shell.log 2>&1 &
SHELL_PID=$!

# Ensure processes are cleaned up on script exit
trap "kill -9 $SHELL_PID 2>/dev/null || true; killall tutor.py 2>/dev/null || true; killall ferro-core 2>/dev/null || true; killall ferro-body 2>/dev/null || true" EXIT

# 4. Start tutor.py in background
echo "Launching simulation tutor..."
python3 scripts/tutor.py > /tmp/tutor.log 2>&1 &
TUTOR_PID=$!

# 5. Wait for 13 seconds (tutor drops panic_dump at 10s, shell reboots child processes)
echo "Waiting for simulated alignment failure, pruning, and self-healing reboot..."
sleep 13

# 6. Verify structural pruning outputs
echo "=== Verifying Governance Logs ==="
cat /tmp/ferro_shell.log

echo "=== Verifying pain_history.csv ==="
if [ -f "${FERRO_MEMORY_DIR}/pain_history.csv" ]; then
    cat "${FERRO_MEMORY_DIR}/pain_history.csv"
    echo "SUCCESS: pain_history.csv was successfully updated."
else
    echo "FAIL: pain_history.csv not found!"
    exit 1
fi

echo "=== Verifying breeding_signals.json ==="
if [ -f "${FERRO_MEMORY_DIR}/breeding_signals.json" ]; then
    cat "${FERRO_MEMORY_DIR}/breeding_signals.json"
    echo "SUCCESS: breeding_signals.json was successfully generated."
else
    echo "FAIL: breeding_signals.json not found!"
    exit 1
fi

# Ensure ferro-core and ferro-body were rebooted and are running
RUNNING=false
for i in {1..5}; do
    if pgrep -x "ferro-core" > /dev/null && pgrep -x "ferro-body" > /dev/null; then
        RUNNING=true
        break
    fi
    sleep 0.5
done

if [ "$RUNNING" = true ]; then
    echo "SUCCESS: Child processes are running after self-healing reboot."
else
    echo "FAIL: Child processes are not running after reboot!"
    exit 1
fi

echo "=== Milestone 4 Verification Completed Successfully ==="
exit 0
