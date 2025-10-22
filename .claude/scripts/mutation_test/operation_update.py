#!/usr/bin/env python3
"""
Atomically update a single operation in a mutation test plan file.

Usage:
  # Success (no additional parameters needed)
  python3 mutation_test_operation_update.py \\
    --file PATH \\
    --operation-id N \\
    --status SUCCESS

  # Failure with error message
  python3 mutation_test_operation_update.py \\
    --file PATH \\
    --operation-id N \\
    --status FAIL \\
    --error "Framework error: Unable to extract parameters"

  # Success with retry count
  python3 mutation_test_operation_update.py \\
    --file PATH \\
    --operation-id N \\
    --status SUCCESS \\
    --entity-id 12345 \\
    --retry-count 1
"""

import argparse
import json
import sys
from typing import Any, TypedDict, cast


class TestPlan(TypedDict):
    """Type for test plan file structure."""

    batch_number: int
    subagent_index: int
    port: int
    test_plan_file: str
    tests: list[dict[str, Any]]  # pyright: ignore[reportExplicitAny]


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Update a single operation in a mutation test plan file"
    )
    _ = parser.add_argument(
        "--file", required=True, help="Path to test plan JSON file"
    )
    _ = parser.add_argument(
        "--operation-id", type=int, required=True, help="Operation ID to update"
    )
    _ = parser.add_argument(
        "--status",
        required=True,
        choices=["SUCCESS", "FAIL"],
        help="Operation status (SUCCESS or FAIL)",
    )
    _ = parser.add_argument("--error", help="Error message (for failed operations)")
    _ = parser.add_argument(
        "--retry-count", type=int, default=0, help="Retry count (default: 0)"
    )

    return parser.parse_args()


def validate_args(args: argparse.Namespace) -> None:
    """Validate argument combinations."""
    # Validate retry_count is non-negative
    retry_arg = cast(int, args.retry_count)
    if retry_arg < 0:
        print(f"Error: --retry-count must be non-negative, got {retry_arg}", file=sys.stderr)
        sys.exit(1)


def main() -> None:
    """Main entry point."""
    args = parse_args()
    validate_args(args)

    file_path: str = cast(str, args.file)
    operation_id: int = cast(int, args.operation_id)
    status: str = cast(str, args.status)
    error: str | None = cast(str | None, args.error)
    retry_count: int = cast(int, args.retry_count)

    # Read test plan file
    try:
        with open(file_path, encoding="utf-8") as f:
            test_plan: TestPlan = cast(TestPlan, json.load(f))
    except FileNotFoundError:
        print(f"Error: Test plan file not found: {file_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in test plan file: {e}", file=sys.stderr)
        sys.exit(1)

    # Find operation by operation_id
    tests = test_plan.get("tests", [])
    if not tests:
        print("Error: No tests found in test plan", file=sys.stderr)
        sys.exit(1)

    # We only have one test per plan currently
    test = tests[0]
    operations = cast(list[dict[str, Any]], test.get("operations", []))  # pyright: ignore[reportExplicitAny]

    # Find operation with matching operation_id
    operation: dict[str, Any] | None = None  # pyright: ignore[reportExplicitAny]
    operation_index: int = -1
    for idx, op in enumerate(operations):
        if op.get("operation_id") == operation_id:
            if operation is not None:
                print(
                    f"Error: Duplicate operation ID {operation_id} found",
                    file=sys.stderr,
                )
                sys.exit(1)
            operation = op
            operation_index = idx

    if operation is None:
        print(
            f"Error: Operation ID {operation_id} not found in test plan",
            file=sys.stderr,
        )
        sys.exit(1)

    # Update operation fields
    operation["status"] = status

    if status == "SUCCESS":
        operation["error"] = None
    else:  # FAIL
        operation["error"] = error if error else "Unknown error"
        # Don't set result fields on failure

    # Set retry_count if provided (or default to 0)
    if "retry_count" in operation or retry_count > 0:
        operation["retry_count"] = retry_count

    # Write updated test plan back atomically
    try:
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(test_plan, f, indent=2)
    except IOError as e:
        print(f"Error: Failed to write test plan file: {e}", file=sys.stderr)
        sys.exit(1)

    # Output next operation ID or completion message
    if operation_index + 1 < len(operations):
        next_op_id = operations[operation_index + 1].get("operation_id")
        print(f"Next operation id: {next_op_id}")
    else:
        print("testing_complete")


if __name__ == "__main__":
    main()
