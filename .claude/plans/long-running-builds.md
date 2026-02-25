# Plan: Long-Running Builds via MCP Tasks

**Status: NEEDS RESEARCH**

## Problem

The `brp_launch_bevy_app` and `brp_launch_bevy_example` tools block during `cargo build` (via `ensure_built()`). When compilation takes >60s, the MCP client (Claude Code) times out with "timed out awaiting tools/call after 60s". The server doesn't know the client gave up — it's still happily building.

Current workaround: pre-build the workspace before calling launch tools. This works but is fragile and requires caller discipline.

## Proposed Solution: MCP Tasks

The [MCP 2025-11-25 spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/tasks) introduced **Tasks** — an experimental "call-now, fetch-later" primitive. rmcp 0.16.0 already has full server-side support for this.

### How it would work

1. Launch tools declare `execution.taskSupport: "optional"` in `tools/list`
2. When a client sends `tools/call` with a `task: { ttl: N }` field, the server:
   - Accepts the request immediately, returns `CreateTaskResult` with `status: "working"`
   - Spawns the build + launch in a background tokio task
   - The client polls via `tasks/get` and retrieves results via `tasks/result`
3. When a client sends `tools/call` **without** the `task` field, the tool works exactly as it does today (synchronous, blocking)

### Why `"optional"` not `"required"`

Backward compatibility. Clients that don't support Tasks (or where the build is fast/pre-built) still work with the synchronous path. Only clients that opt in get the async behavior.

## Research Needed

### 1. Does Claude Code support MCP Tasks as a client?

- The spec is experimental and Claude Code docs don't mention Tasks
- Need to test: does Claude Code send `task: { ttl: N }` when a tool declares `taskSupport: "optional"`?
- Need to test: does Claude Code handle `tasks/get` polling and `tasks/result` retrieval?
- If Claude Code doesn't support it yet, this entire plan is premature
- **How to test**: Add task capability + mark one tool as optional, reload MCP server, check if Claude Code's `tools/call` requests include the `task` field

### 2. rmcp `ServerHandler` trait — does it handle task dispatch automatically?

- rmcp has `OperationProcessor` and task model types
- Need to determine: when `call_tool` receives a task-augmented request, does the rmcp framework automatically return `CreateTaskResult` and manage the lifecycle, or do we need to implement `tasks/get`, `tasks/result`, `tasks/cancel` handlers manually?
- Check the rmcp `ServerHandler` trait for task-related methods
- Check if there's a `#[task_handler]` macro or similar convenience

### 3. What does the `OperationProcessor` actually do?

- rmcp's `task_manager.rs` has `OperationProcessor` with 300s default TTL
- Is this the intended integration point for task-based tool execution?
- How does it connect to the `ServerHandler` trait?

## Implementation Sketch (contingent on research)

### Phase 1: Server capability declaration

**File:** `mcp/src/mcp_service.rs`

```rust
// In get_info(), change:
ServerCapabilities::builder().enable_tools().build()
// To:
ServerCapabilities::builder()
    .enable_tools()
    .enable_tasks()  // or manual TasksCapability::server_default()
    .build()
```

### Phase 2: Mark launch tools with `taskSupport: "optional"`

Where tools are registered in `tools/list`, add execution config to launch tools only:

```rust
tool.with_execution(
    ToolExecution::new().with_task_support(TaskSupport::Optional)
)
```

This likely needs changes to the `BrpTools` derive macro or the tool registration pipeline since tools are currently generated declaratively.

### Phase 3: Implement task-augmented `call_tool`

In `mcp_service.rs` `call_tool`, detect the `task` field in the request:

- If `task` field present AND tool supports tasks:
  - Spawn build + launch as background work
  - Return `CreateTaskResult` immediately
- If no `task` field:
  - Execute synchronously as today

### Phase 4: Implement task lifecycle handlers

Need `tasks/get`, `tasks/result`, `tasks/cancel` handlers. Options:
- Use rmcp's `OperationProcessor` if it handles this
- Or implement manually with a `HashMap<TaskId, TaskState>` behind an `Arc<Mutex<_>>`

### Phase 5: Progress notifications (nice-to-have)

Send `notifications/tasks/status` during long builds to give the client visibility into build progress.

## Key Files

- `mcp/src/mcp_service.rs` — `ServerHandler` impl, `get_info()`, `call_tool()`
- `mcp/src/app_tools/support/launch_common.rs` — `launch_target()`, `ensure_built()`
- `mcp/src/app_tools/brp_launch_bevy_app.rs` — launch app handler
- `mcp/src/app_tools/brp_launch_bevy_example.rs` — launch example handler

## rmcp types available (v0.16.0)

- `TaskSupport` enum: `Forbidden`, `Optional`, `Required`
- `ToolExecution` struct with `task_support` field
- `TasksCapability`, `TaskRequestsCapability`, `ToolsTaskCapability`
- `CreateTaskResult`, `Task`, task status types
- `OperationProcessor` — task lifecycle manager with 300s default TTL
- Methods: `tasks/get`, `tasks/result`, `tasks/cancel`, `tasks/list`

## Risks

1. **Claude Code may not support Tasks yet** — this would make the entire effort unused until they do
2. **Experimental spec** — Tasks may change in future MCP revisions
3. **Complexity** — adds async state management to what is currently a simple sync tool
4. **The prebuild workaround already works** — this is an optimization, not a fix for a broken feature
