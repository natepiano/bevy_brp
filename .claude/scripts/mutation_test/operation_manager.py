#!/usr/bin/env python3
"""
Unified operation manager for mutation testing.

Handles both:
1. Getting next operation (for subagent)
2. Updating operation status (for hook)

Usage:
  # Get next operation
  python3 operation_manager.py --port 30001 --action get-next

  # Update operation status
  echo "$MCP_RESPONSE" | python3 operation_manager.py \\
    --port 30001 \\
    --action update \\
    --tool-name mcp__brp__world_spawn_entity \\
    --mcp-response -
"""

import argparse
import json
import subprocess
import sys
from datetime import datetime
from typing import Any, TypedDict, cast


def load_config() -> dict[str, Any]:  # pyright: ignore[reportExplicitAny]
    """Load mutation test configuration."""
    config_path = ".claude/config/mutation_test_config.json"
    try:
        with open(config_path, encoding="utf-8") as f:
            config_data = json.load(f)  # pyright: ignore[reportAny]
            return cast(dict[str, Any], config_data)  # pyright: ignore[reportExplicitAny]
    except FileNotFoundError:
        print(f"Error: Config file not found: {config_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in config file: {e}", file=sys.stderr)
        sys.exit(1)


# Load configuration at module level
CONFIG = load_config()
MUTATION_TEST_LOG = cast(str, CONFIG["mutation_test_log"])


class QueryResultEntry(TypedDict):
    """Type for a single query result entry."""

    entity: int


class HookToolResponse(TypedDict):
    """Type for hook tool_response element."""

    text: str


class HookEvent(TypedDict):
    """Type for hook event JSON structure."""

    tool_response: list[HookToolResponse]
    tool_name: str
    tool_input: dict[str, object]


class BrpResponseMetadata(TypedDict, total=False):
    """Type for BRP response metadata field."""

    original_error: str


class BrpResponse(TypedDict, total=False):
    """Type for BRP response structure."""

    status: str
    message: str
    metadata: BrpResponseMetadata
    result: list[QueryResultEntry]


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
        description="Unified operation manager for mutation testing"
    )
    _ = parser.add_argument(
        "--port", type=int, required=True, help="Port number (used to locate test plan file)"
    )
    _ = parser.add_argument(
        "--action",
        required=True,
        choices=["get-next", "update"],
        help="Action to perform: get-next or update",
    )

    # Update-specific arguments
    _ = parser.add_argument(
        "--tool-name",
        help="MCP tool name (required for update action)",
    )
    _ = parser.add_argument(
        "--mcp-response",
        help="Full MCP response JSON (use '-' for stdin, required for update action)",
    )

    return parser.parse_args()


def validate_args(args: argparse.Namespace) -> None:
    """Validate argument combinations."""
    action = cast(str, args.action)

    if action == "update":
        tool_name = cast(str | None, getattr(args, "tool_name", None))
        mcp_response = cast(str | None, getattr(args, "mcp_response", None))

        if not tool_name:
            print("Error: --tool-name is required for update action", file=sys.stderr)
            sys.exit(1)

        if not mcp_response:
            print("Error: --mcp-response is required for update action", file=sys.stderr)
            sys.exit(1)


def get_plan_file_path(port: int) -> str:
    """Get test plan file path for given port."""
    try:
        result = subprocess.run(
            [
                "python3",
                ".claude/scripts/mutation_test/get_plan_file_path.py",
                "--port",
                str(port),
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except subprocess.CalledProcessError as e:
        stderr_msg = cast(str, e.stderr)
        print(f"Error: Failed to get test plan path: {stderr_msg}", file=sys.stderr)
        sys.exit(1)


def load_test_plan(file_path: str) -> TestPlan:
    """Load test plan from file."""
    try:
        with open(file_path, encoding="utf-8") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            return cast(TestPlan, test_plan_raw)
    except FileNotFoundError:
        print(f"Error: Test plan file not found: {file_path}", file=sys.stderr)
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in test plan file: {e}", file=sys.stderr)
        sys.exit(1)


def save_test_plan(file_path: str, test_plan: TestPlan) -> None:
    """Save test plan to file atomically."""
    try:
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(test_plan, f, indent=2)
    except IOError as e:
        print(f"Error: Failed to write test plan file: {e}", file=sys.stderr)
        sys.exit(1)


def find_next_operation(
    test_plan: TestPlan,
) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:  # pyright: ignore[reportExplicitAny]
    """
    Find first operation that needs execution.

    Returns:
        Tuple of (operation, test) or (None, None) if all complete
    """
    tests = test_plan.get("tests", [])
    if not tests:
        return None, None

    for test in tests:
        operations = cast(list[dict[str, Any]], test.get("operations", []))  # pyright: ignore[reportExplicitAny]
        for op in operations:
            status = op.get("status")
            # Return operations that need execution (no status or FAIL)
            if status is None or status == "FAIL":
                return op, test

    # No operations found needing execution
    return None, None


def get_execution_params(operation: dict[str, Any]) -> dict[str, Any]:  # pyright: ignore[reportExplicitAny]
    """
    Extract execution parameters from operation, excluding tracking fields.

    Keeps operation_id for hook identification.
    """
    exclude_fields = {
        "operation_announced",
        "status",
        "error",
        "call_count",
    }

    return {k: v for k, v in operation.items() if k not in exclude_fields}  # pyright: ignore[reportAny]


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

    Returns:
        Tuple of (final_status, final_error)
    """
    try:
        result_data_raw = json.loads(query_result_json)  # pyright: ignore[reportAny]

        if not isinstance(result_data_raw, list):
            return "FAIL", "Query result is not an array"

        result_data = cast(list[dict[str, object]], result_data_raw)

        if len(result_data) == 0:
            return "FAIL", "Query returned 0 entities"

        first_result = result_data[0]
        if not isinstance(first_result, dict):  # pyright: ignore[reportUnnecessaryIsInstance]
            return "FAIL", "Query result entry is not an object"

        if "entity" not in first_result:
            return "FAIL", "Query result entry missing entity field"

        entity_id = first_result["entity"]
        if not isinstance(entity_id, int):
            return "FAIL", f"Query result entity ID is not a number: {entity_id}"

        # Success - add entity to operation
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

    Returns:
        Tuple of (final_status, final_error)
    """
    mcp_data: Any = None  # pyright: ignore[reportExplicitAny]
    try:
        # Read from stdin if '-'
        if mcp_response_arg == "-":
            mcp_data = json.load(sys.stdin)  # pyright: ignore[reportExplicitAny]
        else:
            mcp_data = json.loads(mcp_response_arg)  # pyright: ignore[reportExplicitAny]

        # Extract response JSON from tool_response[0].text (OLD WORKING LOGIC)
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
            result = response_json.get("result", [])
            query_result_json = json.dumps(result)
            status, error = validate_query_result(query_result_json, operation, status, error)

        return status, error

    except Exception as e:
        return "FAIL", f"Failed to parse MCP response: {e}"


def action_get_next(port: int) -> None:
    """Get next operation that needs execution."""
    file_path = get_plan_file_path(port)
    test_plan = load_test_plan(file_path)

    operation, _ = find_next_operation(test_plan)

    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    if operation is None:
        # All operations complete
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** FINISHED **\n")
        except Exception:
            # Silently ignore debug log write failures
            pass
        print(json.dumps({"status": "finished"}, indent=2))
        return

    # Log that we're providing this operation to the subagent
    operation_id = cast(int, operation.get("operation_id"))
    try:
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} provided to subagent\n")
            f.flush()  # Explicitly flush to ensure write happens
    except Exception:
        # Silently ignore debug log write failures
        pass

    # Return operation with execution parameters only
    execution_params = get_execution_params(operation)
    response = {"status": "next_operation", "operation": execution_params}
    print(json.dumps(response, indent=2))


def action_update(
    port: int, tool_name: str, mcp_response_arg: str
) -> None:
    """Update operation status based on MCP response."""
    file_path = get_plan_file_path(port)
    test_plan = load_test_plan(file_path)

    # Find next operation to update
    operation, current_test = find_next_operation(test_plan)

    if operation is None:
        print("No operation to update", flush=True)
        return

    operation_id = cast(int, operation.get("operation_id"))

    # Verify tool matches before updating status
    expected_tool = cast(str, operation.get("tool", ""))
    if expected_tool != tool_name:
        # Tool mismatch - auxiliary operation (e.g., entity_id_substitution query)
        # Don't update status, exit silently
        sys.exit(0)

    # Parse MCP response and get final status/error
    status, error = parse_mcp_response(mcp_response_arg, tool_name, operation)

    # Update operation with final status
    operation["status"] = status

    if status == "SUCCESS":
        # Remove error field on success (don't write null)
        operation.pop("error", None)
    else:  # FAIL
        operation["error"] = error if error else "Unknown error"

    # Increment call_count
    current_call_count: int = cast(int, operation.get("call_count", 0))
    operation["call_count"] = current_call_count + 1

    # Propagate entity ID to dependent operations (query operations only)
    if status == "SUCCESS" and tool_name == "mcp__brp__world_query":
        captured_entity_id = operation.get("entity")

        # Only propagate if we have a real entity ID (not the placeholder)
        if captured_entity_id and captured_entity_id != "USE_QUERY_RESULT":
            # Update all subsequent operations in THIS TEST ONLY
            if current_test is not None:
                operations_in_test = cast(
                    list[dict[str, Any]], current_test.get("operations", [])  # pyright: ignore[reportExplicitAny]
                )
                for op in operations_in_test:
                    # Only update operations that come after this query
                    if op.get("operation_id", 0) > operation_id:
                        # Replace USE_QUERY_RESULT placeholder with actual entity ID
                        if op.get("entity") == "USE_QUERY_RESULT":
                            op["entity"] = captured_entity_id

    # Log failures to debug log (successes are already tracked in plan files)
    if status == "FAIL":
        try:
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            short_tool = shorten_tool_name(tool_name)
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(
                    f"[{timestamp}] port={port} op_id={operation_id} status=FAIL tool={short_tool}\n"
                )
                if error:
                    _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} error={error}\n")
                f.flush()
        except Exception:
            # Silently ignore debug log write failures
            pass

    # Write updated test plan back atomically
    save_test_plan(file_path, test_plan)

    # Output message for hook
    if status == "SUCCESS":
        print(f"✅ Op {operation_id}: SUCCESS", flush=True)
    else:
        print(f"💥 Op {operation_id}: FAIL", flush=True)


def main() -> None:
    """Main entry point."""
    args = parse_args()
    validate_args(args)

    port: int = cast(int, args.port)
    action: str = cast(str, args.action)

    if action == "get-next":
        action_get_next(port)
    elif action == "update":
        tool_name: str = cast(str, args.tool_name)
        mcp_response_arg: str = cast(str, args.mcp_response)
        action_update(port, tool_name, mcp_response_arg)


if __name__ == "__main__":
    main()
