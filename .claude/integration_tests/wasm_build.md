# WASM BRP Integration Test

## Objective
Validate that `bevy_brp_extras` compiles to WASM, runs in a browser via `wasm-server-runner` (with BRP relay), and responds to BRP requests end-to-end.

## Self-Managed Test
This test does NOT use MCP tools to launch/query a Bevy app. It runs a single shell script that handles all steps.

## Execution

Run the test script from the project root:
```bash
bash .claude/scripts/integration_tests/wasm_build_test.sh
```

The script handles all steps internally and exits 0 on success, 1 on failure.

## Test Steps (executed by the script)

### 1. Verify Prerequisites
- `wasm32-unknown-unknown` target is installed
- `wasm-server-runner` (johanhelsing's fork with BRP relay) is installed; auto-installs from git if missing

### 2. Build and Run WASM App
- `cargo run --target wasm32-unknown-unknown` from `test-wasm/` in background
- `wasm-server-runner` (johanhelsing's fork) serves the WASM app and relays BRP on port 20200
- Waits for web server at port 1334, then launches headless Chrome with WebGPU to load the app
- Chrome loads the WASM app, which connects via WebSocket back to the relay
- Polls BRP port until `rpc.discover` responds (up to 90 seconds)

### 3. BRP Round-Trip Verification
- `rpc.discover` returns methods (proves BRP is working through the WebSocket relay)
- `brp_extras/` methods are registered (proves `BrpExtrasPlugin` is active on WASM)
- `world.query` for `Name` component finds `wasm-brp-test` entity (proves ECS + BRP reflection work)
- `world.query` for `Transform` returns correct `translation=[1,2,3]` (proves component data round-trip)

### 4. Cleanup
- Kills headless Chrome and `wasm-server-runner` processes
- Removes temporary Chrome profile

## Expected Results
- WASM app builds and runs with WebGPU in headless Chrome
- BRP round-trips work through the WebSocket relay
- `BrpExtrasPlugin` methods are discoverable
- Spawned entities are queryable with correct component data

## Failure Criteria
STOP if: Prerequisites missing (including Chrome), WASM build fails, BRP does not respond within 90 seconds, or any BRP call fails.
