# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- 'brp_extras_set_window_title` tool, allowing agent to change the running apps window title
- `brp_extras_send_keys` tool for simulating keyboard input
- Optional `path` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for disambiguation when multiple apps/examples have the same name
- File-based tracing system with dynamic level control (error, warn, info, debug, trace)
  - `brp_set_tracing_level` tool for runtime diagnostic level management
  - `brp_get_trace_log_path` tool to locate trace log files
  - Implements "do no harm" principle - no trace log files created until explicitly enabled via `brp_set_tracing_level` tool
- `brp_type_guide` tool for type discovery using registry schema introspection
- Optional `port` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for custom BRP port support (requires bevy_brp_extras)
- Configurable timeouts for watch operations (`bevy_get_watch` and `bevy_list_watch`) with `timeout_seconds` parameter
- Trace logging integration for watch operations controlled by tracing level
- Timeout status tracking in `brp_list_active_watches` output
- Optional `verbose` parameter to `brp_list_logs` (default: false) for minimal output
- Tool annotations: All tools now display semantic annotations (read-only vs destructive) with human-readable titles
- Tool call response includes a `call_info` field with the tool name andthe brp method used if it was a brp tool call.
- Tool responses now include a `parameters` field showing the parameters that were passed to the tool
- Some Tool responses now include a `error_info` field showing structured error details

### Changed
- Migrated to rmcp 0.4.0 for improved MCP server functionality
- Added comprehensive output schemas to all tools using automatic schema generation from `ToolCallJsonResponse` struct
- Improved error messages when duplicate app/example names are found across workspaces
- All tools will return a pointer to a file in the local temp directory if the response is too large to return to coding agent. Hard coded using heuristics to fit within claude code limits.
- `bevy_get_watch` parameter: Renamed parameter from `components` to `types` for consistency with other BRP tools
- Substantial tool call response changes. If you have any prompts that depend on the response returned from a tool call, please review the response carefully.
- All BRP tool `port` parameters are now optional with default value 15702

## [0.1.4] - Initial Release

### Added
- Initial release with core BRP tools
- Support for entity and resource operations
- Watch functionality for monitoring changes
- Application and log management tools

[0.2.1]: https://github.com/natepiano/bevy_brp/mcp/compare/v0.1.4...v0.2.1
[0.1.4]: https://github.com/natepiano/bevy_brp/mcp/releases/tag/v0.1.4
