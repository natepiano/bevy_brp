#!/usr/bin/env python3
"""
Process mutation test results from subagent test plans.
Converts test plan JSON files into batch results format for merging.

Usage:
  python3 mutation_test_process_results.py --batch 1 --max-subagents 10 --types-per-subagent 3
"""

import json
import sys
import os
import argparse
import tempfile
from datetime import datetime
from typing import Any, TypedDict, cast


# Test plan types (matching assignment script)
class TestOperation(TypedDict, total=False):
    tool: str
    port: int
    status: str | None
    error: str | None
    retry_count: int
    components: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    filter: dict[str, list[str]] | None
    data: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    result_entity_id: int | None
    result_entities: list[int] | None
    entity: str | int | None
    component: str | None
    resource: str | None
    path: str | None
    value: Any  # pyright: ignore[reportExplicitAny]


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


# Output JSON types
class FailureSummary(TypedDict):
    type: str
    status: str
    summary: str
    entity_id: int | None
    failed_at: str


class BatchInfo(TypedDict):
    number: int
    total_batches: int | None


class Stats(TypedDict):
    types_tested: int
    passed: int
    failed: int
    missing_components: int
    remaining_types: int | None


class ProcessResultsOutput(TypedDict):
    status: str
    batch: BatchInfo
    stats: Stats
    failures: list[FailureSummary]
    warnings: list[str]
    log_file: str | None


# Parse command line arguments
parser = argparse.ArgumentParser(
    description="Process mutation test results from subagent test plans"
)
_ = parser.add_argument(
    "--batch", type=int, required=True, help="Batch number to process results for"
)
_ = parser.add_argument(
    "--max-subagents", type=int, required=True, help="Maximum number of subagents"
)
_ = parser.add_argument(
    "--types-per-subagent",
    type=int,
    required=True,
    help="Number of types each subagent should test",
)

args = parser.parse_args()

batch_num: int = cast(int, args.batch)
max_subagents: int = cast(int, args.max_subagents)
types_per_subagent: int = cast(int, args.types_per_subagent)

if max_subagents <= 0:
    print(
        f"Error: max_subagents must be positive, got: {max_subagents}", file=sys.stderr
    )
    sys.exit(1)

if types_per_subagent <= 0:
    print(
        f"Error: types_per_subagent must be positive, got: {types_per_subagent}",
        file=sys.stderr,
    )
    sys.exit(1)


def convert_test_to_result(test: TypeTest) -> TestResult | None:
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
            # Check for duplicates
            if operation_id in seen_operation_ids:
                print(
                    f"Warning: Duplicate operation_id {operation_id} found in {type_name}",
                    file=sys.stderr,
                )
            seen_operation_ids.add(operation_id)

            # Check if operation_id matches index
            if operation_id != index:
                print(
                    f"Warning: operation_id {operation_id} doesn't match array index {index} in {type_name}",
                    file=sys.stderr,
                )

    # Check if subagent never executed the test (all operations have null status)
    # This indicates subagent workflow failure, not a BRP validation failure
    all_null = all(op.get("status") is None for op in operations)
    if all_null:
        warnings.append(
            f"Type {type_name} has all null status - subagent did not execute test. Will retry in next run."
        )
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

    # Process operations to determine status
    for op in operations:
        op_tool = op.get("tool", "")
        op_status = op.get("status")

        # Spawn/insert operation
        if op_tool == "mcp__brp__world_spawn_entity":
            if op_status and op_status.upper() == "SUCCESS":
                spawn_insert = True
                entity_id = op.get("result_entity_id")
            else:
                status = "FAIL"
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
                    response_received={"error": error_msg if error_msg else "Unknown error"},
                )
                break

        # Query operation
        elif op_tool == "mcp__brp__world_query":
            if op_status and op_status.upper() == "SUCCESS":
                entity_query = True
                result_entities = op.get("result_entities")
                if result_entities is None:
                    result_entities = []
                op_filter = op.get("filter")
                op_data = op.get("data")
                query_details = QueryDetails(
                    filter=op_filter if op_filter is not None else {},
                    data=op_data if op_data is not None else {},
                    entities_found=len(result_entities),
                )
                # Check for component not found
                if len(result_entities) == 0:
                    status = "COMPONENT_NOT_FOUND"
                    break
            else:
                status = "FAIL"
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
                    response_received={"error": error_msg if error_msg else "Unknown error"},
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
                    response_received={"error": error_msg if error_msg else "Unknown error"},
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
    }

    return result


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
            }

    return aggregated


# Calculate port range and read test plans
base_port = 30001
results: list[TestResult] = []
warnings: list[str] = []
tmpdir = tempfile.gettempdir()

# Determine how many subagents were actually used
# This matches the logic in the assignment script
batch_size = max_subagents * types_per_subagent
subagent_count = min(max_subagents, (batch_size + types_per_subagent - 1) // types_per_subagent)

for subagent_idx in range(subagent_count):
    port = base_port + subagent_idx
    test_plan_file = os.path.join(tmpdir, f"mutation_test_subagent_{port}_plan.json")

    # Check if file exists
    if not os.path.exists(test_plan_file):
        print(
            f"Warning: Test plan file not found: {test_plan_file}", file=sys.stderr
        )
        continue

    # Read test plan
    try:
        with open(test_plan_file, encoding="utf-8") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            test_plan = cast(TestPlan, test_plan_raw)

        # Convert each test to result format
        tests = test_plan.get("tests", [])
        for test in tests:
            result = convert_test_to_result(test)
            # Skip tests that were not executed (None = incomplete, will retry next run)
            if result is not None:
                results.append(result)

    except json.JSONDecodeError as e:
        print(f"Error: Failed to parse {test_plan_file}: {e}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: Failed to process {test_plan_file}: {e}", file=sys.stderr)
        sys.exit(1)

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

# Update type_guide entries with aggregated test results
type_guide = all_types.get("type_guide", {})  # pyright: ignore[reportAny]
for type_key, type_data in type_guide.items():  # pyright: ignore[reportAny]
    if type_key in aggregated_results:
        result = aggregated_results[type_key]
        type_data["test_status"] = "passed" if result["status"] == "PASS" else "failed"

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

# Calculate statistics
total_results = len(results)
passed = sum(1 for r in results if r["status"] == "PASS")
failed = sum(1 for r in results if r["status"] == "FAIL")
missing = sum(1 for r in results if r["status"] == "COMPONENT_NOT_FOUND")

# Calculate remaining types
# Read summary from all_types.json to get total count
summary = all_types.get("summary", {})  # pyright: ignore[reportAny]
total_types: int = cast(int, summary.get("total_types", 0))  # pyright: ignore[reportAny]
tested_count: int = cast(int, summary.get("tested_count", 0))  # pyright: ignore[reportAny]
remaining_types: int | None = total_types - tested_count if total_types > 0 else None

# Calculate total batches
total_batches: int | None = None
if total_types > 0 and batch_size > 0:
    total_batches = (total_types + batch_size - 1) // batch_size

# Check for failures and build failure summaries
failure_log_path: str | None = None
failure_summaries: list[FailureSummary] = []
all_failures_are_null_status = False

if failed > 0 or missing > 0:
    # Get all failures
    failures = [r for r in results if r["status"] in ["FAIL", "COMPONENT_NOT_FOUND"]]

    # Check if all failures are due to null status (subagent execution failures)
    null_status_failures: list[TestResult] = []
    for f in failures:
        fail_details = f.get("failure_details")
        if fail_details is not None:
            error_msg = fail_details.get("error_message", "")
            if "status field is null" in error_msg:
                null_status_failures.append(f)
    all_failures_are_null_status = len(null_status_failures) == len(failures)

    # Save detailed failure log
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    failure_log_path = f".claude/transient/all_types_failures_{timestamp}.json"
    with open(failure_log_path, "w", encoding="utf-8") as f:
        json.dump(failures, f, indent=2)

    # Build condensed failure summaries
    for failure in failures:
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

        failure_summaries.append(
            FailureSummary(
                type=failure["type"],
                status=failure["status"],
                summary=summary_msg,
                entity_id=failure.get("entity_id"),
                failed_at=failed_at,
            )
        )

# Determine final status
# NULL_STATUS_ONLY = all failures are subagent execution issues, not BRP validation errors
# FAILURES_DETECTED = at least one real BRP validation failure
# SUCCESS = no failures
final_status = "SUCCESS"
if failed > 0 or missing > 0:
    if all_failures_are_null_status:
        final_status = "NULL_STATUS_ONLY"
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
        "types_tested": total_results,
        "passed": passed,
        "failed": failed,
        "missing_components": missing,
        "remaining_types": remaining_types,
    },
    "failures": failure_summaries,
    "warnings": warnings,
    "log_file": failure_log_path,
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
