# Mutation Test Application Preparation

## Purpose

Prepare Bevy application instances for mutation testing by shutting down existing instances, launching fresh instances, verifying they're running with BRP enabled, and setting window titles based on batch assignments.

## Configuration

**Receive from parent agent:**
- `assignments` - JSON array of subagent assignments with fields:
  - `port` - port number for this instance
  - `window_description` - window title to set
- `max_subagents` - number of subagents to prepare (equals length of assignments array)

**Derived values from assignments JSON:**
- `INSTANCE_COUNT` = length of assignments array
- `BASE_PORT` = lowest port number in assignments
- `MAX_PORT` = highest port number in assignments
- `PORTS` = list of all port numbers from assignments

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <ShutdownExistingApps/>
**STEP 2:** Execute <LaunchFreshApps/>
**STEP 3:** Execute <VerifyAppsRunning/>
**STEP 4:** Execute <SetWindowTitles/>
</ExecutionSteps>

## STEP 1: SHUTDOWN EXISTING APPS

<ShutdownExistingApps>
For each port in the assignments JSON, execute in parallel:

```python
mcp__brp__brp_shutdown(
    app_name="extras_plugin",
    port=assignment.port
)
```

Execute ALL shutdown operations in parallel using the port values from the assignments array.
</ShutdownExistingApps>

## STEP 2: LAUNCH FRESH APPS

<LaunchFreshApps>
Launch application instances for all assigned ports:

1. Extract values from assignments JSON:
   - `instance_count` = length of assignments array
   - `base_port` = lowest port in assignments array

2. Launch instances:
```python
mcp__brp__brp_launch_bevy_example(
    target_name="extras_plugin",
    port=base_port,
    instance_count=instance_count
)
```

This will launch exactly the number of instances needed for this batch.
</LaunchFreshApps>

## STEP 3: VERIFY APPS RUNNING

<VerifyAppsRunning>
For each port in the assignments JSON, execute in parallel:

```python
mcp__brp__brp_status(
    app_name="extras_plugin",
    port=assignment.port
)
```

Execute ALL status checks in parallel using the port values from the assignments array.

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
