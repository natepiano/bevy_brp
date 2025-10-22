# Mutation Test Application Preparation

## Purpose

Prepare Bevy application instances for mutation testing by shutting down existing instances, launching fresh instances, verifying they're running with BRP enabled, and setting window titles based on batch assignments.

## Configuration

**Receive from parent agent:**
- `assignments` - JSON array of subagent assignments with fields:
  - `port` - port number for this instance
  - `window_description` - window title to set

**Constants:**
```
MAX_SUBAGENTS = 10
BASE_PORT = 30001
MAX_PORT = ${BASE_PORT + MAX_SUBAGENTS - 1}
PORT_RANGE = ${BASE_PORT}-${MAX_PORT}
```

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <ShutdownExistingApps/>
**STEP 2:** Execute <LaunchFreshApps/>
**STEP 3:** Execute <VerifyAppsRunning/>
**STEP 4:** Execute <SetWindowTitles/>
</ExecutionSteps>

## STEP 1: SHUTDOWN EXISTING APPS

<ShutdownExistingApps>
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_shutdown
- Parameters: app_name="extras_plugin"
</ShutdownExistingApps>

## STEP 2: LAUNCH FRESH APPS

<LaunchFreshApps>
Launch ${MAX_SUBAGENTS} application instances:

```python
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=${BASE_PORT},
    instance_count=${MAX_SUBAGENTS}
)
```
</LaunchFreshApps>

## STEP 3: VERIFY APPS RUNNING

<VerifyAppsRunning>
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_status
- Parameters: app_name="extras_plugin"

**CRITICAL**: If any app fails to respond with `running_with_brp` status, report the error and STOP.
Do not proceed to window title setting if verification fails.
</VerifyAppsRunning>

## STEP 4: SET WINDOW TITLES

<SetWindowTitles>
For each assignment in the `assignments` array provided by parent:

```python
mcp__brp__brp_extras_set_window_title(
    port=assignment.port,
    title=assignment.window_description
)
```

Execute ALL window title operations in parallel.
</SetWindowTitles>

## REUSABLE PATTERNS

<ParallelPortOperation>
Execute in parallel for ports ${BASE_PORT} through ${MAX_PORT}:
```python
[Operation](app_name=[Parameters.app_name], port=PORT)
```

Where:
- `[Operation]` is the MCP tool to execute (e.g., `mcp__brp__brp_shutdown`, `mcp__brp__brp_status`)
- `[Parameters.app_name]` is the app_name parameter value (e.g., "extras_plugin")
- `PORT` ranges from ${BASE_PORT} to ${MAX_PORT}
</ParallelPortOperation>
