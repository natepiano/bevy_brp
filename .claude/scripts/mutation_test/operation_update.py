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

  # Success with call count
  python3 mutation_test_operation_update.py \\
    --file PATH \\
    --operation-id N \\
    --status SUCCESS \\
    --entity-id 12345 \\
    --call-count 1
"""

import argparse
import json
import subprocess
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
        "--port", type=int, required=True, help="Port number (used to locate test plan file)"
    )
    _ = parser.add_argument(
        "--operation-id", type=int, required=True, help="Operation ID to update"
    )

    # Mode 1: Announcement only
    _ = parser.add_argument(
        "--announced",
        action="store_true",
        help="Mark operation as announced (set operation_announced=true)",
    )

    # Mode 2: Status update from MCP response
    _ = parser.add_argument(
        "--tool-name",
        help="MCP tool name (e.g., mcp__brp__world_query)",
    )
    _ = parser.add_argument(
        "--mcp-response",
        help="Full MCP response JSON (use '-' for stdin)",
    )

    return parser.parse_args()


def validate_args(args: argparse.Namespace) -> None:
    """Validate argument combinations."""
    announced = cast(bool, args.announced)
    mcp_response = cast(str | None, getattr(args, "mcp_response", None))
    tool_name = cast(str | None, getattr(args, "tool_name", None))

    # Must be either announcement OR status update (not both, not neither)
    if announced and mcp_response:
        print("Error: Cannot use both --announced and --mcp-response", file=sys.stderr)
        sys.exit(1)

    if not announced and not mcp_response:
        print("Error: Must provide either --announced or --mcp-response", file=sys.stderr)
        sys.exit(1)

    # If mcp-response, tool-name is required
    if mcp_response and not tool_name:
        print("Error: --tool-name required with --mcp-response", file=sys.stderr)
        sys.exit(1)


def shorten_tool_name(tool_name: str) -> str:
    """Shorten common tool names for logging."""
    tool_map = {
        "mcp__brp__world_insert_resources": "insert_resources",
        "mcp__brp__world_spawn_entity": "spawn_entity",
        "mcp__brp__world_mutate_resources": "mutate_resources",
        "mcp__brp__world_query": "query",
        "mcp__brp__world_mutate_components": "mutate_components",
    }
    return tool_map.get(tool_name, tool_name)


def validate_query_result(
    query_result_json: str,
    operation: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    current_status: str,
    current_error: str | None,
) -> tuple[str, str | None]:
    """
    Validate query result and add entity to operation if found.

    If entities found: Adds "entity" field to operation, returns SUCCESS.
    If no entities: Returns FAIL status with error message.

    Args:
        query_result_json: JSON array from MCP query response
        operation: Operation dict to update
        current_status: Current status from hook
        current_error: Current error message (if any)

    Returns:
        Tuple of (final_status, final_error)
    """
    try:
        result_data: Any = json.loads(query_result_json)  # pyright: ignore[reportExplicitAny]

        # Validate it's a list
        if not isinstance(result_data, list):
            return "FAIL", "Query result is not an array"

        # Check if we have entities
        if len(result_data) == 0:
            return "FAIL", "Query returned 0 entities"

        # Extract first entity
        first_result: Any = result_data[0]  # pyright: ignore[reportExplicitAny]
        if not isinstance(first_result, dict):
            return "FAIL", "Query result entry is not an object"

        if "entity" not in first_result:
            return "FAIL", "Query result entry missing entity field"

        entity_id: Any = first_result["entity"]  # pyright: ignore[reportExplicitAny]
        if not isinstance(entity_id, int):
            return "FAIL", f"Query result entity ID is not a number: {entity_id}"

        # Success - add entity to operation for USE_QUERY_RESULT substitution
        operation["entity"] = entity_id
        return current_status, current_error

    except json.JSONDecodeError as e:
        return "FAIL", f"Query result JSON parsing failed: {e}"
    except Exception as e:
        return "FAIL", f"Unexpected error validating query result: {e}"


def parse_mcp_response(
    mcp_response_arg: str,
    tool_name: str,
    operation: dict[str, Any],  # pyright: ignore[reportExplicitAny]
) -> tuple[str, str | None]:
    """
    Parse MCP response and extract final status/error.

    Handles:
    - Reading from stdin if arg is "-"
    - Extracting tool_response[0].text and parsing as JSON
    - Determining initial status (SUCCESS/FAIL)
    - Query validation (adds entity field or marks as FAIL)

    Args:
        mcp_response_arg: JSON string or "-" for stdin
        tool_name: MCP tool name (e.g., "mcp__brp__world_query")
        operation: Operation dict to potentially add entity field to

    Returns:
        Tuple of (final_status, final_error)
    """
    try:
        # Read from stdin if '-'
        if mcp_response_arg == "-":
            mcp_data: Any = json.load(sys.stdin)  # pyright: ignore[reportExplicitAny]
        else:
            mcp_data = json.loads(mcp_response_arg)  # pyright: ignore[reportExplicitAny]

        # Extract response JSON from tool_response[0].text
        response_text: Any = mcp_data["tool_response"][0]["text"]  # pyright: ignore[reportExplicitAny]
        response_json: Any = json.loads(response_text)  # pyright: ignore[reportExplicitAny]

        # Determine initial status from MCP response
        if response_json.get("status") == "success":
            status = "SUCCESS"
            error = None
        else:
            status = "FAIL"
            # Extract error message
            metadata: Any = response_json.get("metadata", {})  # pyright: ignore[reportExplicitAny]
            error = (
                metadata.get("original_error")
                or response_json.get("message")
                or "Unknown error"
            )

        # Special handling for query operations: validate entity availability
        if tool_name == "mcp__brp__world_query" and status == "SUCCESS":
            result: Any = response_json.get("result", [])  # pyright: ignore[reportExplicitAny]
            query_result_json = json.dumps(result)
            status, error = validate_query_result(query_result_json, operation, status, error)

        return status, error

    except Exception as e:
        return "FAIL", f"Failed to parse MCP response: {e}"


def main() -> None:
    """Main entry point."""
    args = parse_args()
    validate_args(args)

    port: int = cast(int, args.port)
    operation_id: int = cast(int, args.operation_id)
    announced: bool = cast(bool, args.announced)
    mcp_response_arg: str | None = cast(str | None, getattr(args, "mcp_response", None))
    tool_name: str | None = cast(str | None, getattr(args, "tool_name", None))

    # Get file path using shared utility
    result = subprocess.run(
        ["python3", ".claude/scripts/mutation_test/get_plan_file_path.py", "--port", str(port)],
        capture_output=True,
        text=True,
        check=True
    )
    file_path = result.stdout.strip()

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

    # Find operation by operation_id across all tests
    tests = test_plan.get("tests", [])
    if not tests:
        print("Error: No tests found in test plan", file=sys.stderr)
        sys.exit(1)

    # Search for operation across all tests (plan may contain multiple type parts)
    operation: dict[str, Any] | None = None  # pyright: ignore[reportExplicitAny]
    all_operations: list[dict[str, Any]] = []  # pyright: ignore[reportExplicitAny]

    for test in tests:
        operations = cast(list[dict[str, Any]], test.get("operations", []))  # pyright: ignore[reportExplicitAny]
        all_operations.extend(operations)

        for op in operations:
            if op.get("operation_id") == operation_id:
                if operation is not None:
                    print(
                        f"Error: Duplicate operation ID {operation_id} found",
                        file=sys.stderr,
                    )
                    sys.exit(1)
                operation = op

    if operation is None:
        print(
            f"Error: Operation ID {operation_id} not found in test plan",
            file=sys.stderr,
        )
        sys.exit(1)

    # Handle --announced flag (just mark as announced and log it)
    if announced:
        operation["operation_announced"] = True

        # Log announcement to debug log
        debug_log = "/tmp/mutation_hook_debug.log"
        try:
            from datetime import datetime
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            with open(debug_log, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} is next\n")
        except Exception:
            # Silently ignore debug log write failures
            pass

        # Write updated test plan back
        try:
            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(test_plan, f, indent=2)
        except IOError as e:
            print(f"Error: Failed to write test plan file: {e}", file=sys.stderr)
            sys.exit(1)

    # Handle MCP response (status update)
    elif mcp_response_arg and tool_name:
        # Parse MCP response and get final status/error
        status, error = parse_mcp_response(mcp_response_arg, tool_name, operation)

        # Update operation with final status
        operation["status"] = status
        operation["operation_announced"] = True

        if status == "SUCCESS":
            operation["error"] = None
        else:  # FAIL
            operation["error"] = error if error else "Unknown error"

        # Increment call_count (read current value, increment by 1)
        current_call_count: int = cast(int, operation.get("call_count", 0))
        operation["call_count"] = current_call_count + 1

        # Log status update to debug log
        debug_log = "/tmp/mutation_hook_debug.log"
        try:
            from datetime import datetime
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            tool = cast(str, operation.get("tool", "unknown"))
            short_tool = shorten_tool_name(tool)
            with open(debug_log, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} status={status} tool={short_tool}\n")
        except Exception:
            # Silently ignore debug log write failures
            pass

        # Write updated test plan back atomically
        try:
            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(test_plan, f, indent=2)
        except IOError as e:
            print(f"Error: Failed to write test plan file: {e}", file=sys.stderr)
            sys.exit(1)

        # Output final status for hook to read
        print(status, flush=True)

        # If we just completed an operation successfully, write next operation announcement
        # On FAIL, don't announce next - allow subagent to retry the failed operation
        if status == "SUCCESS":
            # Find next operation in sequence across all tests
            next_operation_id = operation_id + 1
            next_operation_exists = False

            for op in all_operations:
                if op.get("operation_id") == next_operation_id:
                    next_operation_exists = True
                    break

            # Write announcement for next operation if it exists, otherwise mark as finished
            debug_log = "/tmp/mutation_hook_debug.log"
            try:
                from datetime import datetime
                timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                with open(debug_log, "a", encoding="utf-8") as f:
                    if next_operation_exists:
                        _ = f.write(f"[{timestamp}] port={port} op_id={next_operation_id} is next\n")
                    else:
                        _ = f.write(f"[{timestamp}] port={port} **FINISHED**\n")
            except Exception:
                # Silently ignore debug log write failures
                pass


if __name__ == "__main__":
    main()
