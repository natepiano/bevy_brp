# Plan: MCP Debug Feature Flag

## Overview
Add a feature flag `mcp-debug` to conditionally compile MCP server debugging tools (`brp_set_tracing_level` and `brp_get_trace_log_path`).

## Goals
1. **Local development**: Debug tools automatically enabled when developing in this workspace
2. **Published crate**: Debug tools disabled by default (opt-in only)

## Rationale
These tools are only useful for debugging the MCP server implementation itself, not for end users of the MCP server. By hiding them behind a feature flag:
- Reduces tool clutter for normal users
- Makes it clear these are internal/debugging tools
- Still available when needed via explicit opt-in

## Implementation

### 1. Add Feature Flag to `mcp/Cargo.toml`
- Add `[features]` section with `mcp-debug = []`
- Do NOT add to default features (disabled by default when published)

### 2. Conditionally Compile Debug Tools
Add `#[cfg(feature = "mcp-debug")]` to:
- `BrpGetTraceLogPath` and `BrpSetTracingLevel` enum variants in `mcp/src/tool/tool_name.rs`
- All references in `get_annotations()`, `get_parameters()`, and `create_handler()` match arms
- Imports of `GetTraceLogPath`, `SetTracingLevel`, and their param/result types
- Re-exports in `mcp/src/log_tools/mod.rs`

### 3. Create `.cargo/config.toml` for Local Development
- Add build configuration to enable `mcp-debug` feature by default locally
- This only affects builds within this workspace
- Not included when users install from crates.io

### 4. Update Help Text (Optional)
- Add note to help text files that these tools require `mcp-debug` feature

## Files to Modify
- `mcp/Cargo.toml` - add feature definition
- `mcp/src/tool/tool_name.rs` - add conditional compilation
- `mcp/src/log_tools/mod.rs` - conditionally export types
- `.cargo/config.toml` - create new file for local dev config

## Expected Behavior
- **Local workspace builds**: Debug tools available automatically
- **Published crate default**: Debug tools hidden
- **Published crate with opt-in**: Users can enable with `bevy_brp_mcp = { version = "...", features = ["mcp-debug"] }`
