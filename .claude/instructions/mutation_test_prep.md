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

**STEP 0:** Execute <BackupDebugLog/>
**STEP 1:** Execute <ShutdownExistingApps/>
**STEP 2:** Execute <LaunchFreshApps/>
**STEP 3:** Execute <VerifyAppsRunning/>
**STEP 4:** Execute <SetWindowTitles/>
</ExecutionSteps>

## STEP 0: BACKUP DEBUG LOG

<BackupDebugLog>
Backup the existing debug log to a timestamped file for later analysis:

```bash
if [ -f /tmp/mutation_hook_debug.log ]; then
  # Extract batch number and timestamp from existing log metadata
  BATCH_NUM=$(grep "^# Batch Number:" /tmp/mutation_hook_debug.log | head -1 | awk '{print $4}')
  LOG_TIMESTAMP=$(grep "^# Started:" /tmp/mutation_hook_debug.log | head -1 | awk '{print $3, $4}' | tr -d ':' | tr ' ' '_' | tr -d '-')

  # Use extracted metadata in backup filename (or defaults if parsing fails)
  BATCH_NUM=${BATCH_NUM:-unknown}
  LOG_TIMESTAMP=${LOG_TIMESTAMP:-$(date '+%Y%m%d_%H%M%S')}

  BACKUP_FILE="/tmp/mutation_hook_debug_batch${BATCH_NUM}_${LOG_TIMESTAMP}.log"

  # Move existing log to backup (preserves all content including metadata)
  mv /tmp/mutation_hook_debug.log "$BACKUP_FILE"
fi

# Create new debug log with metadata for current batch
TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
BATCH_NUM=$(echo "$assignments" | jq -r '.batch_number // "unknown"')

cat > /tmp/mutation_hook_debug.log << EOF
# Mutation Test Debug Log
# Batch Number: ${BATCH_NUM}
# Started: $(date '+%Y-%m-%d %H:%M:%S')
# Ports: $(echo "$assignments" | jq -r '.assignments | map(.port) | join(", ")')
# Types: $(echo "$assignments" | jq -r '.assignments | map(.window_description) | join(", ")')
# ----------------------------------------

EOF
```

This creates a fresh debug log with metadata for the current batch while preserving previous batch logs for analysis.
</BackupDebugLog>

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
