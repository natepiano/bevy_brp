# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - Unreleased

### Added
- `brp_extras_send_keys` tool for simulating keyboard input
- Optional `workspace` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for disambiguation when multiple apps/examples have the same name
- File-based tracing system with dynamic level control (error, warn, info, debug, trace)
- `brp_set_tracing_level` tool for runtime diagnostic level management
- `brp_get_trace_log_path` tool to locate trace log files
- Implements "do no harm" principle - no log files created until explicitly enabled
- `brp_extras_set_debug_mode` tool for bevy_brp_extras plugin debug info - added as a field on the response
- Optional `port` parameter to `brp_launch_bevy_app` and `brp_launch_bevy_example` for custom BRP port support (requires bevy_brp_extras)
- Configurable timeouts for watch operations (`bevy_get_watch` and `bevy_list_watch`) with `timeout_seconds` parameter
- Trace logging integration for watch operations controlled by tracing level
- Timeout status tracking in `brp_list_active_watches` output
- Optional `verbose` parameter to `brp_list_logs` (default: false) for minimal output

### Changed
- Improved error messages when duplicate app/example names are found across workspaces
- `brp_list_logs` returns minimal output by default to reduce token limit errors

## [0.1.4] - Initial Release

### Added
- Initial release with core BRP tools
- Support for entity and resource operations
- Watch functionality for monitoring changes
- Application and log management tools

[0.2.1]: https://github.com/natepiano/bevy_brp/mcp/compare/v0.1.4...v0.2.1
[0.1.4]: https://github.com/natepiano/bevy_brp/mcp/releases/tag/v0.1.4
