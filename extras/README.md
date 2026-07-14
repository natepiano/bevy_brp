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
| 0.19        | 0.20.0-0.22.0   |
| 0.18        | 0.18.0-0.19.0   |
| 0.17        | 0.17.0-0.17.2   |
| 0.16        | 0.1 - 0.2       |


## BRP Methods

- **App Lifecycle**: `screenshot`, `shutdown`, `set_window_title`, `get_diagnostics`
- **Keyboard**: `send_keys`, `type_text`
- **Mouse**: `click_mouse`, `double_click_mouse`, `send_mouse_button`, `move_mouse`, `drag_mouse`, `scroll_mouse`
- **Trackpad Gestures** (macOS): `double_tap_gesture`, `pinch_gesture`, `rotation_gesture`

All methods are prefixed with `brp_extras/` (e.g., `brp_extras/screenshot`). See [docs.rs](https://docs.rs/bevy_brp_extras/) for parameter details.

### Screenshots

`brp_extras/screenshot` is a terminal watching method: a successful response means the complete PNG has replaced the destination. The request requires `path`; `camera`, `entity`, and `padding` are optional.

Success means the PNG is fully encoded and atomically published; it does not assert that scene content is nonuniform. A minimized, hidden, or fully occluded primary-window surface may legitimately produce a black image on platforms that stop presenting it. Entity captures reflect the selected camera target; retained image or other offscreen targets avoid primary-window presentation dependence when the application is designed to use them.

With no `entity` or `camera`, the method captures the primary window. With only `camera`, it captures that active camera's physical viewport from the final composited render target. `padding` is invalid without `entity`.

With an `entity` ID, the method resolves either Bevy UI computed bounds or an `Aabb` and `GlobalTransform`, then crops the selected camera's final composited render target in physical pixels. Optional `padding` defaults to zero. Complete UI computed components take precedence over an incidental AABB and use `ComputedUiTargetCamera`; a different explicit `camera` is rejected. AABB capture uses an exact optional camera ID or requires exactly one eligible active camera. Extras accepts entity IDs only and does not resolve names.

UI capture honors `InheritedVisibility`, transformed node bounds, inherited clipping, the physical camera viewport, and the live window, image, or manual-texture target extent. UI capture does not use AABB render layers. Partial UI computed state is an initialization error. The `ui` feature is enabled by default; disable default features to retain AABB capture without compiling this crate's UI bounds resolver, imports, or capability. This flag does not guarantee removal of UI crates from the resolved dependency graph because upstream Bevy 0.19 `bevy_remote` already brings that family transitively through `bevy_dev_tools`. Screenshot capture does not add textual snapshots.

AABB capture honors visibility, render layers, the camera frustum, viewport, target bounds, and available selected-view visibility data. Entities marked `NoCpuCulling` still must pass visibility, layer, and frustum checks. Generic AABB capture cannot prove that a custom renderer contributes pixels to the selected target. The output can include overlapping UI, geometry, background, post-processing, and occluders. Use padding for effects extending outside the resolved bounds. Only the selected entity is resolved; descendants are not included automatically. Procedural, custom-rendered, and skinned entities must maintain an AABB that covers their rendered content.

Your Bevy app must have the `png` feature enabled. Without it, the request fails before capture begins.

```toml
bevy = { version = "0.19", features = ["png"] }
```

**Diagnostics note**: `get_diagnostics` requires the `diagnostics` cargo feature (enabled by default). Disable with `default-features = false` if you don't want `FrameTimeDiagnosticsPlugin` added to your app.

## WASM Support

`bevy_brp_extras` compiles on `wasm32` targets. On native platforms, HTTP transport (`RemoteHttpPlugin`) is added automatically. On WASM, only the BRP methods are registered -- you need to provide your own transport (e.g., a WebSocket relay).

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_brp_extras = "0.22.0"
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

### Custom HTTP Transport

For full control over the HTTP transport (address, port, headers), provide your own `RemoteHttpPlugin`:

```rust
use bevy_remote::http::RemoteHttpPlugin;

.add_plugins(BrpExtrasPlugin::with_http_plugin(
    RemoteHttpPlugin::default()
        .with_port(9000)
        .with_address([0, 0, 0, 0])
))
```

`with_port()` and `with_http_plugin()` are mutually exclusive -- the compiler enforces this.

### Plugin Composability

`BrpExtrasPlugin` composes with existing BRP setups. If `RemotePlugin` or `RemoteHttpPlugin` are already added to your app, `BrpExtrasPlugin` will skip adding them and register its methods into the existing `RemoteMethods` resource.

If `RemoteHttpPlugin` is already present, any port configuration (`with_port()` / `BRP_EXTRAS_PORT`) is ignored and a warning is logged.

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
