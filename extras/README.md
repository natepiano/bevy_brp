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

| bevy | bevy_brp_extras |
|------|-----------------|
| 0.17 | 0.17.0-0.17.2   |
| 0.16 | 0.1 - 0.2       |


## Features

Adds the following Bevvy Remote Protocol methods:
- `brp_extras/screenshot` - Capture screenshots of the primary window
- `brp_extras/shutdown` - Gracefully shutdown the application
- `brp_extras/send_keys` - Send keyboard input to the application
- `brp_extras/set_window_title` - Change the primary window title

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_brp_extras = "0.17.2"
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
- **Parameters**:
  - `path` (string, required): File path where the screenshot should be saved
- **Returns**: Success status with the absolute path where the screenshot will be saved

**Important**: Your Bevy app must have the `png` feature enabled for screenshots to work:
```toml
[dependencies]
bevy = { version = "0.16", features = ["png"] }
```
Without this feature, screenshot files will be created but will be 0 bytes as Bevy cannot encode the image data.

**Note**: If you're not using this with [bevy_brp_mcp](https://github.com/natepiano/bevy_brp/mcp), you'll need to tell your AI agent that this method requires a `path` parameter, or let it discover this by trying the method and getting an error message.

### Shutdown
- **Method**: `brp_extras/shutdown`
- **Parameters**: None
- **Returns**: Success status with shutdown confirmation

### Send Keys
- **Method**: `brp_extras/send_keys`
- **Parameters**:
  - `keys` (array of strings, required): Key codes to send (e.g., `["KeyA", "Space", "Enter"]`)
  - `duration_ms` (number, optional): How long to hold keys before releasing in milliseconds (default: 100, max: 60000)
- **Returns**: Success status with the keys sent and duration used

Simulates keyboard input by sending press and release events for the specified keys. Keys are pressed simultaneously and held for the specified duration before being released.

**Example:**
```bash
# Send "hi" by pressing H and I keys
curl -X POST http://localhost:15702/brp_extras/send_keys \
  -H "Content-Type: application/json" \
  -d '{"keys": ["KeyH", "KeyI"]}'

# Hold space key for 2 seconds
curl -X POST http://localhost:15702/brp_extras/send_keys \
  -H "Content-Type: application/json" \
  -d '{"keys": ["Space"], "duration_ms": 2000}'
```

### Set Window Title
- **Method**: `brp_extras/set_window_title`
- **Parameters**:
  - `title` (string, required): The new title for the primary window
- **Returns**: Success status with the old and new window titles

Changes the title of the primary window.

**Example:**
```bash
curl -X POST http://localhost:15702/brp_extras/set_window_title \
  -H "Content-Type: application/json" \
  -d '{"title": "My Game - Level 2"}'
```

## Integration with bevy_brp_mcp

This crate is designed to work seamlessly with [bevy_brp_mcp](https://github.com/natepiano/bevy_brp/mcp), which provides a Model Context Protocol (MCP) server for controlling Bevy apps. When both are used together:

1. Add `BrpExtrasPlugin` to your Bevy app
2. Use `bevy_brp_mcp` with your AI coding assistant
3. The additional methods will be automatically discovered and made available in the MCP server so you won't have to manually implement or execute (as with the curl examples above)

## License

Dual-licensed under either:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
