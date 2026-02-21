# About

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_extras.svg)](https://crates.io/crates/bevy_brp_extras)
[![Documentation](https://docs.rs/bevy_brp_extras/badge.svg)](https://docs.rs/bevy_brp_extras/)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp/extras#license)
[![Crates.io](https://img.shields.io/crates/d/bevy_brp_extras.svg)](https://crates.io/crates/bevy_brp_extras)
[![CI](https://github.com/natepiano/bevy_brp/workflows/CI/badge.svg)](https://github.com/natepiano/bevy_brp/actions)

bevy_brp_extras does two things
1. Configures your app for bevy remote protocol (BRP)
2. Adds additional methods that can be used with BRP

## Supported Bevy Versions

| bevy        | bevy_brp_extras |
|-------------|-----------------|
| 0.18        | 0.18.0-0.18.4   |
| 0.17        | 0.17.0-0.17.2   |
| 0.16        | 0.1 - 0.2       |


## Features

Adds the following Bevy Remote Protocol methods:
- `brp_extras/screenshot` - Capture screenshots of the primary window
- `brp_extras/shutdown` - Gracefully shutdown the application
- `brp_extras/send_keys` - Send keyboard input to the application
- `brp_extras/type_text` - Type text sequentially (one character per frame)
- `brp_extras/set_window_title` - Change the primary window title
- `brp_extras/click_mouse` - Click mouse button
- `brp_extras/double_click_mouse` - Double click mouse button
- `brp_extras/send_mouse_button` - Press and hold mouse button
- `brp_extras/move_mouse` - Move mouse cursor (delta or absolute)
- `brp_extras/drag_mouse` - Drag mouse with smooth interpolation
- `brp_extras/scroll_mouse` - Mouse wheel scrolling
- `brp_extras/double_tap_gesture` - Trackpad double tap gesture (macOS)
- `brp_extras/pinch_gesture` - Trackpad pinch gesture (macOS)
- `brp_extras/rotation_gesture` - Trackpad rotation gesture (macOS)
- `brp_extras/get_diagnostics` - Query FPS and frame time diagnostics

## Cargo Features

| Feature | Default | Description |
|---------|---------|-------------|
| `diagnostics` | Yes | Enables the `brp_extras/get_diagnostics` method and installs Bevy's `FrameTimeDiagnosticsPlugin` |

When enabled, `BrpExtrasPlugin` will install Bevy's `FrameTimeDiagnosticsPlugin` if it hasn't been added already. However, the reverse is not safe -- if you add `FrameTimeDiagnosticsPlugin` after `BrpExtrasPlugin` has already installed it, Bevy will panic due to the duplicate plugin. Either add it before `BrpExtrasPlugin`, or let `BrpExtrasPlugin` handle it.

To disable diagnostics entirely (e.g., if you don't want `FrameTimeDiagnosticsPlugin` added to your app):

```toml
[dependencies]
bevy_brp_extras = { version = "0.18.4", default-features = false }
```

## WASM Support

`bevy_brp_extras` compiles on `wasm32` targets. On native platforms, HTTP transport (`RemoteHttpPlugin`) is added automatically. On WASM, only the BRP methods are registered -- you need to provide your own transport (e.g., a WebSocket relay).

```toml
# Works on both native and wasm32
[dependencies]
bevy_brp_extras = "0.18.4"
```

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_brp_extras = "0.18.4"
```

Add the plugin to your Bevy app

```rust
use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin) // will listen on BRP default port 15702
        .run();
}
```

### Custom Port

You can specify a custom port for the BRP server:

```rust
.add_plugins(BrpExtrasPlugin::with_port(8080))
```

Alternatively, you can set the port at runtime using the `BRP_EXTRAS_PORT` environment variable:

```bash
BRP_EXTRAS_PORT=8080 cargo run
```

Port priority: `BRP_EXTRAS_PORT` environment variable > `with_port()` > default port (15702)

## BRP Method Details

### Screenshot
- **Method**: `brp_extras/screenshot`
- **Parameters**: `path` (string, required) - file path where the screenshot should be saved
- **Returns**: Success status with the absolute path where the screenshot will be saved

**Important**: Your Bevy app must have the `png` feature enabled for screenshots to work:
```toml
[dependencies]
bevy = { version = "0.18", features = ["png"] }
```
Without this feature, screenshot files will be created but will be 0 bytes as Bevy cannot encode the image data.

### Shutdown
- **Method**: `brp_extras/shutdown`
- **Parameters**: None
- **Returns**: Success status with shutdown confirmation

### Send Keys
- **Method**: `brp_extras/send_keys`
- **Parameters**:
  - `keys` (array of strings, required): Key codes to send (e.g., `["KeyA", "Space", "Enter"]`)
  - `duration_ms` (number, optional): How long to hold keys before releasing in milliseconds (default: 100, max: 60000)

Simulates keyboard input by sending press and release events for the specified keys. Keys are pressed simultaneously and held for the specified duration before being released.

### Type Text
- **Method**: `brp_extras/type_text`
- **Parameters**: `text` (string, required) - text to type sequentially

Types characters one per frame with proper key press/release cycles. Unlike `send_keys` which sends all keys simultaneously (for chords and shortcuts), `type_text` queues characters for sequential input.

### Set Window Title
- **Method**: `brp_extras/set_window_title`
- **Parameters**: `title` (string, required) - the new title for the primary window

### Mouse Input Methods

All mouse methods accept an optional `window` (number) parameter to target a specific window (defaults to primary window).

- **`click_mouse`** - `button` (string, required): "Left", "Right", "Middle", "Back", or "Forward"
- **`double_click_mouse`** - `button` (string, required), `delay_ms` (number, optional, default: 250)
- **`send_mouse_button`** - `button` (string, required), `duration_ms` (number, optional, default: 100, max: 60000)
- **`move_mouse`** - provide either `delta` [x, y] or `position` [x, y], not both
- **`drag_mouse`** - `button` (string, required), `start` [x, y], `end` [x, y], `frames` (number)
- **`scroll_mouse`** - `x` (number), `y` (number), `unit` ("Line" or "Pixel")

### Trackpad Gestures (macOS)
- **`double_tap_gesture`** - No parameters
- **`pinch_gesture`** - `delta` (number): positive = zoom in, negative = zoom out
- **`rotation_gesture`** - `delta` (number): rotation in radians

### Get Diagnostics
- **Method**: `brp_extras/get_diagnostics`
- **Parameters**: None
- **Requires**: `diagnostics` feature (enabled by default)

Returns current, average, and smoothed values for both FPS and frame time:

```json
{
  "fps": { "current": 60.0, "average": 59.8, "smoothed": 59.9, "history_len": 120, "max_history_len": 120, "history_duration_secs": 2.0 },
  "frame_time_ms": { "current": 16.6, "average": 16.7, "smoothed": 16.7 },
  "frame_count": 3600
}
```

## Integration with bevy_brp_mcp

This crate is designed to work with [bevy_brp_mcp](https://github.com/natepiano/bevy_brp/mcp), which provides a Model Context Protocol (MCP) server for controlling Bevy apps. When both are used together:

1. Add `BrpExtrasPlugin` to your Bevy app
2. Use `bevy_brp_mcp` with your AI coding assistant
3. All methods are automatically discovered and made available as MCP tools

## License

Dual-licensed under either:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
