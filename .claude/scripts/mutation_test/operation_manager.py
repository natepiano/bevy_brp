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
MAX_SUBAGENTS = cast(int, CONFIG["max_subagents"])
BASE_PORT = cast(int, CONFIG["base_port"])


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


def skip_remaining_operations_in_test(
    test: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    reason: str,
) -> None:
    """Mark all non-completed operations in test as FAIL with skip reason."""
    operations = cast(list[dict[str, Any]], test.get("operations", []))  # pyright: ignore[reportExplicitAny]
    for op in operations:
        if op.get("status") != "SUCCESS":
            op["status"] = "FAIL"
            op["error"] = f"Skipped: {reason}"
            # Set high call_count so find_next_operation won't retry
            op["call_count"] = 999


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
            call_count = cast(int, op.get("call_count", 0))
            # Skip operations that have been marked as skipped (call_count=999)
            if status == "FAIL" and call_count == 999:
                continue
            # Return operations that need execution (no status or FAIL)
            # Operations with call_count >= 4 are returned so termination check can handle them
            if status is None or status == "FAIL":
                return op, test

    # No operations found needing execution
    return None, None


def get_execution_params(operation: dict[str, Any]) -> dict[str, Any]:  # pyright: ignore[reportExplicitAny]
    """
    Extract execution parameters from operation, excluding tracking fields.

    Keeps operation_id and call_count for agent circuit breaker logic.
    """
    exclude_fields = {
        "operation_announced",
        "status",
        "error",
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


def parse_mcp_response_with_input(
    mcp_response_arg: str,
    tool_name: str,
    operation: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    port: int,
    operation_id: int,
) -> tuple[str, str | None, dict[str, object]]:
    """
    Parse MCP response and extract final status/error and tool_input.

    Returns:
        Tuple of (final_status, final_error, tool_input)
    """
    try:
        # Read from stdin if '-'
        if mcp_response_arg == "-":
            mcp_data_raw = json.load(sys.stdin)  # pyright: ignore[reportAny]
        else:
            mcp_data_raw = json.loads(mcp_response_arg)  # pyright: ignore[reportAny]

        mcp_data = cast(HookEvent, mcp_data_raw)
        tool_input = mcp_data.get("tool_input", {})

        # Extract response JSON from tool_response[0].text
        response_text = mcp_data["tool_response"][0]["text"]
        response_json_raw = json.loads(response_text)  # pyright: ignore[reportAny]
        response_json = cast(BrpResponse, response_json_raw)

        # Determine initial status from MCP response
        if response_json.get("status") == "success":
            status = "SUCCESS"
            error = None
        else:
            status = "FAIL"
            # Extract error message
            metadata = response_json.get("metadata")
            error = (
                (metadata.get("original_error") if metadata else None)
                or response_json.get("message")
                or "Unknown error"
            )

        # Special handling for query operations: validate entity availability
        if tool_name == "mcp__brp__world_query" and status == "SUCCESS":
            result = response_json.get("result", [])
            query_result_json = json.dumps(result)
            status, error = validate_query_result(query_result_json, operation, status, error)

        return (status, error, tool_input)

    except Exception as e:
        # Subagent mistakenly called --action update for previous operation
        # Hook already handled the update correctly, so this is just informational
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                prior_op_id = operation_id - 1
                _ = f.write(f"[{timestamp}] port={port} op_id={prior_op_id} subagent mistakenly called update\n")
                f.flush()
        except Exception:
            pass
        empty_dict: dict[str, object] = {}
        return ("FAIL", f"Failed to parse MCP response: {e}", empty_dict)


def handle_test_failure(
    port: int,
    file_path: str,
    test_plan: TestPlan,
    current_test: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    operation_id: int,
    reason: str,
) -> None:
    """
    Handle test failure by skipping remaining operations and trying next test.

    Logs the skip, marks remaining operations as failed, saves plan,
    and provides next operation or finishes if no more tests.
    """
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    type_name = cast(str, current_test.get("type_name", "unknown"))

    # Log the skip with type name
    try:
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            _ = f.write(
                f"[{timestamp}] port={port} ** SKIPPING TYPE: {type_name} ** {reason} at op_id={operation_id}\n"
            )
    except Exception:
        pass

    # Skip all remaining operations in this test
    skip_remaining_operations_in_test(current_test, reason)

    # Save updated test plan
    save_test_plan(file_path, test_plan)

    # Try to find next operation from a different test
    next_operation, _ = find_next_operation(test_plan)

    if next_operation is None:
        # No more tests to run - all done
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** FINISHED **\n")
        except Exception:
            pass
        print(json.dumps({"status": "finished"}, indent=2))
        return

    # Provide next operation from next test
    next_op_id = cast(int, next_operation.get("operation_id"))
    try:
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            _ = f.write(f"[{timestamp}] port={port} op_id={next_op_id} provided to subagent\n")
            f.flush()
    except Exception:
        pass

    execution_params = get_execution_params(next_operation)
    response = {"status": "next_operation", "operation": execution_params}
    print(json.dumps(response, indent=2))


def action_get_next(port: int) -> None:
    """Get next operation that needs execution."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    # Defensive check: Has this port already been terminated or finished?
    port_finished = False
    port_timeout_terminated = False
    port_hard_terminated = False
    test_complete = False

    try:
        with open(MUTATION_TEST_LOG, "r", encoding="utf-8") as log_f:
            for line in log_f:
                # Check for test completion
                if "** MUTATION TEST COMPLETE **" in line:
                    test_complete = True

                # Check port-specific termination states
                if f"port={port}" in line:
                    if "** FINISHED **" in line:
                        port_finished = True
                    elif "** TERMINATED (TIMEOUT) **" in line:
                        port_timeout_terminated = True
                    elif "** TERMINATED" in line:  # Other terminations (retry limit, BRP failed, etc.)
                        port_hard_terminated = True
    except Exception:
        pass

    # Block requests if: test complete, port finished, or hard terminated (non-timeout)
    if test_complete or port_finished or port_hard_terminated:
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** REQUEST AFTER TERMINATION ** - Returning finished status\n")
        except Exception:
            pass
        print(json.dumps({"status": "finished"}, indent=2))
        return

    # Resume after timeout - subagent is still alive
    if port_timeout_terminated:
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** RESUMING AFTER TIMEOUT ** - Subagent is still active\n")
        except Exception:
            pass

    file_path = get_plan_file_path(port)
    test_plan = load_test_plan(file_path)

    operation, current_test = find_next_operation(test_plan)

    if operation is None:
        # All operations complete for this subagent
        try:
            # Write FINISHED marker and close file before reading
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** FINISHED **\n")

            # Check if all other subagents are also complete (purely log-based)
            # Parse log to find which ports participated and which finished
            participated_ports: set[int] = set()
            finished_ports: set[int] = set()
            summary_exists = False
            # Track last activity time per port (for timeout detection)
            last_activity: dict[int, datetime] = {}

            try:
                with open(MUTATION_TEST_LOG, "r", encoding="utf-8") as log_f:
                    for line in log_f:
                        # Check if summary already written
                        if "** MUTATION TEST COMPLETE **" in line:
                            summary_exists = True

                        # Extract timestamp and port from any activity line
                        timestamp_match = None
                        port_match = None
                        parts = line.split()
                        for i, part in enumerate(parts):
                            if i == 0 and part.startswith("["):
                                # Extract timestamp [YYYY-MM-DD HH:MM:SS]
                                timestamp_str = " ".join(parts[0:2]).strip("[]")
                                try:
                                    timestamp_match = datetime.strptime(timestamp_str, "%Y-%m-%d %H:%M:%S")
                                except Exception:
                                    pass
                            if part.startswith("port="):
                                try:
                                    port_match = int(part.split("=")[1])
                                except Exception:
                                    pass

                        # Track ports that provided operations (participated in the test)
                        if " provided to subagent" in line and port_match:
                            participated_ports.add(port_match)
                            if timestamp_match:
                                last_activity[port_match] = timestamp_match

                        # Update activity time for status updates (subagent is working)
                        if " status=" in line and port_match and timestamp_match:
                            last_activity[port_match] = timestamp_match

                        # Find ports that finished or hard-terminated (not timeout)
                        # Note: TIMEOUT terminations are not considered done (subagent may resume)
                        if " ** FINISHED **" in line:
                            try:
                                for part in line.split():
                                    if part.startswith("port="):
                                        finished_port = int(part.split("=")[1])
                                        finished_ports.add(finished_port)
                                        break
                            except Exception:
                                pass
                        elif " ** TERMINATED **" in line and " ** TERMINATED (TIMEOUT) **" not in line:
                            # Hard terminations (retry limit, BRP failed, etc.) - not timeout
                            try:
                                for part in line.split():
                                    if part.startswith("port="):
                                        finished_port = int(part.split("=")[1])
                                        finished_ports.add(finished_port)
                                        break
                            except Exception:
                                pass

                        # Resume after timeout - remove from finished list
                        if " ** RESUMING AFTER TIMEOUT **" in line:
                            try:
                                for part in line.split():
                                    if part.startswith("port="):
                                        resumed_port = int(part.split("=")[1])
                                        finished_ports.discard(resumed_port)
                                        # Update activity timestamp to current time
                                        if timestamp_match:
                                            last_activity[resumed_port] = timestamp_match
                                        break
                            except Exception:
                                pass
            except Exception:
                # If we can't read the log, don't write summary
                summary_exists = True

            # Check for timeouts on active ports (participated but not finished)
            active_ports = participated_ports - finished_ports
            timed_out_ports: set[int] = set()
            already_logged_timeout: set[int] = set()
            now = datetime.now()

            # First pass: find which ports already have timeout logs (but not resumed)
            # A port that resumed can timeout again and should be logged
            try:
                with open(MUTATION_TEST_LOG, "r", encoding="utf-8") as log_f:
                    for line in log_f:
                        if " ** TERMINATED (TIMEOUT) **" in line:
                            try:
                                for part in line.split():
                                    if part.startswith("port="):
                                        timeout_port = int(part.split("=")[1])
                                        already_logged_timeout.add(timeout_port)
                                        break
                            except Exception:
                                pass
                        # If port resumed, allow it to be logged again if it times out
                        if " ** RESUMING AFTER TIMEOUT **" in line:
                            try:
                                for part in line.split():
                                    if part.startswith("port="):
                                        resumed_port = int(part.split("=")[1])
                                        already_logged_timeout.discard(resumed_port)
                                        break
                            except Exception:
                                pass
            except Exception:
                pass

            # Second pass: check for new timeouts and log only if not already logged
            for check_port in active_ports:
                if check_port in last_activity:
                    time_since_activity = (now - last_activity[check_port]).total_seconds()
                    if time_since_activity > 60:  # 60 second timeout
                        timed_out_ports.add(check_port)
                        # Only log timeout if we haven't already logged it for this port
                        if check_port not in already_logged_timeout:
                            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                                _ = f.write(f"[{timestamp}] port={check_port} ** TERMINATED (TIMEOUT) ** - No activity for {time_since_activity:.0f} seconds\n")

            # All complete if: summary not written AND all participated ports finished or timed out
            all_ports_done = finished_ports | timed_out_ports
            all_complete = not summary_exists and participated_ports and participated_ports == all_ports_done

            if all_complete:
                # Calculate statistics and duration from log file
                duration_str = ""
                # Track final status per (port, op_id) operation
                operation_status: dict[tuple[int, int], str] = {}

                try:
                    with open(MUTATION_TEST_LOG, "r", encoding="utf-8") as log_f:
                        for line in log_f:
                            if line.startswith("# Started:"):
                                start_time_str = line.split("Started:", 1)[1].strip()
                                start_time = datetime.strptime(start_time_str, "%Y-%m-%d %H:%M:%S")
                                end_time = datetime.now()
                                duration = end_time - start_time
                                total_seconds = int(duration.total_seconds())
                                hours = total_seconds // 3600
                                minutes = (total_seconds % 3600) // 60
                                seconds = total_seconds % 60
                                duration_str = f" (Duration: {hours}h {minutes}m {seconds}s)"

                            # Parse status lines: [timestamp] port=30001 op_id=X status=SUCCESS tool=...
                            if " status=" in line and " port=" in line and " op_id=" in line:
                                try:
                                    parts = line.split()
                                    port_part = None
                                    op_id_part = None
                                    status_part = None

                                    for part in parts:
                                        if part.startswith("port="):
                                            port_part = int(part.split("=")[1])
                                        elif part.startswith("op_id="):
                                            op_id_part = int(part.split("=")[1])
                                        elif part.startswith("status="):
                                            status_part = part.split("=")[1]

                                    if port_part is not None and op_id_part is not None and status_part:
                                        # Update with latest status (handles retries)
                                        operation_status[(port_part, op_id_part)] = status_part
                                except Exception:
                                    # Skip malformed lines
                                    pass
                except Exception:
                    # If we can't read the log, just omit statistics
                    pass

                # Count final status per port
                port_stats: dict[int, dict[str, int]] = {}
                for (port_num, _op_id), final_status in operation_status.items():
                    if port_num not in port_stats:
                        port_stats[port_num] = {"SUCCESS": 0, "FAIL": 0}

                    if final_status == "SUCCESS":
                        port_stats[port_num]["SUCCESS"] += 1
                    elif final_status == "FAIL":
                        port_stats[port_num]["FAIL"] += 1

                # Write summary statistics for each subagent
                total_success = 0
                total_fail = 0
                with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                    if port_stats:
                        _ = f.write(f"[{timestamp}] Subagent Summary:\n")
                        for subagent_port in sorted(port_stats.keys()):
                            success_count = port_stats[subagent_port]["SUCCESS"]
                            fail_count = port_stats[subagent_port]["FAIL"]
                            _ = f.write(f"[{timestamp}]   port={subagent_port}: SUCCESS={success_count}, FAIL={fail_count}\n")
                            total_success += success_count
                            total_fail += fail_count

                    _ = f.write(f"[{timestamp}] ** MUTATION TEST COMPLETE **{duration_str}\n")

                    # Output overall test results
                    if total_fail == 0 and total_success > 0:
                        _ = f.write(f"[{timestamp}] ALL TESTS PASSED\n")
                    elif total_success > 0 or total_fail > 0:
                        _ = f.write(f"[{timestamp}] Tests Passed: {total_success}, Tests Failed: {total_fail}\n")
        except Exception:
            # Silently ignore debug log write failures
            pass
        print(json.dumps({"status": "finished"}, indent=2))
        return

    # Check termination conditions before providing operation to subagent
    # Safety check: operation exists, so current_test must also exist
    if current_test is None:
        print(json.dumps({"status": "finished"}, indent=2))
        return

    operation_id = cast(int, operation.get("operation_id"))
    call_count = cast(int, operation.get("call_count", 0))
    error = cast(str, operation.get("error", ""))

    # Termination check 1: Retry limit exceeded
    if call_count >= 4:
        handle_test_failure(
            port, file_path, test_plan, current_test, operation_id, "Retry limit exceeded"
        )
        return

    # Termination check 2: Hard safety limit (should never reach this if check 1 works)
    if call_count > 10:
        handle_test_failure(
            port,
            file_path,
            test_plan,
            current_test,
            operation_id,
            f"Excessive retries (call_count={call_count})",
        )
        return

    # Termination check 3: BRP connection failed
    if "JSON-RPC error: HTTP request failed" in error:
        handle_test_failure(
            port, file_path, test_plan, current_test, operation_id, "BRP connection failed"
        )
        return

    # Termination check 4: Resource/component not found
    if "not present in the world" in error:
        handle_test_failure(
            port,
            file_path,
            test_plan,
            current_test,
            operation_id,
            "Resource/component not found",
        )
        return

    # Operation is viable - log and provide to subagent
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

    # Parse MCP response once and extract both status/error and tool_input
    status, error, tool_input = parse_mcp_response_with_input(
        mcp_response_arg, tool_name, operation, port, operation_id
    )

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

    # Log operation status to debug log
    try:
        timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        short_tool = shorten_tool_name(tool_name)
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            _ = f.write(
                f"[{timestamp}] port={port} op_id={operation_id} status={status} tool={short_tool}\n"
            )
            if status == "FAIL" and error:
                _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} error={error}\n")
                # Log the actual parameters that were passed to the failing operation
                params_json = json.dumps(tool_input, separators=(",", ":"))
                _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} params={params_json}\n")
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
