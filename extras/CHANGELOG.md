# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **`get_diagnostics` method**: New BRP method for querying FPS and frame time diagnostics from Bevy's `DiagnosticsStore`. Returns current, average, and smoothed FPS values, frame time in milliseconds, total frame count, and history buffer metadata. Defensively installs `FrameTimeDiagnosticsPlugin` if not already present.
- **`diagnostics` feature**: New feature flag (enabled by default) that controls FPS diagnostics support. Disable with `default-features = false` to exclude the diagnostics system and `FrameTimeDiagnosticsPlugin`.

## [0.18.3] - 2026-02-17

### Fixed
- Fixed simulated mouse and keyboard input not triggering Bevy's picking system by dual-writing events to the `WindowEvent` message channel

## [0.18.2] - 2026-02-17

### Added
- **Mouse input methods**: Nine new BRP methods for comprehensive mouse control
  - `click_mouse` - Simple click with configurable button (Left, Right, Middle, Back, Forward)
  - `double_click_mouse` - Double click with configurable delay between clicks
  - `send_mouse_button` - Press and hold mouse button for specified duration
  - `move_mouse` - Move cursor with delta (relative) or absolute positioning
  - `drag_mouse` - Smooth drag operation with interpolated movement over frames
  - `scroll_mouse` - Mouse wheel scrolling (line-based or pixel-based, horizontal/vertical)
  - `double_tap_gesture` - Trackpad double tap gesture (macOS)
  - `pinch_gesture` - Trackpad pinch-to-zoom gesture (macOS)
  - `rotation_gesture` - Trackpad rotation gesture (macOS)

### Changed
- Mouse operations now default to the last window the cursor was moved to instead of always defaulting to the primary window when no explicit `window` parameter is provided

## [0.18.1] - 2026-02-10

### Added
- **`type_text` method**: New BRP method for sequential character typing. Types text one character per frame, handling shift for uppercase/symbols and mapping unmappable characters as skipped. Thanks @tobert!

### Fixed
- **`send_keys` text field population**: Fixed `send_keys` to populate the `text` field on `KeyboardInput` events, enabling proper text input in Bevy UI text fields. Thanks @tobert!

## [0.18.0] - 2026-01-15

### Changed
- Updated dependency to Bevy 0.18.0 stable release

## [0.18.0-rc.1] - 2025-12-21

### Changed
- **Upgraded to Bevy 0.18.0-rc.1**: Updated bevy dependency from 0.17.x to 0.18.0-rc.1
  - `BorderRadius` now set via `Node.border_radius` field instead of standalone component
  - `AnimationTarget` split into `AnimationTargetId` + `AnimatedBy` components
  - `Image::reinterpret_stacked_2d_as_array` now returns `Result`

## [0.17.2] - 2025-11-20

### Changed
- Version bump to 0.17.2 to maintain workspace version synchronization

## [0.17.1] - 2025-11-20

### Changed
- Version bump to 0.17.1 to maintain workspace version synchronization

## [0.17.0] - 2025-10-31

### Changed
- Version numbering now tracks Bevy releases for clearer compatibility signaling

### Added
- Support for Bevy 0.17.0 through 0.17.2
- New `brp_extras/send_keys` method for simulating keyboard input
- New `brp_extras/set_window_title` method for changing window title
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
