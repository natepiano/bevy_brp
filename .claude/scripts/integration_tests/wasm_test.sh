#!/usr/bin/env bash
# WASM BRP integration test script
# Usage: bash .claude/scripts/integration_tests/wasm_test.sh
#
# Validates that bevy_brp_extras compiles to WASM, runs in a browser via
# wasm-server-runner (with BRP relay), and responds to BRP requests.
#
# Exit code: 0 = all passed, 1 = any failures

set -euo pipefail

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BRP_PORT=20200
WEB_PORT=1334
WASM_DIR="test-wasm"
MAX_POLL_SECONDS=90
POLL_INTERVAL=2
FORKED_RUNNER_REPO="https://github.com/johanhelsing/wasm-server-runner"
FORKED_RUNNER_BRANCH="brp-relay"

# Chrome binary detection
CHROME_BIN=""
for candidate in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "google-chrome" \
    "chromium" \
    "chromium-browser"; do
    if command -v "${candidate}" &>/dev/null || [[ -x "${candidate}" ]]; then
        CHROME_BIN="${candidate}"
        break
    fi
done

# Counters
PASS_COUNT=0
FAIL_COUNT=0

# Background process PIDs (for cleanup)
RUNNER_PID=""
CHROME_PID=""

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

pass_() {
    local msg="$1"
    echo "  [PASS] ${msg}"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail_() {
    local msg="$1"
    echo "  [FAIL] ${msg}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

cleanup() {
    echo "  Cleaning up..."

    # Kill Chrome and ALL its child processes (GPU, renderer, utility, etc.)
    # First try by PID, then fallback to pattern match on user-data-dir
    if [[ -n "${CHROME_PID}" ]]; then
        echo "  Killing Chrome process tree (PID ${CHROME_PID})..."
        pkill -P "${CHROME_PID}" 2>/dev/null || true
        kill "${CHROME_PID}" 2>/dev/null || true
        wait "${CHROME_PID}" 2>/dev/null || true
        CHROME_PID=""
    fi
    # Fallback: kill any Chrome processes using our unique profile directory
    # This catches orphaned children even if the parent PID was lost
    pkill -f "bevy_brp_wasm_test_chrome" 2>/dev/null || true

    if [[ -n "${RUNNER_PID}" ]]; then
        echo "  Killing wasm-server-runner (PID ${RUNNER_PID})..."
        kill "${RUNNER_PID}" 2>/dev/null || true
        wait "${RUNNER_PID}" 2>/dev/null || true
        RUNNER_PID=""
    fi

    # Also kill any stray processes on the BRP and web ports
    lsof -ti :"${BRP_PORT}" 2>/dev/null | xargs kill 2>/dev/null || true
    lsof -ti :"${WEB_PORT}" 2>/dev/null | xargs kill 2>/dev/null || true

    # Clean up temporary Chrome profile
    if [[ -n "${CHROME_PROFILE:-}" && -d "${CHROME_PROFILE:-}" ]]; then
        rm -rf "${CHROME_PROFILE}" 2>/dev/null || true
    fi
}

trap cleanup EXIT INT TERM HUP

# JSON-RPC request ID counter
RPC_ID=1

# Make a BRP call and store the response in BRP_RESPONSE
brp_call() {
    local method="$1"
    local params="${2:-null}"

    local body
    if [[ "$params" == "null" ]]; then
        body="{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"id\":${RPC_ID}}"
    else
        body="{\"jsonrpc\":\"2.0\",\"method\":\"${method}\",\"params\":${params},\"id\":${RPC_ID}}"
    fi
    RPC_ID=$((RPC_ID + 1))

    BRP_RESPONSE=$(curl -sf "http://127.0.0.1:${BRP_PORT}" \
        -H "Content-Type: application/json" \
        -d "${body}" 2>/dev/null) || {
        echo "    [ERROR] curl failed for method=${method}"
        return 1
    }

    # Check for JSON-RPC error
    if echo "${BRP_RESPONSE}" | jq -e '.error' >/dev/null 2>&1; then
        local err_msg
        err_msg=$(echo "${BRP_RESPONSE}" | jq -r '.error.message // "unknown error"')
        echo "    [ERROR] BRP error for ${method}: ${err_msg}"
        return 1
    fi

    return 0
}

# ---------------------------------------------------------------------------
# Step 1: Verify Prerequisites
# ---------------------------------------------------------------------------

echo ""
echo "--- Step 1: Verify Prerequisites ---"

if rustup target list --installed 2>/dev/null | grep -q "wasm32-unknown-unknown"; then
    pass_ "wasm32-unknown-unknown target installed"
else
    fail_ "wasm32-unknown-unknown target not installed (run: rustup target add wasm32-unknown-unknown)"
    echo ""
    echo "RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
    echo "STATUS: FAILED"
    exit 1
fi

# Check for the forked wasm-server-runner (with BRP relay support)
if command -v wasm-server-runner &>/dev/null; then
    pass_ "wasm-server-runner installed"
else
    echo "  wasm-server-runner not found, installing from ${FORKED_RUNNER_REPO} (${FORKED_RUNNER_BRANCH})..."
    if cargo install wasm-server-runner --git "${FORKED_RUNNER_REPO}" --branch "${FORKED_RUNNER_BRANCH}" 2>&1; then
        pass_ "wasm-server-runner installed from fork"
    else
        fail_ "failed to install wasm-server-runner from fork"
        echo ""
        echo "RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
        echo "STATUS: FAILED"
        exit 1
    fi
fi

# Check for a headless browser (needed to load WASM and establish WebSocket relay)
if [[ -n "${CHROME_BIN}" ]]; then
    pass_ "headless browser found (${CHROME_BIN##*/})"
else
    fail_ "no Chrome/Chromium found (needed to load WASM app in headless mode)"
    echo ""
    echo "RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
    echo "STATUS: FAILED"
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 2: Build and Run WASM App
# ---------------------------------------------------------------------------

echo ""
echo "--- Step 2: Build and Run WASM App ---"

# Kill anything already on the BRP and web ports
lsof -ti :"${BRP_PORT}" 2>/dev/null | xargs kill 2>/dev/null || true
lsof -ti :"${WEB_PORT}" 2>/dev/null | xargs kill 2>/dev/null || true
sleep 0.5

# Run the WASM app in background (wasm-server-runner serves it and relays BRP)
(cd "${WASM_DIR}" && WASM_SERVER_RUNNER_BRP_PORT="${BRP_PORT}" cargo run --target wasm32-unknown-unknown 2>&1) &
RUNNER_PID=$!
echo "  Started wasm-server-runner (PID ${RUNNER_PID})"

# Wait for the web server to start serving at WEB_PORT
ELAPSED=0
WEB_READY=false
while [[ "${ELAPSED}" -lt "${MAX_POLL_SECONDS}" ]]; do
    if curl -sf "http://127.0.0.1:${WEB_PORT}" >/dev/null 2>&1; then
        WEB_READY=true
        break
    fi
    sleep "${POLL_INTERVAL}"
    ELAPSED=$((ELAPSED + POLL_INTERVAL))
    echo "  Waiting for web server... (${ELAPSED}s)"
done

if [[ "${WEB_READY}" == "true" ]]; then
    pass_ "wasm-server-runner web server ready (${ELAPSED}s)"
else
    fail_ "web server did not start within ${MAX_POLL_SECONDS}s"
    echo ""
    echo "RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
    echo "STATUS: FAILED"
    exit 1
fi

# Launch Chrome to load the WASM app (establishes WebSocket relay connection)
# Uses a temporary profile to avoid interfering with user's browser session
CHROME_PROFILE="${TMPDIR}/bevy_brp_wasm_test_chrome"
mkdir -p "${CHROME_PROFILE}"
echo "  Launching Chrome to load WASM app..."
"${CHROME_BIN}" \
    --headless=new \
    --enable-unsafe-webgpu \
    --enable-features=Vulkan \
    --user-data-dir="${CHROME_PROFILE}" \
    --no-first-run \
    --no-default-browser-check \
    "http://127.0.0.1:${WEB_PORT}" >/dev/null 2>&1 &
CHROME_PID=$!
echo "  Chrome headless started (PID ${CHROME_PID})"

# Poll BRP port until the WASM app connects via WebSocket and BRP becomes responsive
ELAPSED=0
READY=false
while [[ "${ELAPSED}" -lt "${MAX_POLL_SECONDS}" ]]; do
    # Try a simple BRP call — rpc.discover is always available
    if curl -sf "http://127.0.0.1:${BRP_PORT}" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"rpc.discover","id":0}' >/dev/null 2>&1; then
        READY=true
        break
    fi
    sleep "${POLL_INTERVAL}"
    ELAPSED=$((ELAPSED + POLL_INTERVAL))
    echo "  Waiting for BRP via WebSocket relay... (${ELAPSED}s)"
done

if [[ "${READY}" == "true" ]]; then
    pass_ "BRP responsive via WebSocket relay (${ELAPSED}s after Chrome launch)"
else
    fail_ "BRP did not respond within ${MAX_POLL_SECONDS}s after launching browser"
    echo ""
    echo "RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed"
    echo "STATUS: FAILED"
    exit 1
fi

# ---------------------------------------------------------------------------
# Step 3: BRP Round-Trip Verification
# ---------------------------------------------------------------------------

echo ""
echo "--- Step 3: BRP Round-Trip Verification ---"

# Test rpc.discover — should list available methods
if brp_call "rpc.discover"; then
    METHOD_COUNT=$(echo "${BRP_RESPONSE}" | jq '.result.methods | length' 2>/dev/null || echo "0")
    if [[ "${METHOD_COUNT}" -gt 0 ]]; then
        pass_ "rpc.discover returned ${METHOD_COUNT} methods"
    else
        fail_ "rpc.discover returned no methods"
    fi
else
    fail_ "rpc.discover call failed"
fi

# Test that brp_extras methods are registered
if brp_call "rpc.discover"; then
    EXTRAS_METHODS=$(echo "${BRP_RESPONSE}" | jq '[.result.methods[] | select(.name | startswith("brp_extras/"))] | length' 2>/dev/null || echo "0")
    if [[ "${EXTRAS_METHODS}" -gt 0 ]]; then
        pass_ "brp_extras methods registered (${EXTRAS_METHODS} methods)"
    else
        fail_ "no brp_extras methods found in rpc.discover"
    fi
else
    fail_ "rpc.discover call failed (extras check)"
fi

# Test world.query — query for our test entity by Name (proves ECS + BRP reflection work)
if brp_call "world.query" '{"data":{"components":["bevy_ecs::name::Name"]},"filter":{"with":["bevy_ecs::name::Name"]}}'; then
    ENTITY_NAME=$(echo "${BRP_RESPONSE}" | jq -r '.result[0].components."bevy_ecs::name::Name" // "none"' 2>/dev/null || echo "none")
    if [[ "${ENTITY_NAME}" == "wasm-brp-test" ]]; then
        pass_ "world.query found named entity (name=${ENTITY_NAME})"
    else
        fail_ "world.query did not find expected entity (got: ${ENTITY_NAME})"
    fi
else
    fail_ "world.query for Name failed"
fi

# Test world.query — query for Transform data (proves component data round-trip works)
if brp_call "world.query" '{"data":{"components":["bevy_transform::components::transform::Transform"]},"filter":{"with":["bevy_ecs::name::Name","bevy_transform::components::transform::Transform"]}}'; then
    TX=$(echo "${BRP_RESPONSE}" | jq '.result[0].components."bevy_transform::components::transform::Transform".translation[0]' 2>/dev/null || echo "null")
    TY=$(echo "${BRP_RESPONSE}" | jq '.result[0].components."bevy_transform::components::transform::Transform".translation[1]' 2>/dev/null || echo "null")
    TZ=$(echo "${BRP_RESPONSE}" | jq '.result[0].components."bevy_transform::components::transform::Transform".translation[2]' 2>/dev/null || echo "null")
    if [[ ("${TX}" == "1" || "${TX}" == "1.0") && ("${TY}" == "2" || "${TY}" == "2.0") && ("${TZ}" == "3" || "${TZ}" == "3.0") ]]; then
        pass_ "world.query returned correct Transform data (translation=[${TX},${TY},${TZ}])"
    else
        fail_ "world.query Transform data mismatch (expected [1,2,3], got [${TX},${TY},${TZ}])"
    fi
else
    fail_ "world.query for Transform failed"
fi

# ---------------------------------------------------------------------------
# Step 4: Cleanup (handled by trap)
# ---------------------------------------------------------------------------

echo ""
echo "--- Step 4: Cleanup ---"
echo "  Cleanup will run on exit (PID ${RUNNER_PID})"

# ---------------------------------------------------------------------------
# Results
# ---------------------------------------------------------------------------

echo ""
echo "================================================================"
TOTAL=$((PASS_COUNT + FAIL_COUNT))
echo "  RESULTS: ${PASS_COUNT} passed, ${FAIL_COUNT} failed (${TOTAL} total)"
if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    echo "  STATUS: FAILED"
    echo "================================================================"
    exit 1
else
    echo "  STATUS: PASSED"
    echo "================================================================"
    exit 0
fi
