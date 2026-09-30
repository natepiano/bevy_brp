#!/usr/bin/env python3
"""
Operation manager for mutation testing.

The mutation test runner (run.py) calls it once per operation, from one worker thread
per port, holding one lock around every call:
1. `action_get_next(port)` hands out the next operation of the port's test plan
2. `action_update(port, event)` records the result of the call the runner made

Both read and write the port's test plan file and append to the mutation test log.
The log lines double as the protocol the timeout and summary logic parses, so their
wording (" provided to subagent", " status=", "** FINISHED **", ...) stays fixed.
"""

import json
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Required, TypedDict, cast

# Add script directory to path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from config import PROVISION_LIMIT_REASON, MutationTestConfig, load_config as load_mutation_config

# Load configuration at module level
CONFIG = load_mutation_config()
MUTATION_TEST_LOG = CONFIG["mutation_test_log"]
MAX_SUBAGENTS = CONFIG["max_subagents"]
BASE_PORT = CONFIG["base_port"]

# The body of a JSON string: characters other than quote and backslash, or an escape pair
JSON_STRING_BODY = r'((?:[^"\\]|\\.)*)'
# Where the MCP server puts the error text in a failed response, most specific first:
# `error_info.original_error` holds the BRP message, `message` the MCP summary
ERROR_MESSAGE_PATTERNS = (
    re.compile(r'"original_error"\s*:\s*"' + JSON_STRING_BODY + '"'),
    re.compile(r'"message"\s*:\s*"' + JSON_STRING_BODY + '"'),
)
# Characters of an unparseable error kept when no message field is found
RAW_ERROR_PREFIX_LENGTH = 500


class QueryResultEntry(TypedDict):
    """Type for a single query result entry."""

    entity: int


class HookEvent(TypedDict, total=False):
    """
    The result of one operation's tool call, in the shape of a Claude Code tool hook event.

    run.py builds it from the MCP response: `tool_response` or `error` holds the tool's JSON
    response text (`content[0].text`), `tool_input` the arguments it sent.
    """

    hook_event_name: str  # PostToolUse, or PostToolUseFailure for a call flagged isError
    tool_response: str  # PostToolUse: JSON-serialized BrpResponse
    error: str  # PostToolUseFailure: JSON-serialized BrpResponse of the failed call
    tool_name: str  # The operation's `tool`, e.g. mcp__brp__world_spawn_entity
    tool_input: dict[str, object]


class GetNextResponse(TypedDict, total=False):
    """What `action_get_next` hands the runner."""

    status: Required[str]  # "next_operation" or "finished"
    operation: dict[str, object]  # Present with "next_operation"


class OperationManagerError(RuntimeError):
    """A test plan file could not be read or written."""


class BrpResponseErrorInfo(TypedDict, total=False):
    """Type for BRP error response error_info field."""

    original_error: str


class BrpResponse(TypedDict, total=False):
    """Type for BRP response structure."""

    status: str
    message: str
    error_info: BrpResponseErrorInfo
    result: list[QueryResultEntry]


class TestPlan(TypedDict):
    """Type for test plan file structure."""

    batch_number: int
    subagent_index: int
    port: int
    test_plan_file: str
    tests: list[dict[str, Any]]  # pyright: ignore[reportExplicitAny]


def get_plan_file_path(port: int, config: MutationTestConfig) -> str:
    """Get test plan file path for given port."""
    return config["test_plan_file_pattern"].format(port=port)


def load_test_plan(file_path: str) -> TestPlan:
    """Load test plan from file."""
    try:
        with open(file_path, encoding="utf-8") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            return cast(TestPlan, test_plan_raw)
    except FileNotFoundError as e:
        raise OperationManagerError(f"Test plan file not found: {file_path}") from e
    except json.JSONDecodeError as e:
        raise OperationManagerError(f"Invalid JSON in test plan file {file_path}: {e}") from e


def save_test_plan(file_path: str, test_plan: TestPlan) -> None:
    """Save test plan to file."""
    try:
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(test_plan, f, indent=2)
    except IOError as e:
        raise OperationManagerError(f"Failed to write test plan file {file_path}: {e}") from e


def skip_remaining_operations_in_test(
    test: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    reason: str,
) -> None:
    """
    Mark all non-completed operations in test as FAIL with skip reason.

    An operation that already failed keeps its recorded error after the reason, so the
    error that exhausted the retries still reaches the results.
    """
    operations = cast(list[dict[str, Any]], test.get("operations", []))  # pyright: ignore[reportExplicitAny]
    for op in operations:
        if op.get("status") != "SUCCESS":
            last_error = cast(str | None, op.get("error"))
            op["status"] = "FAIL"
            op["error"] = (
                f"Skipped: {reason}; last error: {last_error}"
                if last_error
                else f"Skipped: {reason}"
            )
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

    Keeps operation_id, call_count, and times_provided for circuit breaker logic.
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


def extract_error_message(error_text: str) -> str:
    """
    Extract the error message from a PostToolUseFailure `error` that is not valid JSON.

    A truncated response (Claude Code cut long `error` strings mid-document when a hook
    delivered them) makes `json.loads` reject the text. Take the `original_error` value,
    else the `message` value, else a bounded prefix of the raw text.
    """
    for pattern in ERROR_MESSAGE_PATTERNS:
        match = pattern.search(error_text)
        if match and match.group(1):
            escaped = match.group(1)
            try:
                # strict=False accepts the raw newlines a truncation marker inserts
                return cast(str, json.loads(f'"{escaped}"', strict=False))
            except json.JSONDecodeError:
                return escaped

    if not error_text:
        return "Unknown error"
    if len(error_text) <= RAW_ERROR_PREFIX_LENGTH:
        return error_text
    return f"{error_text[:RAW_ERROR_PREFIX_LENGTH]}... [truncated]"


def parse_mcp_response_with_input(
    mcp_data: HookEvent,
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
    tool_input = mcp_data.get("tool_input", {})
    try:
        # A succeeded call carries the MCP response in `tool_response`. A call the MCP server
        # flagged `isError` fires PostToolUseFailure instead, which carries it in `error`.
        if mcp_data.get("hook_event_name") == "PostToolUseFailure":
            error_text = mcp_data.get("error", "")
            try:
                response_json = cast(BrpResponse, json.loads(error_text))
            except json.JSONDecodeError:
                return ("FAIL", extract_error_message(error_text), tool_input)
        else:
            response_text = mcp_data.get("tool_response", "")
            response_json = cast(BrpResponse, json.loads(response_text))

        # Determine initial status from MCP response
        if response_json.get("status") == "success":
            status = "SUCCESS"
            error = None
        else:
            status = "FAIL"
            # Extract error message
            error_info = response_json.get("error_info")
            error = (
                (error_info.get("original_error") if error_info else None)
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
        # The response text is not a JSON tool response; record the call as failed
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} unparseable MCP response: {e}\n")
                f.flush()
        except Exception:
            pass
        return ("FAIL", f"Failed to parse MCP response: {e}", tool_input)


def handle_test_failure(
    port: int,
    file_path: str,
    test_plan: TestPlan,
    current_test: dict[str, Any],  # pyright: ignore[reportExplicitAny]
    operation_id: int,
    reason: str,
) -> GetNextResponse:
    """
    Handle test failure by skipping remaining operations and trying next test.

    Logs the skip, marks remaining operations as failed, saves plan,
    and returns the next operation, or "finished" if no more tests.
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
        return {"status": "finished"}

    # Provide next operation from next test
    next_op_id = cast(int, next_operation.get("operation_id"))
    try:
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            _ = f.write(f"[{timestamp}] port={port} op_id={next_op_id} provided to subagent\n")
            f.flush()
    except Exception:
        pass

    return {"status": "next_operation", "operation": get_execution_params(next_operation)}


def check_and_log_timeouts(
    participated_ports: set[int],
    finished_ports: set[int],
    last_activity: dict[int, datetime],
    timestamp: str,
    log_file: str,
    timeout_threshold: int = 90,
) -> set[int]:
    """
    Check for timed out ports and log newly detected timeouts.

    Returns:
        Set of ports that have timed out (including previously logged ones)
    """
    active_ports = participated_ports - finished_ports
    timed_out_ports: set[int] = set()
    already_logged_timeout: set[int] = set()
    now = datetime.now()

    # First pass: find which ports already have timeout logs (but not resumed)
    try:
        with open(log_file, "r", encoding="utf-8") as log_f:
            for line in log_f:
                if "** TERMINATED (TIMEOUT) **" in line:
                    try:
                        for part in line.split():
                            if part.startswith("port="):
                                timeout_port = int(part.split("=")[1])
                                already_logged_timeout.add(timeout_port)
                                break
                    except Exception:
                        pass
                # If port resumed, allow it to be logged again if it times out
                if "** RESUMING AFTER TIMEOUT **" in line:
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
            if time_since_activity > timeout_threshold:
                timed_out_ports.add(check_port)
                # Only log timeout if we haven't already logged it for this port
                if check_port not in already_logged_timeout:
                    try:
                        with open(log_file, "a", encoding="utf-8") as f:
                            _ = f.write(
                                f"[{timestamp}] port={check_port} ** TERMINATED (TIMEOUT) ** - No activity for {time_since_activity:.0f} seconds\n"
                            )
                    except Exception:
                        pass

    return timed_out_ports


def action_get_next(port: int) -> GetNextResponse:
    """Get next operation that needs execution."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    # Defensive check: Has this port already been terminated or finished?
    port_finished = False
    port_timeout_terminated = False
    port_hard_terminated = False
    test_complete = False
    start_time = None
    last_elapsed_log_time = None
    participated_ports: set[int] = set()
    finished_ports: set[int] = set()
    last_activity: dict[int, datetime] = {}

    try:
        with open(MUTATION_TEST_LOG, "r", encoding="utf-8") as log_f:
            for line in log_f:
                # Check for test completion
                if "** MUTATION TEST COMPLETE **" in line:
                    test_complete = True

                # Track start time for elapsed duration
                if line.startswith("# Started:"):
                    start_time_str = line.split("Started:", 1)[1].strip()
                    try:
                        start_time = datetime.strptime(start_time_str, "%Y-%m-%d %H:%M:%S")
                    except Exception:
                        pass

                # Track last elapsed duration log
                if "** ELAPSED:" in line:
                    try:
                        timestamp_str = " ".join(line.split()[0:2]).strip("[]")
                        last_elapsed_log_time = datetime.strptime(timestamp_str, "%Y-%m-%d %H:%M:%S")
                    except Exception:
                        pass

                # Extract timestamp and port from activity lines for timeout tracking
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

                # Track ports that provided operations (participated) and their activity time
                if " provided to subagent" in line and port_match:
                    participated_ports.add(port_match)
                    if timestamp_match:
                        last_activity[port_match] = timestamp_match

                # Update activity time for status updates (subagent is working)
                if " status=" in line and port_match and timestamp_match:
                    last_activity[port_match] = timestamp_match

                # Track ports that finished or hard-terminated
                if " ** FINISHED **" in line or (" ** TERMINATED **" in line and " ** TERMINATED (TIMEOUT) **" not in line):
                    try:
                        for part in line.split():
                            if part.startswith("port="):
                                finished_ports.add(int(part.split("=")[1]))
                                break
                    except Exception:
                        pass

                # Remove ports that resumed after timeout from finished set
                if " ** RESUMING AFTER TIMEOUT **" in line:
                    try:
                        for part in line.split():
                            if part.startswith("port="):
                                finished_ports.discard(int(part.split("=")[1]))
                                break
                    except Exception:
                        pass

                # Check port-specific termination states
                if f"port={port}" in line:
                    if "** FINISHED **" in line:
                        port_finished = True
                    elif "** RESUMING AFTER TIMEOUT **" in line:
                        port_timeout_terminated = False  # Reset timeout flag on resume
                    elif "** TERMINATED (TIMEOUT) **" in line:
                        port_timeout_terminated = True
                    elif "** TERMINATED" in line:  # Other terminations (retry limit, BRP failed, etc.)
                        port_hard_terminated = True
    except Exception:
        pass

    # Log elapsed duration once per minute - designated logger is lowest active port
    if start_time and not test_complete and participated_ports:
        active_ports = participated_ports - finished_ports
        if active_ports:
            designated_logger = min(active_ports)
            if port == designated_logger:
                now = datetime.now()
                should_log_elapsed = False

                if last_elapsed_log_time is None:
                    # Never logged elapsed duration - check if at least 60 seconds have passed
                    if (now - start_time).total_seconds() >= 60:
                        should_log_elapsed = True
                else:
                    # Check if 60 seconds have passed since last elapsed log
                    if (now - last_elapsed_log_time).total_seconds() >= 60:
                        should_log_elapsed = True

                if should_log_elapsed:
                    try:
                        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                            elapsed = now - start_time
                            total_seconds = int(elapsed.total_seconds())
                            hours = total_seconds // 3600
                            minutes = (total_seconds % 3600) // 60
                            seconds = total_seconds % 60
                            _ = f.write(f"[{timestamp}] ** ELAPSED: {hours}h {minutes}m {seconds}s **\n")
                    except Exception:
                        pass

                    # Check for timeouts after logging elapsed duration
                    _ = check_and_log_timeouts(
                        participated_ports,
                        finished_ports,
                        last_activity,
                        timestamp,
                        MUTATION_TEST_LOG,
                    )

    # Block requests if: test complete, port finished, or hard terminated (non-timeout)
    if test_complete or port_finished or port_hard_terminated:
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** REQUEST AFTER TERMINATION ** - Returning finished status\n")
        except Exception:
            pass
        return {"status": "finished"}

    # Resume after timeout - subagent is still alive
    if port_timeout_terminated:
        try:
            with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
                _ = f.write(f"[{timestamp}] port={port} ** RESUMING AFTER TIMEOUT ** - Subagent is still active\n")
        except Exception:
            pass

    file_path = get_plan_file_path(port, CONFIG)
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
            # Re-scan log to get complete picture (reuse outer scope variables)
            participated_ports = set()
            finished_ports = set()
            summary_exists = False
            # Track last activity time per port (for timeout detection)
            last_activity = {}

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
            timed_out_ports = check_and_log_timeouts(
                participated_ports,
                finished_ports,
                last_activity,
                timestamp,
                MUTATION_TEST_LOG,
            )

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
                    if timed_out_ports:
                        timeout_port_list = ", ".join(str(p) for p in sorted(timed_out_ports))
                        _ = f.write(f"[{timestamp}] INCOMPLETE - {len(timed_out_ports)} subagent(s) timed out (ports: {timeout_port_list})\n")
                    elif total_fail == 0 and total_success > 0:
                        _ = f.write(f"[{timestamp}] ALL TESTS PASSED\n")
                    elif total_success > 0 or total_fail > 0:
                        _ = f.write(f"[{timestamp}] Tests Passed: {total_success}, Tests Failed: {total_fail}\n")
        except Exception:
            # Silently ignore debug log write failures
            pass
        return {"status": "finished"}

    # Check termination conditions before providing operation to subagent
    # Safety check: operation exists, so current_test must also exist
    if current_test is None:
        return {"status": "finished"}

    operation_id = cast(int, operation.get("operation_id"))
    call_count = cast(int, operation.get("call_count", 0))
    times_provided = cast(int, operation.get("times_provided", 0))
    error = cast(str, operation.get("error", ""))

    # Termination check 1: Execution retry limit exceeded
    if call_count >= 4:
        return handle_test_failure(
            port, file_path, test_plan, current_test, operation_id, "Execution retry limit exceeded"
        )

    # Termination check 2: Provision retry limit exceeded (handed out without a result)
    if times_provided >= 4:
        return handle_test_failure(
            port, file_path, test_plan, current_test, operation_id, PROVISION_LIMIT_REASON
        )

    # Termination check 3: Hard safety limit (should never reach this if checks 1-2 work)
    if call_count > 10 or times_provided > 10:
        return handle_test_failure(
            port,
            file_path,
            test_plan,
            current_test,
            operation_id,
            f"Excessive retries (call_count={call_count}, times_provided={times_provided})",
        )

    # Termination check 3: BRP connection failed
    if "JSON-RPC error: HTTP request failed" in error:
        return handle_test_failure(
            port, file_path, test_plan, current_test, operation_id, "BRP connection failed"
        )

    # Termination check 4: Resource/component not found
    if "not present in the world" in error:
        return handle_test_failure(
            port,
            file_path,
            test_plan,
            current_test,
            operation_id,
            "Resource/component not found",
        )

    # Operation is viable - increment times_provided and provide to subagent
    operation["times_provided"] = times_provided + 1

    # Save updated test plan with incremented times_provided
    save_test_plan(file_path, test_plan)

    try:
        with open(MUTATION_TEST_LOG, "a", encoding="utf-8") as f:
            attempt_suffix = f" (attempt {operation['times_provided']})" if operation['times_provided'] > 1 else ""
            _ = f.write(f"[{timestamp}] port={port} op_id={operation_id} provided to subagent{attempt_suffix}\n")
            f.flush()  # Explicitly flush to ensure write happens
    except Exception:
        # Silently ignore debug log write failures
        pass

    # Return operation with execution parameters only
    return {"status": "next_operation", "operation": get_execution_params(operation)}


def action_update(port: int, event: HookEvent) -> str:
    """
    Update operation status based on MCP response.

    Returns a one-line message naming the operation and its recorded status.
    """
    tool_name = event.get("tool_name", "")
    file_path = get_plan_file_path(port, CONFIG)
    test_plan = load_test_plan(file_path)

    # Find next operation to update
    operation, current_test = find_next_operation(test_plan)

    if operation is None:
        return "No operation to update"

    operation_id = cast(int, operation.get("operation_id"))

    # Verify tool matches before updating status
    expected_tool = cast(str, operation.get("tool", ""))
    if expected_tool != tool_name:
        # Tool mismatch - not the call of the operation handed out; leave its status alone
        return f"Op {operation_id}: ignored {tool_name} (expected {expected_tool})"

    # Parse MCP response once and extract both status/error and tool_input
    status, error, tool_input = parse_mcp_response_with_input(
        event, tool_name, operation, port, operation_id
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

    return f"Op {operation_id}: {status}"
