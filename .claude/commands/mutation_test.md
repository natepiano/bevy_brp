# Type Guide Comprehensive Validation Test

Configuration is loaded from `.claude/config/mutation_test_config.json`.

<ExecutionFlow>
**STEP 1:** Execute <BatchProcessingLoop/> (DO NOT output results yet)
**STEP 2:** Execute <FinalCleanup/> (ALWAYS - shutdown apps)
**STEP 3:** Execute <TestResultOutput/> followed by <FinalDiagnosticOutput/> (ALWAYS - show summary and diagnostic table)
**STEP 4:** Execute <AskUserToReviewFailures/> (ONLY if review failures detected - ask user before proceeding)
**STEP 5:** Execute <InteractiveFailureReview/> (ONLY if user confirms they want to review)
</ExecutionFlow>

## STEP 1: BATCH PROCESSING LOOP

<BatchProcessingLoop>
For each batch (auto-discovered from all_types.json):
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
python3 ./.claude/scripts/mutation_test/prepare.py
```

The script auto-discovers the current batch number and loads configuration from `.claude/config/mutation_test_config.json`.

Parse JSON output (on stdout):
- `batch_number` - current batch number (auto-discovered)
- `total_types` - unique types being tested in this batch
- `max_subagents` - number of subagents being used
- `types_per_subagent` - configured types per subagent
- `progress_message` - pre-formatted progress message for display
- `assignments` - array of subagent assignments with fields:
  - `window_description` - window title
  - `task_description` - task description
  - `test_plan_file` - test plan path
  - `port` - port number

Store the complete JSON response for use in <ReportProgress/>, <PrepareApplications/>, <LaunchMutationTestSubagents/>, and <FinalCleanup/>.
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
python3 ./.claude/scripts/mutation_test/process_results.py
```

The script auto-discovers the current batch number and loads configuration from `.claude/config/mutation_test_config.json`.

2. Parse JSON response (output is on stdout):
- `status` - "SUCCESS", "RETRY_ONLY", "FAILURES_DETECTED", or "ERROR"
- `batch` - {number, total_batches}
- `stats` - {types_tested, passed, failed, missing_components, remaining_types}
- `retry_failures` - array of retry failure summaries
- `review_failures` - array of review failure summaries
- `warnings` - array of warning messages
- `retry_log_file` - path to detailed retry failure log (null if none)
- `review_log_file` - path to detailed review failure log (null if none)
- `diagnostic_info` - array of diagnostic entries for all tested types

3. Store the complete JSON output for use in <CheckForFailures/>, <TestResultOutput/>, <FinalDiagnosticOutput/>, and <InteractiveFailureReview/>

**IMPORTANT**: DO NOT output results yet - results will be displayed in STEP 3 after cleanup.
</ProcessBatchResults>

<CheckForFailures>
Based on `status` field from ProcessBatchResults JSON:

**"SUCCESS"**:
- If STOP_AFTER_EACH_BATCH is true: EXIT batch loop
- If STOP_AFTER_EACH_BATCH is false: Continue to next batch

**"RETRY_ONLY"**:
- If STOP_AFTER_EACH_BATCH is true: EXIT batch loop (retries will be picked up on next run)
- If STOP_AFTER_EACH_BATCH is false: Continue to next batch (renumbering will retry these types)

**"FAILURES_DETECTED"**:
- EXIT batch loop (will show results and review failures in later steps)

**"ERROR"**:
- EXIT batch loop with error flag

**Note**: FinalCleanup, TestResultOutput, and Diagnostic Output happen in STEP 2-3 after batch loop exits.
</CheckForFailures>

## STEP 2: FINAL CLEANUP

<FinalCleanup>
For each port in the assignments array (stored from <GetBatchAssignments/>), execute in parallel:

```
mcp__brp__brp_shutdown(app_name="extras_plugin", port=PORT)
```

Where PORT is each assignment's port value from the assignments JSON.

Mode: SILENT (no output to user)
</FinalCleanup>


## STEP 3: RESULT OUTPUT AND DIAGNOSTICS

**ALWAYS execute both sections below, regardless of STOP_AFTER_EACH_BATCH setting or test status.**

### Test Result Summary

<TestResultOutput>
Using the stored JSON output from <ProcessBatchResults/>, present results using the template defined in the <TestResultOutput/> section below.
</TestResultOutput>

### Diagnostic Information

<FinalDiagnosticOutput>
Using the `diagnostic_info` array from the ProcessBatchResults JSON output:

```
---

## DIAGNOSTIC INFORMATION

**Hook Debug Log**: /tmp/mutation_hook_debug.log

**Tested Types**:

| Port | Type | Status | Failed Op |
|------|------|--------|-----------|
{FOR each entry in diagnostic_info:}
| {entry.port} | `{entry.type_name}` | {entry.status} | {entry.failed_operation_id if not None else ""} |
{END FOR}

---
```

**Purpose**: Provides quick access to all test artifacts for debugging:
- Port numbers for filtering debug logs and identifying test instances
- Test plan files showing operation status
- Failed operation IDs for pinpointing issues
- Hook debug log for comprehensive execution trace
</FinalDiagnosticOutput>


## STEP 4: ASK USER TO REVIEW FAILURES

<AskUserToReviewFailures>
**Only execute if `review_failures` array is not empty in ProcessBatchResults JSON.**

After displaying TestResultOutput and FinalDiagnosticOutput, ask the user:

```
## Next Steps

Review failures were detected. Would you like to:
- **review** - Start interactive failure review
- **stop** - Stop here (you can review failures later using the log files)

Please choose an option.
```

Wait for user response:
- If user chooses "review": Proceed to STEP 5 <InteractiveFailureReview/>
- If user chooses "stop": End command execution
- If unclear: Ask for clarification
</AskUserToReviewFailures>


## STEP 5: INTERACTIVE FAILURE REVIEW

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
