#!/usr/bin/env python3
"""
Process mutation test results from subagent test plans.
Converts test plan JSON files into batch results format for merging.

Configuration is loaded from .claude/config/mutation_test_config.json.
The batch number is auto-discovered by finding the first untested batch.

Usage:
  python3 mutation_test_process_results.py
"""

import json
import sys
import os
import glob
from datetime import datetime
from pathlib import Path
from typing import Any, TypedDict, cast

# Add script directory to path for local imports
_script_dir = Path(__file__).parent
sys.path.insert(0, str(_script_dir))

# Import shared config module - must come after sys.path modification
if True:  # Scope block for import after sys.path change
    from config import (  # type: ignore[import-not-found]
        AllTypesData,
        find_current_batch,
        load_config,
    )


# Test plan types (matching assignment script)
class TestOperation(TypedDict, total=False):
    tool: str
    port: int
    status: str | None
    error: str | None
    retry_count: int
    components: dict[str, object] | None
    filter: dict[str, list[str]] | None
    data: dict[str, object] | None
    entity: str | int | None
    component: str | None
    resource: str | None
    path: str | None
    value: object


class TypeTest(TypedDict):
    type_name: str
    mutation_type: str
    part_number: int  # Which part of this type (1-indexed)
    total_parts: int  # Total parts for this type
    operations: list[TestOperation]


class TestPlan(TypedDict):
    batch_number: int
    subagent_index: int
    port: int
    test_plan_file: str
    tests: list[TypeTest]


# Result format types (matching mutation_test.md schema)
class OperationsCompleted(TypedDict):
    spawn_insert: bool
    entity_query: bool
    mutations_passed: list[str]
    total_mutations_attempted: int


class FailureDetails(TypedDict):
    failed_operation: str
    failed_mutation_path: str | None
    error_message: str
    request_sent: dict[str, Any]  # pyright: ignore[reportExplicitAny]
    response_received: dict[str, Any]  # pyright: ignore[reportExplicitAny]


class QueryDetails(TypedDict):
    filter: dict[str, list[str]]
    data: dict[str, Any]  # pyright: ignore[reportExplicitAny]
    entities_found: int


class TestResult(TypedDict):
    type: str
    tested_type: str
    status: str
    entity_id: int | None
    part_number: int  # Which part of this type (1-indexed)
    total_parts: int  # Total parts for this type
    operations_completed: OperationsCompleted
    failure_details: FailureDetails | None
    query_details: QueryDetails | None
    test_plan_file: str
    port: int
    failed_operation_id: int | None


# Output JSON types
class FailureSummary(TypedDict):
    type: str
    status: str
    summary: str
    entity_id: int | None
    failed_at: str
    test_plan_file: str
    failed_operation_id: int | None


class BatchInfo(TypedDict):
    number: int
    total_batches: int | None


class Stats(TypedDict):
    types_tested: int
    passed: int
    failed: int
    retry: int
    missing_components: int
    remaining_types: int | None


class DiagnosticEntry(TypedDict):
    type_name: str
    failed_operation_id: int | None
    status: str  # "PASS", "RETRY", or "FAIL"
    hook_debug_log: str


class ProcessResultsOutput(TypedDict):
    status: str
    batch: BatchInfo
    stats: Stats
    retry_failures: list[FailureSummary]
    review_failures: list[FailureSummary]
    warnings: list[str]
    retry_log_file: str | None
    review_log_file: str | None
    diagnostic_info: list[DiagnosticEntry]


def cleanup_old_logs(pattern: str, keep_count: int) -> None:
    """
    Remove old log files matching pattern, keeping only the most recent N files.

    Args:
        pattern: Glob pattern to match files (e.g., ".claude/transient/all_types_retry_failures_*.json")
        keep_count: Number of most recent files to keep
    """
    # Get all matching files
    matching_files = glob.glob(pattern)

    if len(matching_files) <= keep_count:
        # Nothing to clean up
        return

    # Sort by modification time (most recent last)
    matching_files.sort(key=lambda f: os.path.getmtime(f))

    # Remove oldest files, keep only keep_count most recent
    files_to_remove = matching_files[:-keep_count]

    for filepath in files_to_remove:
        try:
            os.remove(filepath)
        except OSError:
            pass  # Ignore errors if file can't be removed


# Load configuration from config file
try:
    config = load_config()
except FileNotFoundError as e:
    print(f"Error loading config: {e}", file=sys.stderr)
    sys.exit(1)

max_subagents: int = config["max_subagents"]
ops_per_subagent: int = config["ops_per_subagent"]
base_port: int = config["base_port"]

# Get the JSON file path
json_file = ".claude/transient/all_types.json"

if not os.path.exists(json_file):
    print(f"Error: {json_file} not found!", file=sys.stderr)
    sys.exit(1)

# Load all_types.json to discover current batch
try:
    with open(json_file, "r", encoding="utf-8") as f:
        all_types_data: AllTypesData = cast(AllTypesData, json.load(f))
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Auto-discover current batch number
batch_result: int | str = find_current_batch(all_types_data)
if batch_result == "COMPLETE":
    print("All tests complete! No untested batches remaining.", file=sys.stderr)
    sys.exit(0)

# At this point batch_result must be int (we exited if it was "COMPLETE")
assert isinstance(batch_result, int)
batch_num: int = batch_result


def build_diagnostic_entry(test: TypeTest, _test_plan_file: str) -> DiagnosticEntry:
    """Build a diagnostic entry from a test.

    Args:
        test: The test data
        test_plan_file: Path to the test plan file (not used in output, kept for compatibility)

    Returns:
        Diagnostic entry with status and failure info
    """
    type_name = test["type_name"]
    operations = test["operations"]

    # Find first failed operation (if any) and determine status
    failed_op_id: int | None = None
    has_null_status = False

    for op in operations:
        op_status = op.get("status")
        if op_status is None:
            has_null_status = True
            if failed_op_id is None:
                failed_op_id = cast(int | None, op.get("operation_id"))
        elif op_status.upper() != "SUCCESS":
            if failed_op_id is None:
                failed_op_id = cast(int | None, op.get("operation_id"))

    # Determine diagnostic status
    if failed_op_id is None:
        diag_status = "PASS"
    elif has_null_status:
        diag_status = "RETRY"
    else:
        diag_status = "FAIL"

    return cast(
        DiagnosticEntry,
        cast(
            object,
            {
                "type_name": type_name,
                "failed_operation_id": failed_op_id,
                "status": diag_status,
                "hook_debug_log": "/tmp/mutation_hook_debug.log",
            },
        ),
    )


def convert_test_to_result(
    test: TypeTest,
    null_status_types: dict[str, list[tuple[int, int]]],
    test_plan_file: str,
    port: int,
) -> TestResult | None:
    """Convert a test plan test to a result format.

    Returns None if the test was not executed (all operations have null status),
    indicating the type should remain untested for retry in the next run.
    """
    type_name = test["type_name"]
    operations = test["operations"]

    # Validate operation_id fields
    seen_operation_ids: set[int] = set()
    for index, op in enumerate(operations):
        operation_id: int | None = cast(int | None, op.get("operation_id"))

        # Check operation_id exists
        if operation_id is None:
            print(
                f"Warning: Operation at index {index} in {type_name} missing operation_id field",
                file=sys.stderr,
            )
        else:
            # Check for duplicates within this test plan file
            if operation_id in seen_operation_ids:
                print(
                    f"Warning: Duplicate operation_id {operation_id} found in {type_name}",
                    file=sys.stderr,
                )
            seen_operation_ids.add(operation_id)

    # Check if subagent never executed the test (all operations have null status)
    # This indicates subagent workflow failure, not a BRP validation failure
    all_null = all(op.get("status") is None for op in operations)
    if all_null:
        # Track this type and part for grouped warning output
        part_num = test.get("part_number", 1)
        total_parts = test.get("total_parts", 1)
        if type_name not in null_status_types:
            null_status_types[type_name] = []
        null_status_types[type_name].append((part_num, total_parts))
        return None

    # Initialize result
    entity_id: int | None = None
    spawn_insert = False
    entity_query = False
    mutations_passed: list[str] = []
    total_mutations_attempted = 0
    status = "PASS"
    failure_details: FailureDetails | None = None
    query_details: QueryDetails | None = None
    failed_operation_id: int | None = None

    # Process operations to determine status
    for op in operations:
        op_tool = op.get("tool", "")
        op_status = op.get("status")

        # Spawn/insert operation
        if op_tool == "mcp__brp__world_spawn_entity":
            if op_status and op_status.upper() == "SUCCESS":
                spawn_insert = True
                # Note: entity_id tracking removed - not needed for validation
            else:
                status = "FAIL"
                failed_operation_id = cast(int | None, op.get("operation_id"))
                error_msg = op.get("error")
                # Detect subagent failure (never executed operation)
                if op_status is None:
                    error_msg = "Subagent failure - operation not executed (status field is null)"
                failure_details = FailureDetails(
                    failed_operation="spawn",
                    failed_mutation_path=None,
                    error_message=error_msg if error_msg else "Unknown error",
                    request_sent={
                        "components": op.get("components", {}),
                        "port": op.get("port"),
                    },
                    response_received={
                        "error": error_msg if error_msg else "Unknown error"
                    },
                )
                break

        # Query operation
        elif op_tool == "mcp__brp__world_query":
            if op_status and op_status.upper() == "SUCCESS":
                entity_query = True
                op_filter = op.get("filter")
                op_data = op.get("data")
                query_details = QueryDetails(
                    filter=op_filter if op_filter is not None else {},
                    data=op_data if op_data is not None else {},
                    entities_found=0,  # Not tracked anymore - query success is sufficient
                )

            else:
                status = "FAIL"
                failed_operation_id = cast(int | None, op.get("operation_id"))
                error_msg = op.get("error")
                # Detect subagent failure (never executed operation)
                if op_status is None:
                    error_msg = "Subagent failure - operation not executed (status field is null)"
                op_filter = op.get("filter")
                op_data = op.get("data")
                failure_details = FailureDetails(
                    failed_operation="query",
                    failed_mutation_path=None,
                    error_message=error_msg if error_msg else "Unknown error",
                    request_sent={
                        "filter": op_filter if op_filter is not None else {},
                        "data": op_data if op_data is not None else {},
                        "port": op.get("port"),
                    },
                    response_received={
                        "error": error_msg if error_msg else "Unknown error"
                    },
                )
                break

        # Mutation operation (component or resource)
        elif op_tool in [
            "mcp__brp__world_mutate_components",
            "mcp__brp__world_mutate_resources",
        ]:
            total_mutations_attempted += 1
            mutation_path = op.get("path")

            if op_status and op_status.upper() == "SUCCESS":
                if mutation_path is not None:
                    mutations_passed.append(mutation_path)
            else:
                status = "FAIL"
                failed_operation_id = cast(int | None, op.get("operation_id"))
                error_msg = op.get("error")
                # Detect subagent failure (never executed operation)
                if op_status is None:
                    error_msg = "Subagent failure - operation not executed (status field is null)"
                failure_details = FailureDetails(
                    failed_operation="mutation",
                    failed_mutation_path=mutation_path,
                    error_message=error_msg if error_msg else "Unknown error",
                    request_sent={
                        "entity": op.get("entity"),
                        "component": op.get("component"),
                        "resource": op.get("resource"),
                        "path": mutation_path,
                        "value": op.get("value"),
                        "port": op.get("port"),
                    },
                    response_received={
                        "error": error_msg if error_msg else "Unknown error"
                    },
                )
                break

    # Build final result
    result: TestResult = {
        "type": type_name,
        "tested_type": type_name,
        "status": status,
        "entity_id": entity_id,
        "part_number": test.get("part_number", 1),
        "total_parts": test.get("total_parts", 1),
        "operations_completed": {
            "spawn_insert": spawn_insert,
            "entity_query": entity_query,
            "mutations_passed": mutations_passed,
            "total_mutations_attempted": total_mutations_attempted,
        },
        "failure_details": failure_details,
        "query_details": query_details,
        "test_plan_file": test_plan_file,
        "port": port,
        "failed_operation_id": failed_operation_id,
    }

    return result


def is_retry_failure(result: TestResult) -> bool:
    """
    Determine if a failure should be retried (subagent crash) vs reviewed (real BRP error).

    Retry scenarios:
    - Subagent crashed mid-execution (some operations succeeded, rest are null)
    - Error message contains "status field is null"

    Review scenarios:
    - Got actual BRP error response (like "0 entities found")
    """
    fail_details = result.get("failure_details")
    if fail_details is None:
        return False

    error_msg = fail_details.get("error_message", "")
    return "status field is null" in error_msg


def aggregate_results_by_type(results: list[TestResult]) -> dict[str, TestResult]:
    """
    Aggregate multi-part results by type.
    ANY part failure = type failure.

    Returns dict of type_name -> aggregated TestResult
    """
    type_parts: dict[str, list[TestResult]] = {}

    # Group results by type
    for result in results:
        type_name = result["type"]
        if type_name not in type_parts:
            type_parts[type_name] = []
        type_parts[type_name].append(result)

    aggregated: dict[str, TestResult] = {}

    for type_name, parts in type_parts.items():
        # Sort by part_number
        parts.sort(key=lambda p: p.get("part_number", 1))

        # Check if any part failed
        failed_part: TestResult | None = None
        for part in parts:
            if part["status"] != "PASS":
                failed_part = part
                break

        # Aggregate operations
        total_mutations = sum(
            p["operations_completed"]["total_mutations_attempted"] for p in parts
        )
        all_mutations_passed: list[str] = []
        for p in parts:
            all_mutations_passed.extend(p["operations_completed"]["mutations_passed"])

        # Build aggregated result
        if failed_part:
            # Use failed part's result (already has part_number info)
            aggregated[type_name] = failed_part
        else:
            # Use first part as base, mark as passed
            base = parts[0]
            aggregated[type_name] = {
                "type": type_name,
                "tested_type": type_name,
                "status": "PASS",
                "entity_id": base.get("entity_id"),
                "part_number": 1,
                "total_parts": parts[0].get("total_parts", 1),
                "operations_completed": {
                    "spawn_insert": any(
                        p["operations_completed"]["spawn_insert"] for p in parts
                    ),
                    "entity_query": any(
                        p["operations_completed"]["entity_query"] for p in parts
                    ),
                    "mutations_passed": all_mutations_passed,
                    "total_mutations_attempted": total_mutations,
                },
                "failure_details": None,
                "query_details": base.get("query_details"),
                "test_plan_file": base.get("test_plan_file", ""),
                "port": base.get("port", 0),
                "failed_operation_id": None,
            }

    return aggregated


# Calculate port range and read test plans
base_port = 30001
results: list[TestResult] = []
warnings: list[str] = []
# Track types with null status by type_name -> list of (part_number, total_parts)
null_status_types: dict[str, list[tuple[int, int]]] = {}
# Diagnostic entries for all tested types (built during first loop)
diagnostic_entries: list[DiagnosticEntry] = []
# Use /tmp consistently with prepare.py and get_plan_file_path.py
tmpdir = "/tmp"

# Determine how many subagents were actually used
# With operation-based packing, we use up to max_subagents
# The actual count depends on how many test plans were created
subagent_count = max_subagents

for subagent_idx in range(subagent_count):
    port = base_port + subagent_idx
    test_plan_file = os.path.join(tmpdir, f"mutation_test_{port}.json")
    # Normalize path for display (use /tmp/ instead of full macOS path)
    normalized_test_plan_file = test_plan_file.replace(tmpdir, "/tmp")

    # Check if file exists
    if not os.path.exists(test_plan_file):
        print(f"Warning: Test plan file not found: {test_plan_file}", file=sys.stderr)
        continue

    # Read test plan
    try:
        with open(test_plan_file, encoding="utf-8") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            test_plan = cast(TestPlan, test_plan_raw)

        # Convert each test to result format AND build diagnostic entries
        tests = test_plan.get("tests", [])
        for test in tests:
            # Build diagnostic entry for all tests
            diagnostic_entries.append(
                build_diagnostic_entry(test, normalized_test_plan_file)
            )

            # Build result for executed tests
            result = convert_test_to_result(
                test, null_status_types, normalized_test_plan_file, port
            )
            # Skip tests that were not executed (None = incomplete, will retry next run)
            if result is not None:
                results.append(result)

    except json.JSONDecodeError as e:
        print(f"Error: Failed to parse {test_plan_file}: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: Failed to process {test_plan_file}: {e}", file=sys.stderr)
        sys.exit(1)

# Note: Warnings for null status types removed - this information is shown in diagnostic table

# Write batch results to temp file
batch_results_file = f".claude/transient/batch_results_{batch_num}.json"
with open(batch_results_file, "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2)

# Merge results into all_types.json
all_types_file = ".claude/transient/all_types.json"
if not os.path.exists(all_types_file):
    print(f"Error: {all_types_file} not found!", file=sys.stderr)
    sys.exit(1)

# Read all_types.json
with open(all_types_file, encoding="utf-8") as f:
    all_types_raw = json.load(f)  # pyright: ignore[reportAny]
    all_types = cast(dict[str, Any], all_types_raw)  # pyright: ignore[reportExplicitAny]

# Aggregate multi-part results (ANY part failure = type failure)
aggregated_results = aggregate_results_by_type(results)

# Calculate statistics from aggregated results (unique types only)
# Include types from both aggregated_results and null_status_types
all_tested_types = set(aggregated_results.keys()) | set(null_status_types.keys())
total_types_tested = len(all_tested_types)

# Identify types with incomplete execution (appear in both aggregated and null status)
# These types had SOME parts execute but OTHER parts didn't - they're retries
incomplete_types = set(aggregated_results.keys()) & set(null_status_types.keys())

# Update type_guide entries with aggregated test results
type_guide = all_types.get("type_guide", {})  # pyright: ignore[reportAny]
for type_key, type_data in type_guide.items():  # pyright: ignore[reportAny]
    if type_key in aggregated_results:
        result = aggregated_results[type_key]

        # Check if type has incomplete execution (some parts didn't execute)
        if type_key in incomplete_types:
            # Mark as failed - not all parts completed
            type_data["test_status"] = "failed"
            type_data["fail_reason"] = "Incomplete execution - some parts not executed"
        else:
            # All parts executed - use aggregated status
            type_data["test_status"] = (
                "passed" if result["status"] == "PASS" else "failed"
            )

            # Build fail_reason with part info
            if result["status"] != "PASS":
                failure_details = result.get("failure_details")
                fail_parts: list[str] = []

                # Include part number if type was split
                if result.get("total_parts", 1) > 1:
                    fail_parts.append(
                        f"Part {result.get('part_number', 1)}/{result.get('total_parts', 1)}"
                    )

                if failure_details:
                    fail_parts.append(failure_details.get("error_message", ""))
                    failed_path = failure_details.get("failed_mutation_path")
                    if failed_path:
                        fail_parts.append(f"Path: {failed_path}")

                type_data["fail_reason"] = " | ".join(fail_parts)
            else:
                type_data["fail_reason"] = ""

# Write updated all_types.json
with open(all_types_file, "w", encoding="utf-8") as f:
    json.dump(all_types, f, indent=2)

# Count statistics:
# PASS: In aggregated_results with status=PASS AND not in null_status_types (all parts executed)
# FAIL: In aggregated_results with status=FAIL and not a retry failure
# RETRY: In aggregated_results with status=FAIL and is a retry failure, OR has incomplete execution
# MISSING: In aggregated_results with status=COMPONENT_NOT_FOUND

passed = sum(
    1
    for type_name, r in aggregated_results.items()
    if r["status"] == "PASS" and type_name not in incomplete_types
)
failed = sum(
    1
    for r in aggregated_results.values()
    if r["status"] == "FAIL" and not is_retry_failure(r)
)
retry = sum(
    1
    for r in aggregated_results.values()
    if r["status"] == "FAIL" and is_retry_failure(r)
)
missing = sum(
    1 for r in aggregated_results.values() if r["status"] == "COMPONENT_NOT_FOUND"
)

# Count types with incomplete execution as retries
retry += len(incomplete_types)

# Count types that were not executed at all (all null status - these are retries too)
# Only count types that don't appear in aggregated_results at all
null_only_types = set(null_status_types.keys()) - set(aggregated_results.keys())
retry += len(null_only_types)

# Calculate remaining types
# Read summary from all_types.json to get total count
summary = all_types.get("summary", {})  # pyright: ignore[reportAny]
total_types: int = cast(int, summary.get("total_types", 0))  # pyright: ignore[reportAny]
tested_count: int = cast(int, summary.get("tested_count", 0))  # pyright: ignore[reportAny]
remaining_types: int | None = total_types - tested_count if total_types > 0 else None

# Calculate total batches from actual batch assignments
# Note: With operation-based packing, batch count is determined by actual packing results
# We find the max batch number from the type_guide
total_batches: int | None = max(
    (t.get("batch_number") or 0 for t in all_types_data["type_guide"].values()),
    default=None
) if all_types_data else None

# Check for failures and build failure summaries
retry_log_path: str | None = None
review_log_path: str | None = None
retry_summaries: list[FailureSummary] = []
review_summaries: list[FailureSummary] = []

if failed > 0 or missing > 0:
    # Get all failures
    all_failures = [
        r for r in results if r["status"] in ["FAIL", "COMPONENT_NOT_FOUND"]
    ]

    # Classify failures: retry vs review
    retry_failures: list[TestResult] = []
    review_failures: list[TestResult] = []

    for failure in all_failures:
        if is_retry_failure(failure):
            retry_failures.append(failure)
        else:
            review_failures.append(failure)

    # Save detailed failure logs (separate files for retry vs review)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    if retry_failures:
        retry_log_path = f".claude/transient/all_types_retry_failures_{timestamp}.json"
        with open(retry_log_path, "w", encoding="utf-8") as f:
            json.dump(retry_failures, f, indent=2)

        # Cleanup old retry failure logs, keep only 2 most recent
        cleanup_old_logs(
            ".claude/transient/all_types_retry_failures_*.json", keep_count=2
        )

    if review_failures:
        review_log_path = (
            f".claude/transient/all_types_review_failures_{timestamp}.json"
        )
        with open(review_log_path, "w", encoding="utf-8") as f:
            json.dump(review_failures, f, indent=2)

        # Cleanup old review failure logs, keep only 2 most recent
        cleanup_old_logs(
            ".claude/transient/all_types_review_failures_*.json", keep_count=2
        )

    # Build condensed failure summaries
    def build_summary(failure: TestResult) -> FailureSummary:
        fail_details = failure.get("failure_details")

        # Determine summary message
        if failure["status"] == "COMPONENT_NOT_FOUND":
            summary_msg = "Component not found after spawn"
        elif fail_details:
            summary_msg = fail_details.get("error_message", "Unknown error")
        else:
            summary_msg = "Unknown error"

        # Determine failed_at
        failed_at = "unknown"
        if fail_details:
            failed_op = fail_details.get("failed_operation", "unknown")
            failed_path = fail_details.get("failed_mutation_path")
            if failed_path:
                failed_at = f"{failed_op} ({failed_path})"
            else:
                failed_at = failed_op

        # Add part info to failed_at if type was split
        part_num = failure.get("part_number", 1)
        total_parts = failure.get("total_parts", 1)
        if total_parts > 1:
            failed_at = f"{failed_at} [Part {part_num}/{total_parts}]"

        return FailureSummary(
            type=failure["type"],
            status=failure["status"],
            summary=summary_msg,
            entity_id=failure.get("entity_id"),
            failed_at=failed_at,
            test_plan_file=failure.get("test_plan_file", ""),
            failed_operation_id=failure.get("failed_operation_id"),
        )

    retry_summaries = [build_summary(f) for f in retry_failures]
    review_summaries = [build_summary(f) for f in review_failures]

# Determine final status
# RETRY_ONLY = only retry failures (subagent crashes), will be retried automatically
# FAILURES_DETECTED = at least one real BRP validation failure needing review
# SUCCESS = no failures
final_status = "SUCCESS"
if failed > 0 or missing > 0:
    if len(review_summaries) == 0:
        final_status = "RETRY_ONLY"
    else:
        final_status = "FAILURES_DETECTED"

# Build output JSON
output: ProcessResultsOutput = {
    "status": final_status,
    "batch": {
        "number": batch_num,
        "total_batches": total_batches,
    },
    "stats": {
        "types_tested": total_types_tested,
        "passed": passed,
        "failed": failed,
        "retry": retry,
        "missing_components": missing,
        "remaining_types": remaining_types,
    },
    "retry_failures": retry_summaries,
    "review_failures": review_summaries,
    "warnings": warnings,
    "retry_log_file": retry_log_path,
    "review_log_file": review_log_path,
    "diagnostic_info": diagnostic_entries,
}

# Output JSON to stdout
print(json.dumps(output, indent=2))

# Cleanup batch results file
os.remove(batch_results_file)

# Exit with appropriate code
if failed > 0 or missing > 0:
    sys.exit(2)  # Failures exist
else:
    sys.exit(0)  # Success
