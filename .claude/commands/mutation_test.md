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
**STEP 1:** Execute <BatchProcessingLoop/>
**STEP 2:** Execute <FinalCleanup/> (SILENTLY if failures detected)
**STEP 3:** Execute <InteractiveFailureReview/> (ONLY if failures detected)
</ExecutionFlow>

## STEP 1: BATCH PROCESSING LOOP

<BatchProcessingLoop>
For each batch N (starting from 1):
1. Execute <ReportProgress/>
2. Execute <GetBatchAssignments/>
3. Execute <PrepareApplications/>
4. Execute <LaunchMutationTestSubagents/>
5. Execute <ProcessBatchResults/>
6. Execute <CheckForFailures/>

Continue until all batches processed or failures occur.
</BatchProcessingLoop>

<ReportProgress>
Display: "Processing batch N of [TOTAL_BATCHES] - Testing [TYPES_IN_BATCH] types ([REMAINING_TYPES] remaining)"
</ReportProgress>

<GetBatchAssignments>
Execute script and capture JSON output:
```bash
python3 ./.claude/scripts/mutation_test_prepare.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
```

Parse JSON output (on stdout):
- `assignments` - array of subagent assignments with fields:
  - `window_description` - window title
  - `task_description` - task description
  - `test_plan_file` - test plan path
  - `port` - port number

Store the complete JSON response for use in <PrepareApplications/> and <LaunchMutationTestSubagents/>.
</GetBatchAssignments>

<PrepareApplications>
Task a general-purpose subagent to prepare applications using the assignments JSON:

```
description: "Prepare apps for batch N"
subagent_type: "general-purpose"
prompt: |
  Execute the workflow defined in @.claude/instructions/mutation-test-prep.md

  You are preparing application instances for mutation test batch N.

  Use the following assignments JSON to set window titles in STEP 4:

  ```json
  [PASTE COMPLETE JSON FROM GetBatchAssignments]
  ```

  Follow all steps in the instruction file. Report any errors immediately.
```

Wait for subagent to complete before proceeding to <LaunchMutationTestSubagents/>.
</PrepareApplications>

<LaunchMutationTestSubagents>
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

**"NULL_STATUS_ONLY"**:
- Display <NullStatusRetryNotice/> using the stored `failures` array
- Continue to next batch (renumbering will retry these types)

**"FAILURES_DETECTED"**:
- Execute <FinalCleanup/> SILENTLY
- Execute <InteractiveFailureReview/> using the stored `failures` array

**"ERROR"**:
- Execute <FinalCleanup/> SILENTLY
- Stop with error message
</CheckForFailures>

## STEP 2: FINAL CLEANUP

<FinalCleanup>
Execute <ParallelPortOperation/> with:
- Operation: mcp__brp__brp_shutdown
- Parameters: app_name="extras_plugin"
- Mode: SILENT (no output)
</FinalCleanup>

## NULL STATUS RETRY NOTICE

<NullStatusRetryNotice>
Display the following notice when status is "NULL_STATUS_ONLY":

---

## Batch {batch.number} - Subagent Execution Failures (Retrying)

**Status**: ⚠️ SUBAGENT EXECUTION ISSUES

The following types had subagent execution failures (null status fields). These are NOT BRP validation errors.
These types will be automatically retried in the next batch.

**Affected Types** ({count}):
{FOR each failure in failures array:}
- `{failure.type}` - {failure.summary}
{END FOR}

**Details**: {log_file}

**Next Action**: Continuing to next batch. The renumbering process will pick these up for retry.

---
</NullStatusRetryNotice>

## STEP 3: INTERACTIVE FAILURE REVIEW

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
- "NULL_STATUS_ONLY" → ⚠️ SUBAGENT EXECUTION ISSUES (will retry)
- "FAILURES_DETECTED" → ❌ FAILURES DETECTED
- "ERROR" → 🔥 PROCESSING ERROR
</TestResultOutput>
