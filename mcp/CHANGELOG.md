# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.18.5] - 2026-02-22

### Fixed
- `list_bevy_apps`, `list_bevy_examples`, and `list_brp_apps` now work in clients that don't provide file roots (e.g., OpenAI Codex). Previously, workspace discovery failed silently when the client returned empty roots; it now falls back to the current working directory.

## [0.18.4] - 2026-02-20

### Added
- **`brp_extras_get_diagnostics` tool**: New MCP tool for querying FPS and frame time diagnostics from a running Bevy app. Returns current, average, and smoothed FPS/frame time values, total frame count, and history buffer metadata via `brp_extras/get_diagnostics`.

## [0.18.3] - 2026-02-17

### Changed
- Version bump to 0.18.3 to maintain workspace version synchronization

## [0.18.2] - 2026-02-17

### Added
- **Mouse input tools**: Nine new MCP tools for mouse control via `bevy_brp_extras`
  - `brp_extras_click_mouse` - Click mouse button (Left, Right, Middle, Back, Forward)
  - `brp_extras_double_click_mouse` - Double click with configurable delay
  - `brp_extras_send_mouse_button` - Press and hold mouse button for duration
  - `brp_extras_move_mouse` - Move cursor (delta or absolute positioning)
  - `brp_extras_drag_mouse` - Drag with smooth interpolated movement
  - `brp_extras_scroll_mouse` - Scroll wheel (line/pixel, horizontal/vertical)
  - `brp_extras_double_tap_gesture` - Trackpad double tap (macOS)
  - `brp_extras_pinch_gesture` - Trackpad pinch-to-zoom (macOS)
  - `brp_extras_rotation_gesture` - Trackpad rotation (macOS)

### Fixed
- `world_trigger_event` now correctly sends struct payloads as JSON objects instead of stringified JSON
- Parameter handling for MCP clients that stringify JSON objects/arrays for `Any`-typed parameters (affects `world_insert_resources`, `world_mutate_resources`, `world_mutate_components`, `registry_schema`)
- Gracefully fall back to current directory when MCP client doesn't support `roots/list` instead of returning a hard error. Thanks kasbah!

## [0.18.1] - 2026-02-10

### Added
- **`brp_extras_type_text` tool**: New MCP tool for typing text sequentially via `brp_extras/type_text` method. Returns number of characters queued and any skipped unmappable characters. Thanks @tobert!

## [0.18.0] - 2026-01-15

### Changed
- Updated dependency to Bevy 0.18.0 stable release

## [0.18.0-rc.1] - 2025-12-21

### Added

- **`world_trigger_event` tool**: Trigger Bevy events remotely via the new `world.trigger_event` BRP method (Bevy 0.18+). Events must derive `Reflect` with `#[reflect(Event)]` to be triggerable. Example: "Trigger the SpawnEnemy event with enemy_type goblin at position 10, 0, 5"

### Changed
- **Upgraded to Bevy 0.18.0-rc.1**: Updated bevy dependency from 0.17.x to 0.18.0-rc.1
- **BREAKING**: `brp_type_guide` and `brp_all_type_guides` responses now return `spawn_example` (for Components) or `resource_example` (for Resources) instead of `spawn_format`. Each example now includes an `agent_guidance` field alongside the `example` value.

## [0.17.2] - 2025-11-20

### Fixed
- Corrected workspace dependency versioning to ensure proper crates.io dependency resolution

## [0.17.1] - 2025-11-20

### Changed
- Improved mutation path descriptions for non-mutable paths to clarify when examples are unavailable

### Fixed
- `world_query` tool description now correctly specifies `filter` parameter as object type, preventing some MCP clients from passing it as a JSON string instead of a structured object
- Fixed "Invalid input" and "did not return structured content" errors in some MCP clients (e.g., Gemini) by:
    - Upgrading `rmcp` to 0.9.0 to support structured content responses via `CallToolResult::structured()` instead of text-wrapped JSON
    - Correcting JSON Schema generation for optional response fields (`metadata`, `result`, etc.) to properly signal optionality and use compatible schema definitions
    - **Note**: This changes the response structure returned to coding agents from `{tool_response: [{text: "..."}]}` to `{tool_response: [{...}]}` (structured data). While coding agents handle both formats transparently, custom code that inspects the raw agent response structure (e.g., hooks, testing infrastructure) may require updates. The actual response content remains identical.
    - Thanks to @tobert for identifying and fixing this issue!

## [0.17.0] - 2025-10-31

### Changed
- Version numbering now tracks Bevy releases for clearer compatibility signaling

### Added
- Support for Bevy 0.17.0 through 0.17.2
- `brp_type_guide` tool for type discovery using registry schema introspection
- Multi-instance launch support via `instance_count` parameter for `brp_launch_bevy_app` and `brp_launch_bevy_example`
  - Launch multiple instances on sequential ports for parallel testing and load simulation
- 'brp_extras_set_window_title` tool, allowing agent to change the running apps window title
- `brp_extras_send_keys` tool for simulating keyboard input
- Optional `path` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for disambiguation when multiple apps/examples have the same name
- Optional Feature for mcp development: File-based tracing system with dynamic level control (error, warn, info, debug, trace)
  - `brp_set_tracing_level` tool for runtime diagnostic level management
  - `brp_get_trace_log_path` tool to locate trace log files
  - Implements "do no harm" principle - no trace log files created until explicitly enabled via `brp_set_tracing_level` tool
- Optional `port` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for custom BRP port support (requires bevy_brp_extras)
- Configurable timeouts for watch operations (`world_get_components_watch` and `world_list_components_watch`) with `timeout_seconds` parameter
- Timeout status tracking in `brp_list_active_watches` output
- Optional `verbose` parameter to `brp_list_logs` (default: false) for minimal output
- Tool annotations: All tools now display semantic annotations (read-only vs destructive) with human-readable titles
- Tool call response includes a `call_info` field with the tool name andthe brp method used if it was a brp tool call.
- Tool responses now include a `parameters` field showing the parameters that were passed to the tool
- Some Tool responses now include a `error_info` field showing structured error details

### Changed
- Added comprehensive output schemas to all tools using automatic schema generation from `ToolCallJsonResponse` struct
- Improved error messages when duplicate app/example names are found across workspaces
- All tools will return a pointer to a file in the local temp directory if the response is too large to return to coding agent. Hard coded using heuristics to fit within claude code limits.
- `world_get_components_watch` parameter: Renamed parameter from `components` to `types` for consistency with other BRP tools
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
