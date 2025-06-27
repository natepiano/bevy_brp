# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - Unreleased

### Added
- claude code agentic test suite for parallel execution of bevy_brp_extras and bevy_brp_mcp
- New `brp_extras/send_keys` method for simulating keyboard input
- Debug mode for format discovery via `brp_extras/set_debug_mode` method
  - Provides detailed diagnostic information about type discovery process
  - Helps troubleshoot format discovery issues with complex types
- Environment variable port override support via `BRP_PORT`
  - Allows runtime port configuration without code changes
  - Priority: `BRP_PORT` environment variable > `with_port()` > default port (15702)
  - Enables unique port assignment for testing and CI/CD environments

## [0.2.0] - 2025-06-24

### Added
- Screenshot functionality via `brp_extras/screenshot` method
- Graceful shutdown via `brp_extras/shutdown` method
- Component format discovery via `brp_extras/discover_format` method

[0.2.1]: https://github.com/natepiano/bevy_brp/extras/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/natepiano/bevy_brp/extras/releases/tag/v0.2.0
