#!/usr/bin/env python3
"""
Mutation test runner: executes every batch of the mutation test through one MCP client.

Builds bevy_brp_mcp from this checkout, starts it over stdio, then for each batch:
1. prepare.py writes the per-port test plans (with --keep-failed after the first batch,
   so one pass never retests its own failures)
2. brp_launch starts one extras_plugin per port, brp_status waits for BRP to answer,
   and each window gets its assignment's title
3. one worker thread per port loops: get-next -> entity-id substitution -> tool call ->
   update, until operation_manager.py reports the port finished
4. the batch's apps are shut down, even when the batch fails
5. process_results.py records the batch's statuses in all_types.json

The loop stops when prepare.py reports nothing left to test, on an error, or when a
batch makes no progress. Failures do not stop it: one pass surfaces every failure.

Each batch's log and plans are copied to .claude/transient/mutation_run_<timestamp>/,
because prepare.py moves them aside by the minute and process_results.py keeps only
the two newest failure logs. The full run summary is written to
.claude/transient/mutation_run_summary.json; a concise one is printed on stdout.

Usage:
  python3 .claude/scripts/mutation_test/run.py [--max-batches N]
"""

import argparse
import json
import shutil
import subprocess
import sys
import threading
import time
from datetime import datetime
from pathlib import Path
from typing import Required, TypedDict, cast

# Add script directory to path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from config import AllTypesData, load_config  # noqa: E402
from mcp_client import (  # noqa: E402
    REPO_ROOT,
    LatencyStats,
    McpClient,
    McpClientError,
    ToolCallOutcome,
)
from operation_manager import HookEvent, action_get_next, action_update  # noqa: E402

APP_NAME = "extras_plugin"
MCP_TOOL_PREFIX = "mcp__brp__"
TRANSIENT_DIR = REPO_ROOT / ".claude/transient"
ALL_TYPES_FILE = TRANSIENT_DIR / "all_types.json"
SUMMARY_FILE = TRANSIENT_DIR / "mutation_run_summary.json"
PREPARE_SCRIPT = script_dir / "prepare.py"
PROCESS_RESULTS_SCRIPT = script_dir / "process_results.py"
# Seconds allowed for brp_launch, which builds the example before starting it
LAUNCH_TIMEOUT = 1800.0
# Seconds allowed for one BRP tool call
CALL_TIMEOUT = 120.0
# Seconds allowed for brp_shutdown and brp_status calls
APP_TOOL_TIMEOUT = 60.0
# Seconds to wait for every launched app to answer BRP
READY_DEADLINE = 300.0
# Seconds between brp_status rounds while waiting for an app
READY_POLL_INTERVAL = 1.0
# Arguments that name the target of an operation and never hold a placeholder
SUBSTITUTION_EXEMPT_ARGUMENTS = ("entity", "port")


class Assignment(TypedDict):
    """One port's share of a batch, from prepare.py."""

    subagent: int
    port: int
    window_description: str
    task_description: str
    test_plan_file: str
    type_descriptions: list[str]


class PrepareOutput(TypedDict):
    """prepare.py's stdout JSON."""

    batch_number: int
    max_subagents: int
    ops_per_subagent: int
    total_types: int
    progress_message: str
    assignments: list[Assignment]


class FailureSummary(TypedDict, total=False):
    """One entry of process_results.py's retry_failures or review_failures."""

    type: str
    status: str
    summary: str
    entity_id: int | None
    failed_at: str
    test_plan_file: str
    failed_operation_id: int | None


class ProcessStats(TypedDict, total=False):
    """process_results.py's stats."""

    types_tested: int
    passed: int
    failed: int
    retry: int
    missing_components: int
    remaining_types: int | None


class ProcessOutput(TypedDict, total=False):
    """process_results.py's stdout JSON."""

    status: str  # SUCCESS, RETRY_ONLY, FAILURES_DETECTED or ERROR
    stats: ProcessStats
    retry_failures: list[FailureSummary]
    review_failures: list[FailureSummary]
    warnings: list[str]
    retry_log_file: str | None
    review_log_file: str | None
    diagnostic_info: list[dict[str, object]]


class PortResult(TypedDict):
    """What one port's worker did."""

    port: int
    operations: int
    succeeded: int
    failed: int
    substitution_queries: int
    seconds: float
    error: str | None


class BatchRecord(TypedDict, total=False):
    """Everything the run summary keeps about one batch."""

    batch_number: Required[int]
    progress_message: Required[str]
    ports: Required[list[int]]
    seconds: Required[dict[str, float]]
    ports_detail: list[PortResult]
    process_status: str
    stats: ProcessStats
    review_failures: list[dict[str, object]]
    retry_failures: list[dict[str, object]]
    warnings: list[str]
    archive_dir: str
    error: str


class Totals(TypedDict):
    """Stats summed over every batch of the run."""

    types_tested: int
    passed: int
    failed: int
    retry: int
    missing_components: int


class RunSummary(TypedDict):
    """The full run summary written to SUMMARY_FILE."""

    status: str
    stop_reason: str
    started: str
    wall_seconds: float
    batches_run: int
    totals: Totals
    all_types_status_counts: dict[str, int]
    review_failures: list[dict[str, object]]
    retry_failures: list[dict[str, object]]
    latency_ms: dict[str, LatencyStats]
    archive_dir: str
    batches: list[BatchRecord]


class ScriptResult(TypedDict):
    """A helper script's exit code and output."""

    returncode: int
    stdout: str
    stderr: str


class RunState:
    """Mutable totals the batch loop accumulates."""

    def __init__(self) -> None:
        """Start with no batches and zero totals."""
        self.batches: list[BatchRecord] = []
        self.totals: Totals = Totals(
            types_tested=0, passed=0, failed=0, retry=0, missing_components=0
        )
        self.review_failures: list[dict[str, object]] = []
        self.retry_failures: list[dict[str, object]] = []


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Run the mutation test through one MCP client")
    _ = parser.add_argument(
        "--max-batches",
        type=int,
        default=None,
        help="Stop after this many batches (default: run until nothing is left)",
    )
    return parser.parse_args()


def log(message: str) -> None:
    """Write a progress line to stderr."""
    print(f"[{datetime.now().strftime('%H:%M:%S')}] {message}", file=sys.stderr, flush=True)


def run_script(script: Path, arguments: list[str]) -> ScriptResult:
    """Run a helper script from the repository root, as its relative paths expect."""
    completed = subprocess.run(
        [sys.executable, str(script), *arguments],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return ScriptResult(
        returncode=completed.returncode, stdout=completed.stdout, stderr=completed.stderr
    )


def parse_trailing_json(stdout: str) -> object:
    """Parse the JSON document a script prints last; earlier lines may be warnings."""
    lines = stdout.splitlines()
    start = next((index for index, line in enumerate(lines) if line == "{"), None)
    if start is None:
        raise ValueError("no JSON object in output")
    return cast(object, json.loads("\n".join(lines[start:])))


def untested_count() -> int:
    """Types still untested and packed into a batch."""
    with open(ALL_TYPES_FILE, encoding="utf-8") as f:
        data = cast(AllTypesData, json.load(f))
    return sum(
        1
        for type_data in data["type_guide"].values()
        if type_data.get("test_status") == "untested"
        and (type_data.get("batch_number") or 0) > 0
    )


def status_counts() -> dict[str, int]:
    """Count all_types.json entries by test_status."""
    with open(ALL_TYPES_FILE, encoding="utf-8") as f:
        data = cast(AllTypesData, json.load(f))
    counts: dict[str, int] = {}
    for type_data in data["type_guide"].values():
        status = type_data.get("test_status") or "none"
        counts[status] = counts.get(status, 0) + 1
    return dict(sorted(counts.items()))


def substitute_placeholder(value: object, placeholder: int, entity_id: int) -> object:
    """Replace the placeholder entity id, as a number or as a string, in keys and values."""
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return entity_id if value == placeholder else value
    if isinstance(value, str):
        return str(entity_id) if value == str(placeholder) else value
    if isinstance(value, list):
        return [
            substitute_placeholder(item, placeholder, entity_id)
            for item in cast(list[object], value)
        ]
    if isinstance(value, dict):
        return {
            (str(entity_id) if key == str(placeholder) else key): substitute_placeholder(
                item, placeholder, entity_id
            )
            for key, item in cast(dict[str, object], value).items()
        }
    return value


def query_substitute_entity(
    client: McpClient, port: int, exclude: object
) -> tuple[int | None, str | None]:
    """
    Pick an existing entity to stand in for the placeholder.

    Queries every entity and returns the first one that is not the operation's own
    `entity`, or None with the reason no entity could be picked.
    """
    outcome = client.call_tool(
        "world_query", {"data": {}, "filter": {}, "port": port}, CALL_TIMEOUT
    )
    if outcome.is_error:
        return None, f"entity query failed: {outcome.response().get('message', outcome.text)}"
    result = outcome.result()
    if not isinstance(result, list):
        return None, "entity query result is not a list"
    for entry in cast(list[object], result):
        if not isinstance(entry, dict):
            continue
        entity = cast(dict[str, object], entry).get("entity")
        if isinstance(entity, int) and entity != exclude:
            return entity, None
    return None, "entity query found no entity to substitute"


def failure_event(tool: str, arguments: dict[str, object], message: str) -> HookEvent:
    """A failure event for a call that produced no tool response."""
    return HookEvent(
        hook_event_name="PostToolUseFailure",
        tool_name=tool,
        tool_input=arguments,
        error=json.dumps({"status": "error", "message": message}),
    )


def outcome_event(tool: str, arguments: dict[str, object], outcome: ToolCallOutcome) -> HookEvent:
    """The event the hook would have received for this call."""
    if outcome.is_error:
        return HookEvent(
            hook_event_name="PostToolUseFailure",
            tool_name=tool,
            tool_input=arguments,
            error=outcome.text,
        )
    return HookEvent(
        hook_event_name="PostToolUse",
        tool_name=tool,
        tool_input=arguments,
        tool_response=outcome.text,
    )


def execute_operation(
    client: McpClient, operation: dict[str, object]
) -> tuple[HookEvent, bool]:
    """
    Make the operation's tool call and describe its result as a hook event.

    Returns the event and whether an entity-id substitution query ran first.
    """
    tool = str(operation.get("tool", ""))
    tool_name = tool.removeprefix(MCP_TOOL_PREFIX)
    accepted = client.argument_names(tool_name)
    arguments = {key: value for key, value in operation.items() if key in accepted}

    queried = False
    placeholder = operation.get("entity_id_substitution")
    if isinstance(placeholder, int):
        queried = True
        port = cast(int, operation.get("port"))
        entity_id, reason = query_substitute_entity(client, port, operation.get("entity"))
        if entity_id is None:
            return failure_event(tool, arguments, reason or "entity substitution failed"), queried
        arguments = {
            key: value
            if key in SUBSTITUTION_EXEMPT_ARGUMENTS
            else substitute_placeholder(value, placeholder, entity_id)
            for key, value in arguments.items()
        }

    try:
        outcome = client.call_tool(tool_name, arguments, CALL_TIMEOUT)
    except TimeoutError as e:
        return failure_event(tool, arguments, str(e)), queried
    return outcome_event(tool, arguments, outcome), queried


def run_port(client: McpClient, port: int, manager_lock: threading.Lock) -> PortResult:
    """Execute a port's operations until operation_manager.py reports it finished."""
    started = time.perf_counter()
    result = PortResult(
        port=port,
        operations=0,
        succeeded=0,
        failed=0,
        substitution_queries=0,
        seconds=0.0,
        error=None,
    )
    try:
        while True:
            with manager_lock:
                response = action_get_next(port)
            if response["status"] != "next_operation":
                break
            operation = response.get("operation", {})
            event, queried = execute_operation(client, operation)
            if queried:
                result["substitution_queries"] += 1
            with manager_lock:
                message = action_update(port, event)
            result["operations"] += 1
            if message.endswith("SUCCESS"):
                result["succeeded"] += 1
            else:
                result["failed"] += 1
    except Exception as e:
        result["error"] = f"{type(e).__name__}: {e}"
    result["seconds"] = round(time.perf_counter() - started, 2)
    return result


def ports_are_consecutive(ports: list[int]) -> bool:
    """Whether the sorted ports form one run a single multi-instance launch covers."""
    return ports == list(range(ports[0], ports[0] + len(ports)))


def shutdown_apps(client: McpClient, ports: list[int]) -> None:
    """Shut down the app on each port; an app that is not running is not an error."""
    for port in ports:
        try:
            _ = client.call_tool(
                "brp_shutdown", {"app_name": APP_NAME, "port": port}, APP_TOOL_TIMEOUT
            )
        except (TimeoutError, McpClientError) as e:
            log(f"shutdown on port {port} failed: {e}")


def launch_apps(client: McpClient, ports: list[int]) -> None:
    """Launch one app per port, after shutting down any already there, and wait for BRP."""
    shutdown_apps(client, ports)

    launches = (
        [(ports[0], len(ports))] if ports_are_consecutive(ports) else [(port, 1) for port in ports]
    )
    for base_port, count in launches:
        outcome = client.call_tool(
            "brp_launch",
            {"target_name": APP_NAME, "port": base_port, "instance_count": count},
            LAUNCH_TIMEOUT,
        )
        if outcome.is_error:
            raise RuntimeError(
                f"brp_launch on port {base_port} failed: {outcome.response().get('message')}"
            )

    deadline = time.monotonic() + READY_DEADLINE
    waiting = set(ports)
    while waiting:
        for port in sorted(waiting):
            outcome = client.call_tool(
                "brp_status", {"app_name": APP_NAME, "port": port}, APP_TOOL_TIMEOUT
            )
            if not outcome.is_error:
                waiting.discard(port)
        if not waiting:
            break
        if time.monotonic() > deadline:
            raise RuntimeError(f"apps on ports {sorted(waiting)} never answered BRP")
        time.sleep(READY_POLL_INTERVAL)


def set_window_titles(client: McpClient, assignments: list[Assignment]) -> None:
    """Title each window with its assignment, so a person can tell the apps apart."""
    for assignment in assignments:
        outcome = client.call_tool(
            "brp_extras_set_window_title",
            {"port": assignment["port"], "title": assignment["window_description"]},
            APP_TOOL_TIMEOUT,
        )
        if outcome.is_error:
            log(f"window title on port {assignment['port']} not set: {outcome.text}")


def run_workers(client: McpClient, ports: list[int]) -> list[PortResult]:
    """Run one worker thread per port and collect their results."""
    manager_lock = threading.Lock()
    results: dict[int, PortResult] = {}

    def work(port: int) -> None:
        results[port] = run_port(client, port, manager_lock)

    threads = [
        threading.Thread(target=work, args=(port,), name=f"port-{port}") for port in ports
    ]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    return [results[port] for port in ports]


def read_failure_log(path: str | None) -> list[dict[str, object]]:
    """Read a process_results.py failure log before a later batch deletes it."""
    if not path:
        return []
    with open(REPO_ROOT / path, encoding="utf-8") as f:
        return cast(list[dict[str, object]], json.load(f))


def archive_batch(archive_root: Path, batch_number: int, ports: list[int]) -> Path:
    """Copy the batch's log and test plans before the next prepare.py moves them."""
    config = load_config()
    batch_dir = archive_root / f"batch_{batch_number}"
    batch_dir.mkdir(parents=True, exist_ok=True)
    sources = [Path(config["mutation_test_log"])] + [
        Path(config["test_plan_file_pattern"].format(port=port)) for port in ports
    ]
    for source in sources:
        if source.exists():
            _ = shutil.copy2(source, batch_dir / source.name)
    return batch_dir


def attach_archive_paths(
    failures: list[dict[str, object]], batch_dir: Path
) -> list[dict[str, object]]:
    """Point each failure at the archived copies of its test plan and the batch log."""
    config = load_config()
    log_name = Path(config["mutation_test_log"]).name
    for failure in failures:
        plan = failure.get("test_plan_file")
        if isinstance(plan, str) and plan:
            failure["archived_test_plan_file"] = str(
                (batch_dir / Path(plan).name).relative_to(REPO_ROOT)
            )
        failure["archived_log_file"] = str((batch_dir / log_name).relative_to(REPO_ROOT))
    return failures


def run_batch(
    client: McpClient,
    prepare: PrepareOutput,
    archive_root: Path,
    state: RunState,
) -> BatchRecord:
    """Launch, test, shut down and process one prepared batch."""
    assignments = prepare["assignments"]
    ports = sorted(assignment["port"] for assignment in assignments)
    batch_number = prepare["batch_number"]
    record = BatchRecord(
        batch_number=batch_number,
        progress_message=prepare["progress_message"],
        ports=ports,
        seconds={},
    )
    log(prepare["progress_message"])

    try:
        phase_started = time.perf_counter()
        launch_apps(client, ports)
        set_window_titles(client, assignments)
        record["seconds"]["launch"] = round(time.perf_counter() - phase_started, 2)

        phase_started = time.perf_counter()
        record["ports_detail"] = run_workers(client, ports)
        record["seconds"]["operations"] = round(time.perf_counter() - phase_started, 2)
    finally:
        phase_started = time.perf_counter()
        shutdown_apps(client, ports)
        record["seconds"]["shutdown"] = round(time.perf_counter() - phase_started, 2)

    worker_errors = [
        f"port {detail['port']}: {detail['error']}"
        for detail in record.get("ports_detail", [])
        if detail["error"]
    ]

    phase_started = time.perf_counter()
    processed = run_script(PROCESS_RESULTS_SCRIPT, [])
    record["seconds"]["process_results"] = round(time.perf_counter() - phase_started, 2)
    batch_dir = archive_batch(archive_root, batch_number, ports)
    record["archive_dir"] = str(batch_dir.relative_to(REPO_ROOT))

    if processed["returncode"] != 0:
        record["process_status"] = "ERROR"
        record["error"] = f"process_results.py failed: {processed['stderr'].strip()}"
        return record
    try:
        output = cast(ProcessOutput, parse_trailing_json(processed["stdout"]))
    except ValueError as e:
        record["process_status"] = "ERROR"
        record["error"] = f"process_results.py output unreadable: {e}"
        return record

    record["process_status"] = output.get("status", "ERROR")
    record["stats"] = output.get("stats", {})
    record["warnings"] = output.get("warnings", [])
    record["review_failures"] = attach_archive_paths(
        read_failure_log(output.get("review_log_file")), batch_dir
    )
    record["retry_failures"] = attach_archive_paths(
        read_failure_log(output.get("retry_log_file")), batch_dir
    )
    if worker_errors:
        record["error"] = "; ".join(worker_errors)

    stats = record["stats"]
    for key in ("types_tested", "passed", "failed", "retry", "missing_components"):
        state.totals[key] += cast(int, stats.get(key, 0) or 0)
    state.review_failures.extend(record["review_failures"])
    state.retry_failures.extend(record["retry_failures"])
    return record


def run_batches(
    client: McpClient, max_batches: int | None, archive_root: Path, state: RunState
) -> str:
    """Run batches until one of the stop conditions holds; return the stop reason."""
    while max_batches is None or len(state.batches) < max_batches:
        arguments = ["--keep-failed"] if state.batches else []
        prepared = run_script(PREPARE_SCRIPT, arguments)
        if prepared["returncode"] != 0:
            return f"error: prepare.py failed: {prepared['stderr'].strip()[-2000:]}"
        if not prepared["stdout"].strip():
            return "complete"
        try:
            prepare = cast(PrepareOutput, parse_trailing_json(prepared["stdout"]))
        except ValueError as e:
            return f"error: prepare.py output unreadable: {e}"

        untested_before = untested_count()
        record = run_batch(client, prepare, archive_root, state)
        state.batches.append(record)

        if "error" in record:
            return f"error: batch {record['batch_number']}: {record['error']}"
        if record.get("process_status") == "ERROR":
            return f"error: batch {record['batch_number']} processing returned ERROR"
        if untested_count() >= untested_before:
            return f"no progress in batch {record['batch_number']}"
        log(
            f"batch {record['batch_number']}: {record.get('process_status')} "
            + f"{json.dumps(record.get('stats', {}))}"
        )
    return "max batches reached"


def final_status(stop_reason: str, state: RunState) -> str:
    """Overall run status from the stop reason and totals."""
    if stop_reason.startswith("error"):
        return "ERROR"
    if stop_reason.startswith("no progress") or state.totals["retry"] > 0:
        # A scripted run executes every operation, so a retry means a runner defect
        return "RUNNER_PROBLEM"
    if state.totals["failed"] > 0 or state.totals["missing_components"] > 0:
        return "FAILURES_DETECTED"
    if stop_reason != "complete":
        return "INCOMPLETE"
    return "SUCCESS"


def concise_summary(summary: RunSummary) -> dict[str, object]:
    """The part of the run summary worth printing."""
    return {
        "status": summary["status"],
        "stop_reason": summary["stop_reason"],
        "wall_seconds": summary["wall_seconds"],
        "batches_run": summary["batches_run"],
        "totals": summary["totals"],
        "all_types_status_counts": summary["all_types_status_counts"],
        "review_failures": [
            {
                "type": failure.get("type"),
                "error": cast(dict[str, object], failure.get("failure_details") or {}).get(
                    "error_message"
                ),
                "failed_operation_id": failure.get("failed_operation_id"),
                "archived_test_plan_file": failure.get("archived_test_plan_file"),
            }
            for failure in summary["review_failures"]
        ],
        "retry_failure_types": [failure.get("type") for failure in summary["retry_failures"]],
        "latency_ms": {
            name: {"n": stats["count"], "median": stats["median_ms"], "p90": stats["p90_ms"]}
            for name, stats in summary["latency_ms"].items()
        },
        "summary_file": str(SUMMARY_FILE.relative_to(REPO_ROOT)),
    }


def main() -> None:
    """Build the server, run the batches, and write the run summary."""
    args = parse_args()
    max_batches = cast(int | None, args.max_batches)
    started_at = datetime.now()
    started = time.perf_counter()
    archive_root = TRANSIENT_DIR / f"mutation_run_{started_at.strftime('%Y%m%d_%H%M%S')}"
    state = RunState()

    log("building and starting bevy_brp_mcp")
    try:
        client = McpClient.build_and_start()
    except (McpClientError, TimeoutError) as e:
        print(json.dumps({"status": "ERROR", "stop_reason": f"error: {e}"}, indent=2))
        sys.exit(1)

    with client:
        try:
            stop_reason = run_batches(client, max_batches, archive_root, state)
        except (McpClientError, TimeoutError, RuntimeError, OSError) as e:
            stop_reason = f"error: {type(e).__name__}: {e}"
        latency = client.latency_stats()

    summary = RunSummary(
        status=final_status(stop_reason, state),
        stop_reason=stop_reason,
        started=started_at.strftime("%Y-%m-%d %H:%M:%S"),
        wall_seconds=round(time.perf_counter() - started, 1),
        batches_run=len(state.batches),
        totals=state.totals,
        all_types_status_counts=status_counts(),
        review_failures=state.review_failures,
        retry_failures=state.retry_failures,
        latency_ms=latency,
        archive_dir=str(archive_root.relative_to(REPO_ROOT)),
        batches=state.batches,
    )
    with open(SUMMARY_FILE, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    print(json.dumps(concise_summary(summary), indent=2))
    if summary["status"] == "ERROR":
        sys.exit(1)


if __name__ == "__main__":
    main()
