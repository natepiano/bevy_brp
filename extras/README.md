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
| 0.18        | 0.18.0-0.18.6   |
| 0.17        | 0.17.0-0.17.2   |
| 0.16        | 0.1 - 0.2       |


## BRP Methods

- **App Lifecycle**: `screenshot`, `shutdown`, `set_window_title`, `get_diagnostics`
- **Keyboard**: `send_keys`, `type_text`
- **Mouse**: `click_mouse`, `double_click_mouse`, `send_mouse_button`, `move_mouse`, `drag_mouse`, `scroll_mouse`
- **Trackpad Gestures** (macOS): `double_tap_gesture`, `pinch_gesture`, `rotation_gesture`

All methods are prefixed with `brp_extras/` (e.g., `brp_extras/screenshot`). See [docs.rs](https://docs.rs/bevy_brp_extras/) for parameter details.

**Screenshot note**: Your Bevy app must have the `png` feature enabled for screenshots to work. Without it, screenshot files will be created but will be 0 bytes.
```toml
bevy = { version = "0.18", features = ["png"] }
```

**Diagnostics note**: `get_diagnostics` requires the `diagnostics` cargo feature (enabled by default). Disable with `default-features = false` if you don't want `FrameTimeDiagnosticsPlugin` added to your app.

## WASM Support

`bevy_brp_extras` compiles on `wasm32` targets. On native platforms, HTTP transport (`RemoteHttpPlugin`) is added automatically. On WASM, only the BRP methods are registered -- you need to provide your own transport (e.g., a WebSocket relay).

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
