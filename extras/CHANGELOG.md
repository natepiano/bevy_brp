# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Support for Bevy 0.17.0 through 0.17.2
- New `brp_extras/send_keys` method for simulating keyboard input
- New `brp_extras/set_window_title` method for changing window title
- Optional `enable_debug_info` parameter for `brp_extras/discover_format` method
  - Provides detailed diagnostic information about type discovery process when enabled
  - Helps troubleshoot format discovery issues with complex types
- Environment variable port override support via `BRP_EXTRAS_PORT`
  - Allows runtime port configuration without code changes
  - Priority: `BRP_EXTRAS_PORT` environment variable > `with_port()` > default port (15702)
  - Enables unique port assignment for testing and CI/CD environments

### Breaking Change
- Removed `brp_extras/discover_format` method in favor of using the BevyBrpMcp tool, `brp_type_guide` method which uses the TypeRegistry to create a more accurate response than this retired method. And given it is built into the mcp tool itself, will not require `BevyBrpExtras` dependency.

## [0.2.0] - 2025-06-24

### Added
- Screenshot functionality via `brp_extras/screenshot` method
- Graceful shutdown via `brp_extras/shutdown` method
- Component format discovery via `brp_extras/discover_format` method

[0.2.1]: https://github.com/natepiano/bevy_brp/extras/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/natepiano/bevy_brp/extras/releases/tag/v0.2.0
