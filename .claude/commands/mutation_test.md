# Type Guide Comprehensive Validation Test

**Command Type**: Runs every batch through `run.py`, then offers an interactive review of review failures.

<ExecutionFlow>
**STEP 1:** Execute <RunMutationTest/>
**STEP 2:** Execute <PresentSummary/>
**STEP 3:** Execute <AskUserToReviewFailures/> (ONLY if `review_failures` is non-empty)
**STEP 4:** Execute <InteractiveFailureReview/> (ONLY if the user chooses review)
</ExecutionFlow>

## STEP 1: RUN

<RunMutationTest>
Run in the background (`run_in_background: true`) and wait for the completion notification:
```bash
python3 .claude/scripts/mutation_test/run.py
```
Append `--max-batches N` only when the user gives a batch limit.

It builds bevy_brp_mcp, runs every batch (prepare.py → launch → operations → shutdown → process_results.py), and continues past failures. Progress goes to stderr; the concise JSON summary is the last thing on stdout. The full summary is `.claude/transient/mutation_run_summary.json`.
</RunMutationTest>

## STEP 2: SUMMARY

<PresentSummary>
From the concise stdout JSON:

```
## Mutation Test Results

**Status**: {status} ({stop_reason})
**Wall time**: {wall_seconds}s over {batches_run} batches
**Totals**: {types_tested} tested, {passed} passed, {failed} failed, {missing_components} missing components, {retry} retry
**All types**: {all_types_status_counts}

| Tool | n | median ms | p90 ms |
|------|---|-----------|--------|
{one row per latency_ms entry}

{IF review_failures:}
**Review failures**:
{index}. `{type}` - op {failed_operation_id}: {error}
{END IF}

**Summary file**: {summary_file}
```

Status handling:
- **SUCCESS**: done.
- **FAILURES_DETECTED**: continue to STEP 3.
- **RUNNER_PROBLEM**: a runner defect, not a type failure — any retry or a batch with no progress means run.py/operation_manager.py mishandled an operation. Report `retry_failure_types` and `stop_reason` as a runner bug; review any `review_failures` as usual.
- **INCOMPLETE**: stopped before every batch ran (e.g. `--max-batches`).
- **ERROR**: report `stop_reason`; each batch's `error` field in the summary file has details.
</PresentSummary>

## STEP 3: ASK USER TO REVIEW FAILURES

<AskUserToReviewFailures>
```
## Next Steps

Review failures were detected. Would you like to:
- **review** - Start interactive failure review
- **stop** - Stop here (the summary file keeps every failure)

Please choose an option.
```

- "review": proceed to STEP 4
- "stop": end
- unclear: ask again
</AskUserToReviewFailures>

## STEP 4: INTERACTIVE FAILURE REVIEW

<InteractiveFailureReview>
1. Read `review_failures` from `.claude/transient/mutation_run_summary.json`. Each record holds `type`, `entity_id`, `port`, `failed_operation_id`, `operations_completed`, `failure_details` (`failed_operation`, `failed_mutation_path`, `error_message`, `request_sent`, `response_received`), `query_details`, `archived_test_plan_file` and `archived_log_file`. Always use the archived files: the live `.claude/transient/mutation_test.log` and `mutation_test_port_{port}.json` hold only the last batch.

2. TodoWrite: one "Review failure [X] of [TOTAL]" per failure.

3. For each failure, present:
```
## FAILURE [X] of [TOTAL]: `[type]`

### Overview
- **Entity ID**: [entity_id]
- **Total Mutations**: [total_mutations_attempted] attempted
- **Mutations Passed**: [count of mutations_passed] succeeded
- **Failed At**: [failed_operation or failed_mutation_path]

### What Succeeded Before Failure
[List successful operations]

### The Failure

**Failed [Operation/Path]**: [specific failure point]

**What We Sent**:
```json
[request_sent]
```

**Error Response**:
```json
[response_received]
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

4. Execute <CheckCommonPatterns/> right after presenting each failure.
5. Wait for the keyword: investigate (already done), skip (next failure), stop (exit).
</InteractiveFailureReview>

<CheckCommonPatterns>
For each pattern, run the quick diagnosis. If the signature matches, execute the pattern section.

**Pattern 1: BRP Connection Lost**

```bash
grep "port={port}" {archived_log_file} | tail -20
```

Signature:
- Multiple `status=SUCCESS` operations
- Sudden "HTTP request failed" or "Connection failed"

**If signature matches**: Execute <BRPConnectionLost/>

---

**No patterns matched**: Execute <InvestigateFailure/>
</CheckCommonPatterns>

<BRPConnectionLost>
1. Parse {archived_log_file} for port {port}:
   - Last successful operation timestamp
   - Failed operation details
   - Time gap

2. Read {archived_test_plan_file} to find the last successful operation:
   - Extract operation details (component, path, value)
   - This mutation likely crashed the app

Present findings:
```
✅ PATTERN: BRP Connection Lost

BRP server connection failed after {N} successful operations.

Log Evidence:
- Port {port}: op_id={last_success_id} → SUCCESS at {timestamp1}
- Port {port}: op_id={fail_id} → FAIL at {timestamp2}
- Gap: {seconds}s

Likely Culprit - Last Successful Mutation:
- Component: `{component}`
- Path: `{path}`
- Value: {value}
- Type: `{type_name}`

Root Cause:
The mutation succeeded from BRP's perspective, but caused the app to crash
or become unresponsive shortly after, breaking the connection for subsequent operations.

**Investigation Focus**:
This specific mutation is likely incompatible with the component's implementation:
1. The value may violate invariants not checked by BRP
2. The component may have unsafe code that panics on this value
3. The mutation may trigger a cascade failure in dependent systems

**Recommended Fix**:
1. Test this exact mutation in isolation to reproduce the crash
2. Check {component} implementation for panics or unsafe code
3. Add validation or mark this mutation path as invalid if appropriate
```
</BRPConnectionLost>

<InvestigateFailure>
1. Run: `.claude/scripts/mutation_test/get_type_guide.sh <failed_type_name> --file .claude/transient/all_types.json`
2. Examine type guide for failed mutation path
3. Check `path_info` for `applicable_variants`, `example`/`unavailable_reason`, `mutability`
4. Present findings and recommendations
5. Do NOT launch Task agents
</InvestigateFailure>
