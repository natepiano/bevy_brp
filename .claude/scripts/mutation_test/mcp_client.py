#!/usr/bin/env python3
"""
Stdio client for the bevy_brp_mcp server this repository builds.

Builds the server with cargo, starts it, performs the MCP handshake, caches the tool
schemas, and serves tool calls from any number of threads over one connection. A reader
thread routes each JSON-RPC response to the caller waiting on its id.

The mutation test runner (run.py) and the type-guide fetch for /create_mutation_test_json
(fetch_type_guides.py) import it; all three sit in this directory so the imports resolve
for basedpyright without configuration.
"""

import json
import statistics
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from types import TracebackType
from typing import IO, TypedDict, cast

REPO_ROOT = Path(__file__).resolve().parents[3]
SERVER_BINARY = "bevy_brp_mcp"
MCP_PROTOCOL_VERSION = "2025-06-18"
CLIENT_NAME = "bevy_brp_scripts"
# Where the server's stderr goes, for diagnosing a crash
SERVER_STDERR_LOG = REPO_ROOT / ".claude/transient/mcp_client_server_stderr.log"
# Seconds to wait for a handshake or tools/list response
HANDSHAKE_TIMEOUT = 60.0
# Seconds to wait for the server to exit after its stdin closes
SHUTDOWN_TIMEOUT = 10.0
# Fraction of the sorted latencies below the reported p90
P90_FRACTION = 0.9
MILLISECONDS_PER_SECOND = 1000.0


class JsonRpcError(TypedDict, total=False):
    """Error member of a JSON-RPC response."""

    code: int
    message: str
    data: object


class JsonRpcMessage(TypedDict, total=False):
    """A JSON-RPC message read from the server."""

    jsonrpc: str
    id: int | str
    method: str
    params: dict[str, object]
    result: dict[str, object]
    error: JsonRpcError


class ToolContent(TypedDict, total=False):
    """One content block of a tools/call result."""

    type: str
    text: str


class InputSchema(TypedDict, total=False):
    """JSON schema of a tool's arguments."""

    type: str
    properties: dict[str, object]
    required: list[str]


class ToolSchema(TypedDict, total=False):
    """One entry of a tools/list result."""

    name: str
    description: str
    inputSchema: InputSchema


class ToolResponse(TypedDict, total=False):
    """The JSON document every bevy_brp_mcp tool returns as its text content."""

    status: str  # "success" or "error"
    message: str
    call_info: dict[str, object]
    result: object
    error_info: dict[str, object]


class SavedToFileResult(TypedDict):
    """What replaces `result` when the response is too large to return inline."""

    saved_to_file: bool
    filepath: str
    instructions: str
    original_size_tokens: int


class CargoTarget(TypedDict, total=False):
    """The target member of a cargo compiler-artifact message."""

    name: str
    kind: list[str]


class CargoMessage(TypedDict, total=False):
    """One line of `cargo build --message-format json` output."""

    reason: str
    target: CargoTarget
    executable: str | None


class LatencyStats(TypedDict):
    """Round-trip latency of one tool's calls, in milliseconds."""

    count: int
    median_ms: float
    p90_ms: float
    max_ms: float


class McpClientError(RuntimeError):
    """The server could not be built, started, or reached."""


@dataclass(frozen=True)
class ToolCallOutcome:
    """
    Result of one tools/call.

    `text` is the tool's JSON response exactly as an MCP client receives it in
    `content[0].text`. A JSON-RPC-level error carries no content, so its `text` is a
    JSON document with `status` "error" and a `message` describing the error.
    """

    tool_name: str
    text: str
    is_error: bool
    latency_ms: float

    def response(self) -> ToolResponse:
        """Parse `text` as a tool response; text that is not a JSON object becomes an error."""
        try:
            parsed = cast(object, json.loads(self.text))
        except json.JSONDecodeError:
            return ToolResponse(status="error", message=self.text)
        if not isinstance(parsed, dict):
            return ToolResponse(status="error", message=self.text)
        return cast(ToolResponse, cast(object, parsed))

    def result(self) -> object:
        """Return the response's `result`, reading it from disk when the server saved it to a file."""
        return resolve_result(self.response().get("result"))


@dataclass
class _PendingCall:
    """A request waiting for its response."""

    done: threading.Event = field(default_factory=threading.Event)
    message: JsonRpcMessage | None = None
    failure: str | None = None


def saved_result_path(result: object) -> str | None:
    """Return the file the server saved `result` to, or None when it is inline."""
    if not isinstance(result, dict):
        return None
    fields = cast(dict[str, object], result)
    if fields.get("saved_to_file") is not True:
        return None
    return cast(SavedToFileResult, cast(object, fields))["filepath"]


def resolve_result(result: object) -> object:
    """Return `result`, or the JSON the server saved to a file in its place."""
    path = saved_result_path(result)
    if path is None:
        return result
    with open(path, encoding="utf-8") as f:
        return cast(object, json.load(f))


def build_server(repo_root: Path = REPO_ROOT) -> Path:
    """
    Build the MCP server from this checkout and return the executable cargo produced.

    Uses `--workspace` so the build resolves the same features as the one brp_launch runs
    for the test apps, and the two builds share artifacts. Cargo's progress goes to stderr.
    """
    command = [
        "cargo",
        "build",
        "--workspace",
        "--bin",
        SERVER_BINARY,
        "--message-format",
        "json",
    ]
    process = subprocess.Popen(
        command, cwd=repo_root, stdout=subprocess.PIPE, text=True, encoding="utf-8"
    )
    # typeshed types Popen.stdout as IO[Any]; text=True makes it IO[str]
    stdout = cast(IO[str], process.stdout)
    executable: str | None = None
    for line in stdout:
        try:
            message = cast(CargoMessage, json.loads(line))
        except json.JSONDecodeError:
            continue
        if message.get("reason") != "compiler-artifact":
            continue
        target = message.get("target", {})
        if target.get("name") == SERVER_BINARY and "bin" in target.get("kind", []):
            executable = message.get("executable")
    return_code = process.wait()
    if return_code != 0:
        raise McpClientError(f"`{' '.join(command)}` failed with exit code {return_code}")
    if executable is None:
        raise McpClientError(f"cargo reported no executable for {SERVER_BINARY}")
    return Path(executable)


class McpClient:
    """One stdio connection to bevy_brp_mcp, safe to call from several threads."""

    def __init__(self, executable: Path, cwd: Path = REPO_ROOT) -> None:
        """Start the server and complete the MCP handshake."""
        SERVER_STDERR_LOG.parent.mkdir(parents=True, exist_ok=True)
        self._stderr_file: IO[str] = open(SERVER_STDERR_LOG, "w", encoding="utf-8")
        # brp_launch finds launch targets under the server's working directory
        self._process: subprocess.Popen[str] = subprocess.Popen(
            [str(executable)],
            cwd=cwd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self._stderr_file,
            text=True,
            encoding="utf-8",
            bufsize=1,
        )
        self._write_lock: threading.Lock = threading.Lock()
        self._state_lock: threading.Lock = threading.Lock()
        self._next_id: int = 0
        self._pending: dict[int, _PendingCall] = {}
        self._closed: bool = False
        self._latencies: dict[str, list[float]] = {}
        self._reader: threading.Thread = threading.Thread(
            target=self._read_responses, name="mcp-reader", daemon=True
        )
        self._reader.start()
        self.tools: dict[str, ToolSchema] = {}
        try:
            self._handshake()
        except BaseException:
            self.close()
            raise

    @classmethod
    def build_and_start(cls, repo_root: Path = REPO_ROOT) -> "McpClient":
        """Build the server from this checkout, then start it with the checkout as its cwd."""
        return cls(build_server(repo_root), cwd=repo_root)

    def __enter__(self) -> "McpClient":
        """Return the started client."""
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        """Stop the server."""
        self.close()

    def _handshake(self) -> None:
        """Run initialize and notifications/initialized, then cache the tool schemas."""
        _ = self._request(
            "initialize",
            {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {"name": CLIENT_NAME, "version": "1"},
            },
            HANDSHAKE_TIMEOUT,
        )
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        cursor: str | None = None
        while True:
            params: dict[str, object] = {} if cursor is None else {"cursor": cursor}
            result = self._request("tools/list", params, HANDSHAKE_TIMEOUT)
            for tool in cast(list[ToolSchema], result.get("tools", [])):
                self.tools[tool.get("name", "")] = tool
            next_cursor = result.get("nextCursor")
            if not isinstance(next_cursor, str) or not next_cursor:
                break
            cursor = next_cursor

    def argument_names(self, tool_name: str) -> set[str]:
        """Names of the arguments the tool's input schema declares."""
        schema = self.tools.get(tool_name)
        if schema is None:
            raise McpClientError(f"server has no tool named {tool_name}")
        return set(schema.get("inputSchema", {}).get("properties", {}))

    def call_tool(
        self, name: str, arguments: dict[str, object], timeout: float
    ) -> ToolCallOutcome:
        """
        Call a tool and wait up to `timeout` seconds for its result.

        Raises TimeoutError when no response arrives in time, and McpClientError when the
        server has exited.
        """
        started = time.perf_counter()
        message = self._exchange(
            "tools/call", {"name": name, "arguments": arguments}, timeout
        )
        latency_ms = (time.perf_counter() - started) * MILLISECONDS_PER_SECOND
        with self._state_lock:
            self._latencies.setdefault(name, []).append(latency_ms)

        rpc_error = message.get("error")
        if rpc_error is not None:
            text = json.dumps(
                {
                    "status": "error",
                    "message": (
                        f"MCP tools/call error {rpc_error.get('code')}: "
                        + f"{rpc_error.get('message', 'no message')}"
                    ),
                }
            )
            return ToolCallOutcome(name, text, True, latency_ms)

        result = message.get("result", {})
        content = cast(list[ToolContent], result.get("content", []))
        text = next(
            (block.get("text", "") for block in content if block.get("type") == "text"),
            "",
        )
        if not text:
            structured = result.get("structuredContent")
            text = json.dumps(structured) if structured is not None else ""
        return ToolCallOutcome(name, text, result.get("isError") is True, latency_ms)

    def latency_stats(self) -> dict[str, LatencyStats]:
        """Median, p90 and max round-trip latency per tool."""
        with self._state_lock:
            samples = {name: sorted(values) for name, values in self._latencies.items()}
        stats: dict[str, LatencyStats] = {}
        for name, values in sorted(samples.items()):
            p90_index = max(0, int(len(values) * P90_FRACTION + 0.5) - 1)
            stats[name] = LatencyStats(
                count=len(values),
                median_ms=round(statistics.median(values), 1),
                p90_ms=round(values[p90_index], 1),
                max_ms=round(values[-1], 1),
            )
        return stats

    def close(self) -> None:
        """Close the server's stdin and wait for it to exit, killing it if it does not."""
        with self._state_lock:
            if self._closed:
                return
            self._closed = True
        try:
            if self._process.stdin is not None:
                self._process.stdin.close()
        except OSError:
            pass
        try:
            _ = self._process.wait(timeout=SHUTDOWN_TIMEOUT)
        except subprocess.TimeoutExpired:
            self._process.kill()
            _ = self._process.wait()
        self._reader.join(timeout=SHUTDOWN_TIMEOUT)
        self._stderr_file.close()

    def _request(
        self, method: str, params: dict[str, object], timeout: float
    ) -> dict[str, object]:
        """Send a request and return its result, raising on a JSON-RPC error."""
        message = self._exchange(method, params, timeout)
        rpc_error = message.get("error")
        if rpc_error is not None:
            raise McpClientError(f"{method} failed: {rpc_error.get('message', rpc_error)}")
        return message.get("result", {})

    def _exchange(
        self, method: str, params: dict[str, object], timeout: float
    ) -> JsonRpcMessage:
        """Send a request and wait for the response with the same id."""
        pending = _PendingCall()
        with self._state_lock:
            if self._closed:
                raise McpClientError("client is closed")
            self._next_id += 1
            request_id = self._next_id
            self._pending[request_id] = pending
        try:
            self._send(
                {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
            )
            if not pending.done.wait(timeout):
                raise TimeoutError(f"{method} got no response within {timeout:.0f}s")
        finally:
            with self._state_lock:
                _ = self._pending.pop(request_id, None)
        if pending.message is None:
            raise McpClientError(pending.failure or f"{method} got no response")
        return pending.message

    def _send(self, message: dict[str, object]) -> None:
        """Write one JSON-RPC message as a line on the server's stdin."""
        line = json.dumps(message) + "\n"
        with self._write_lock:
            stdin = self._process.stdin
            if stdin is None:
                raise McpClientError("server stdin is not available")
            try:
                _ = stdin.write(line)
                stdin.flush()
            except (BrokenPipeError, ValueError) as e:
                raise McpClientError(f"server is not accepting input: {e}") from e

    def _read_responses(self) -> None:
        """Route each response line to the caller waiting on its id until stdout closes."""
        # typeshed types Popen.stdout as IO[Any]; text=True makes it IO[str]
        stdout = cast(IO[str] | None, self._process.stdout)
        if stdout is not None:
            for line in stdout:
                try:
                    message = cast(JsonRpcMessage, json.loads(line))
                except json.JSONDecodeError:
                    print(f"mcp_client: ignoring non-JSON line: {line.rstrip()}", file=sys.stderr)
                    continue
                response_id = message.get("id")
                if not isinstance(response_id, int) or "method" in message:
                    # Notification or server request; the server sends no requests we must answer
                    continue
                with self._state_lock:
                    pending = self._pending.get(response_id)
                if pending is not None:
                    pending.message = message
                    pending.done.set()
        # The server exited: fail every call still waiting
        with self._state_lock:
            waiting = list(self._pending.values())
        for pending in waiting:
            pending.failure = f"server exited (code {self._process.poll()})"
            pending.done.set()
