# Type Guide Comprehensive Validation Test

<ExecutionFlow/>

<TestConfiguration>
TYPES_PER_SUBAGENT = 1
MAX_SUBAGENTS = 10
BATCH_SIZE = ${TYPES_PER_SUBAGENT * MAX_SUBAGENTS}
BASE_PORT = 30001
MAX_PORT = ${BASE_PORT + MAX_SUBAGENTS - 1}
PORT_RANGE = ${BASE_PORT}-${MAX_PORT}
</TestConfiguration>

<ExecutionFlow>
**STEP 1:** Execute <ApplicationLaunch/>
**STEP 2:** Execute <ApplicationVerification/>
**STEP 3:** Execute <BatchProcessingLoop/>
**STEP 4:** Execute <FinalCleanup/> (SILENTLY if failures detected)
**STEP 5:** Execute <InteractiveFailureReview/> (ONLY if failures detected)
</ExecutionFlow>

## STEP 1: APPLICATION LAUNCH

<ApplicationLaunch>
1. Shutdown existing apps:
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_shutdown
- Parameters: app_name="extras_plugin"

2. Launch ${MAX_SUBAGENTS} apps:
```python
mcp__brp__brp_launch_bevy_example(
    example_name="extras_plugin",
    port=${BASE_PORT},
    instance_count=${MAX_SUBAGENTS}
)
```
</ApplicationLaunch>

## STEP 2: APPLICATION VERIFICATION

<ApplicationVerification>
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_status
- Parameters: app_name="extras_plugin"

**STOP IF** any app fails to respond.
</ApplicationVerification>

## STEP 3: BATCH PROCESSING LOOP

<BatchProcessingLoop>
For each batch N (starting from 1):
1. **REPORT PROGRESS**: "Processing batch N of [TOTAL_BATCHES] - Testing [TYPES_IN_BATCH] types ([REMAINING_TYPES] remaining)"
2. Execute <GetBatchAssignments/>
3. Execute <SetWindowTitles/>
4. Execute <LaunchSubagents/>
5. Execute <ProcessBatchResults/>
6. Execute <CheckForFailures/>

Continue until all batches processed or failures occur.
</BatchProcessingLoop>

<GetBatchAssignments>
```bash
python3 ./.claude/scripts/mutation_test_prepare.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
```

Extract from JSON output:
- `assignments` - array of subagent assignments
- `assignments[i].window_description` - window title
- `assignments[i].task_description` - task description
- `assignments[i].test_plan_file` - test plan path
- `assignments[i].port` - port number

Store assignments array for SetWindowTitles and LaunchSubagents.
</GetBatchAssignments>

<SetWindowTitles>
For each assignment in assignments array:

```python
mcp__brp__brp_extras_set_window_title(
    port=assignment.port,
    title=assignment.window_description
)
```

Execute ALL in parallel.
</SetWindowTitles>

<LaunchSubagents>
For each assignment in assignments array, create Task:

```
description: assignment.task_description
subagent_type: "mutation-test-executor"
prompt: |
  EXECUTE the mutation test workflow defined in @.claude/instructions/mutation_test_subagent.md

  Your configuration:
  - TEST_PLAN_FILE = [assignment.test_plan_file]
  - PORT = [assignment.port]

  The test plan file contains ALL the operations you need to execute. Read it and execute each operation in sequence.

  CRITICAL: After completing all operations, just finish. The test plan file will contain all results. Do not return any JSON output.
```

Send ALL Tasks in ONE message for parallel execution.
</LaunchSubagents>

<ProcessBatchResults>
1. Execute script and capture JSON output:
```bash
python3 ./.claude/scripts/mutation_test_process_results.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
```

2. Parse JSON response (output is on stdout):
- `status` - "SUCCESS", "FAILURES_DETECTED", or "ERROR"
- `batch` - {number, total_batches}
- `stats` - {types_tested, passed, failed, missing_components, remaining_types}
- `failures` - array of failure summaries
- `warnings` - array of warning messages
- `log_file` - path to detailed failure log (null if no failures)

3. Execute <TestResultOutput/> to present results immediately

4. Store the parsed `failures` array for use in <InteractiveFailureReview/> if needed
</ProcessBatchResults>

<CheckForFailures>
Based on `status` field from ProcessBatchResults JSON:

**"SUCCESS"**: Continue to next batch

**"FAILURES_DETECTED"**:
- Execute <FinalCleanup/> SILENTLY
- Execute <InteractiveFailureReview/> using the stored `failures` array

**"ERROR"**:
- Execute <FinalCleanup/> SILENTLY
- Stop with error message
</CheckForFailures>

## STEP 4: FINAL CLEANUP

<FinalCleanup>
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_shutdown
- Parameters: app_name="extras_plugin"
- Mode: SILENT (no output)
</FinalCleanup>

## STEP 5: INTERACTIVE FAILURE REVIEW

<InteractiveFailureReview>
**Input**: Use `log_file` path from ProcessBatchResults JSON output

1. Read detailed failure data from log file:
```bash
Read tool on {log_file}
```
This gives full failure details including operations_completed, failure_details, query_details.

2. Create todos using TodoWrite:
   - "Display failure summary and initialize review process"
   - "Review failure [X] of [TOTAL]" (one per failure)

3. Display summary:
```
## MUTATION TEST EXECUTION COMPLETE

- **Status**: STOPPED DUE TO FAILURES
- **Progress**: Batch [N] of [TOTAL] processed
- **Results**: [PASS_COUNT] PASSED, [FAIL_COUNT] FAILED, [MISSING_COUNT] MISSING COMPONENTS
- **Detailed failure log**: [log_file path]
```

4. For each failure, present:
```
## FAILURE [X] of [TOTAL]: `[type_name]`

### Overview
- **Entity ID**: [entity_id]
- **Total Mutations**: [total] attempted
- **Mutations Passed**: [count] succeeded
- **Failed At**: [operation type or mutation path]

### What Succeeded Before Failure
[List successful operations]

### The Failure

**Failed [Operation/Path]**: [specific failure point]

**What We Sent**:
```json
[request]
```

**Error Response**:
```json
[response]
```

### Analysis
[Error analysis]

---

## Available Actions
- **investigate** - Investigate this failure (DEFAULT - always investigate first)
- **skip** - Skip to next failure
- **stop** - Stop review

Please select one of the keywords above.
```

5. Execute <InvestigateFailure/> immediately after presenting each failure
6. Wait for user response
7. Handle keyword: investigate (already done), skip (next failure), stop (exit)

**Note**: Detailed failure data is read ONCE from the log file at the start of this phase, eliminating the need to hunt for files during review.
</InteractiveFailureReview>

<InvestigateFailure>
1. Run: `.claude/scripts/get_type_guide.sh <failed_type_name> --file .claude/transient/all_types.json`
2. Examine type guide for failed mutation path
3. Check `path_info` for `applicable_variants`, `root_example`, `mutability`
4. Present findings and recommendations
5. Do NOT launch Task agents
</InvestigateFailure>

## REUSABLE PATTERNS

<ParallelPortOperation>
Execute in parallel for ports ${BASE_PORT}-${MAX_PORT}:
```python
[Operation](app_name=[Parameters.app_name], port=PORT)
```
</ParallelPortOperation>

<TestResultOutput>
After receiving JSON output from mutation_test_process_results.py, present results immediately:

---

## Batch {batch.number} of {batch.total_batches} Results

**Status**: {status_icon} {status_text}

**Statistics**:
- Types Tested: {stats.types_tested}
- ✓ Passed: {stats.passed}
- ✗ Failed: {stats.failed}
- ⚠️ Missing Components: {stats.missing_components}
- Remaining: {stats.remaining_types} types

{IF failures array is not empty:}
**Failures Summary**:
{FOR each failure in failures array with index:}
{index+1}. `{failure.type}` - {failure.summary}
   Failed at: {failure.failed_at} operation
{END FOR}

**Detailed Log**: {log_file}
{END IF}

{IF warnings array is not empty:}
**Warnings**:
{FOR each warning in warnings array:}
- {warning}
{END FOR}
{END IF}

---

**Status Icons**:
- "SUCCESS" → ✅ ALL TESTS PASSED
- "FAILURES_DETECTED" → ❌ FAILURES DETECTED
- "ERROR" → 🔥 PROCESSING ERROR
</TestResultOutput>
