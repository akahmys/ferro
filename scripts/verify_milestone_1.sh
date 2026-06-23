#!/bin/bash
set -e

echo "=== Milestone 1 Verification Started ==="

# 1. Setup testing directory
export FERRO_MEMORY_DIR="/tmp/ferro_memory"
echo "Setting up memory directory at ${FERRO_MEMORY_DIR}"
rm -rf "${FERRO_MEMORY_DIR}"
mkdir -p "${FERRO_MEMORY_DIR}"

# 2. Build binaries
echo "Building project workspace..."
cargo build --workspace

# 3. Start ferro-body in background
echo "Launching ferro-body in background..."
./target/debug/ferro-body > /tmp/ferro_body.log 2>&1 &
BODY_PID=$!

# Ensure body PID is cleaned up on script exit
trap "kill -9 $BODY_PID 2>/dev/null || true" EXIT

# 4. Start ferro-core (runs synchronously, will self-terminate after 3s)
echo "Launching ferro-core..."
if ./target/debug/ferro-core; then
    echo "ferro-core exited successfully."
else
    echo "ferro-core exited with an error!"
    exit 1
fi

# 5. Wait for ferro-body to self-terminate (polling for 3 seconds max)
echo "Waiting for ferro-body to detect panic_dump and terminate..."
for i in {1..30}; do
    if ! kill -0 $BODY_PID 2>/dev/null; then
        echo "ferro-body terminated as expected."
        break
    fi
    sleep 0.1
done

# If body is still running, it's a failure
if kill -0 $BODY_PID 2>/dev/null; then
    echo "FAIL: ferro-body did not terminate after core panic!"
    exit 1
fi

# 6. Verify outputs and panic_dump.json
echo "=== Verifying Output Logs ==="
cat /tmp/ferro_body.log

echo "=== Verifying panic_dump.json ==="
if [ -f "${FERRO_MEMORY_DIR}/panic_dump.json" ]; then
    cat "${FERRO_MEMORY_DIR}/panic_dump.json"
    echo "SUCCESS: panic_dump.json was successfully generated."
else
    echo "FAIL: panic_dump.json not found!"
    exit 1
fi

echo "=== Milestone 1 Verification Completed Successfully ==="
exit 0
