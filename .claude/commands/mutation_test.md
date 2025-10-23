# Type Guide Comprehensive Validation Test

<ExecutionFlow/>

<TestConfiguration>
TYPES_PER_SUBAGENT = 1
MAX_SUBAGENTS = 4
BATCH_SIZE = ${TYPES_PER_SUBAGENT * MAX_SUBAGENTS}
BASE_PORT = 30001
MAX_PORT = ${BASE_PORT + MAX_SUBAGENTS - 1}
PORT_RANGE = ${BASE_PORT}-${MAX_PORT}

# Execution Mode
STOP_AFTER_EACH_BATCH = true   # Set to true for diagnostic mode, false for continuous execution
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

<GetBatchAssignments>
Execute script and capture JSON output:
```bash
python3 ./.claude/scripts/mutation_test/prepare.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
```

Parse JSON output (on stdout):
- `batch_number` - current batch number
- `total_types` - unique types being tested in this batch
- `max_subagents` - number of subagents being used
- `types_per_subagent` - configured types per subagent
- `progress_message` - pre-formatted progress message for display
- `assignments` - array of subagent assignments with fields:
  - `window_description` - window title
  - `task_description` - task description
  - `test_plan_file` - test plan path
  - `port` - port number

Store the complete JSON response for use in <ReportProgress/>, <PrepareApplications/>, and <LaunchMutationTestSubagents/>.
</GetBatchAssignments>

<ReportProgress>
Display the `progress_message` field from <GetBatchAssignments/> JSON output.

Example: "Processing batch 1 of 16 - Testing 8 types split across 10 subagents (152 remaining)"
</ReportProgress>

<PrepareApplications>
Task a general-purpose subagent to prepare applications using the assignments JSON:

```
description: "Prepare apps for batch N"
subagent_type: "general-purpose"
prompt: |
  Execute the workflow defined in @.claude/instructions/mutation_test_prep.md

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
python3 ./.claude/scripts/mutation_test/process_results.py --batch [BATCH_NUMBER] --max-subagents ${MAX_SUBAGENTS} --types-per-subagent ${TYPES_PER_SUBAGENT}
```

2. Parse JSON response (output is on stdout):
- `status` - "SUCCESS", "RETRY_ONLY", "FAILURES_DETECTED", or "ERROR"
- `batch` - {number, total_batches}
- `stats` - {types_tested, passed, failed, missing_components, remaining_types}
- `retry_failures` - array of retry failure summaries
- `review_failures` - array of review failure summaries
- `warnings` - array of warning messages
- `retry_log_file` - path to detailed retry failure log (null if none)
- `review_log_file` - path to detailed review failure log (null if none)

3. Execute <TestResultOutput/> to present results immediately

4. Store the complete JSON output for use in <CheckForFailures/> and <InteractiveFailureReview/>
</ProcessBatchResults>

<CheckForFailures>
Based on `status` field from ProcessBatchResults JSON:

**"SUCCESS"**:
- If STOP_AFTER_EACH_BATCH is true: Execute <FinalCleanup/> and STOP
- If STOP_AFTER_EACH_BATCH is false: Continue to next batch

**"RETRY_ONLY"**:
- Display retry notice showing `retry_failures` array
- If STOP_AFTER_EACH_BATCH is true: Execute <FinalCleanup/> and STOP
- If STOP_AFTER_EACH_BATCH is false: Continue to next batch (renumbering will retry these types)

**"FAILURES_DETECTED"**:
- Execute <FinalCleanup/> SILENTLY
- Execute <InteractiveFailureReview/> using the stored JSON output

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


## STEP 3: INTERACTIVE FAILURE REVIEW

<InteractiveFailureReview>
**Input**: Use `review_log_file` path from ProcessBatchResults JSON output

1. Read detailed failure data from review log file:
```bash
Read tool on {review_log_file}
```
This gives full failure details including operations_completed, failure_details, query_details.

2. Create todos using TodoWrite:
   - "Display failure summary and initialize review process"
   - "Review failure [X] of [TOTAL]" (one per review failure)

3. Display summary:
```
## MUTATION TEST EXECUTION COMPLETE

- **Status**: STOPPED DUE TO FAILURES
- **Progress**: Batch [N] of [TOTAL] processed
- **Results**: [PASS_COUNT] PASSED, [FAIL_COUNT] FAILED, [MISSING_COUNT] MISSING COMPONENTS
- **Review failures log**: [review_log_file path]
- **Retry failures log**: [retry_log_file path] (if any)
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

**Note**: Only review failures (real BRP errors) are reviewed. Retry failures (subagent crashes) are automatically retried in the next batch.
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
After receiving JSON output from process_results.py, present results immediately:

---

## Batch {batch.number} of {batch.total_batches} Results

**Status**: {status_icon} {status_text}

**Statistics**:
- Types Tested: {stats.types_tested}
- ✓ Passed: {stats.passed}
- ✗ Failed: {stats.failed}
- 🔄 Retry: {stats.retry}
- ⚠️ Missing Components: {stats.missing_components}
- Remaining: {stats.remaining_types} types

{IF retry_failures array is not empty:}
**Retry Failures** (will be retried automatically):
{FOR each failure in retry_failures array with index:}
{index+1}. `{failure.type}` - {failure.summary}
   Failed at: {failure.failed_at}
{END FOR}

**Retry Log**: {retry_log_file}
{END IF}

{IF review_failures array is not empty:}
**Review Failures** (need investigation):
{FOR each failure in review_failures array with index:}
{index+1}. `{failure.type}` - {failure.summary}
   Failed at: {failure.failed_at}
{END FOR}

**Review Log**: {review_log_file}
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
- "RETRY_ONLY" → ⚠️ SUBAGENT EXECUTION ISSUES (will retry)
- "FAILURES_DETECTED" → ❌ FAILURES DETECTED
- "ERROR" → 🔥 PROCESSING ERROR
</TestResultOutput>
