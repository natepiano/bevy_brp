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
| 0.19        | 0.22.1          |
| 0.18        | 0.19.0          |
| 0.17        | 0.17.2          |
| 0.16        | 0.2             |


## BRP Methods

- **App Lifecycle**: `screenshot`, `shutdown`, `set_window_title`, `get_diagnostics`
- **Keyboard**: `send_keys`, `type_text`
- **Mouse**: `click_mouse`, `double_click_mouse`, `send_mouse_button`, `move_mouse`, `drag_mouse`, `scroll_mouse`
- **Trackpad Gestures** (macOS): `double_tap_gesture`, `pinch_gesture`, `rotation_gesture`
- **Agent Tools**: `agent_tools`

All methods are prefixed with `brp_extras/` (e.g., `brp_extras/screenshot`). See [docs.rs](https://docs.rs/bevy_brp_extras/) for parameter details.

### Screenshots

`brp_extras/screenshot` writes a PNG to `path` and returns only after the complete file is in place.

- With no `camera` or `entity`, it captures the primary window.
- With only `camera`, it captures that camera's viewport.
- With `entity`, it crops to that entity as seen by the selected camera. `padding` adds physical pixels around the crop and defaults to zero.

UI nodes use their computed UI bounds; other entities use `Aabb` and `GlobalTransform`. The entity must be visible to the selected camera. If no camera is given, exactly one eligible active camera must be available. The default `ui` feature enables UI bounds; AABB capture still works without it.

The crop comes from the final composited target, so it may include overlapping UI, geometry, effects, or occluders. It covers only the selected entity's bounds—children are not added automatically. Extras accepts entity IDs, not names; `bevy_brp_mcp` can resolve names before calling it.

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
bevy_brp_extras = "0.22.1"
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

## BRP methods and agent tools

This crate is designed to work with [bevy_brp_mcp](https://github.com/natepiano/bevy_brp/mcp), which provides a Model Context Protocol (MCP) server for controlling Bevy apps.

Registering a remote method in `RemoteMethods` makes it callable and visible through exhaustive
`rpc.discover` transport discovery. Calling `register_agent_tool` is separate: it publishes a
description and raw JSON parameter/result schemas that teach an agent how to call one existing
instant BRP method. It does not create a native MCP tool.

Every published agent entry names a BRP method, while most BRP methods need not be agent tools. The
complete runnable [registration example](examples/agent_tool_registration.rs) adds
`BrpExtrasPlugin` first, inserts `example/multiply` into `RemoteMethods`, ends that resource's
mutable borrow, and then publishes:

```rust
app.register_agent_tool(
    AgentTool::new(
        "example_multiply",
        "example/multiply",
        "Multiply two signed integers",
    )
    .params_schema_for::<MultiplyParams>()
    .result_schema_for::<MultiplyResult>(),
);
```

Use the MCP workflow in this order:

```text
cargo run -p bevy_brp_extras --example agent_tool_registration
brp_list_agent_tools(port: 15702)
brp_execute(
    port: 15702,
    method: "example/multiply",
    params: { "value": 6, "factor": 7 }
)
```

`brp_list_agent_tools` returns the public structured `result` with `usage` and `tools`. Agents
should follow `result.usage`, select a record from `result.tools`, pass its `method` to
`brp_execute`, and supply raw `params` matching its `params_schema`.
`rpc.discover` remains the exhaustive list of registered BRP methods; the published agent list is
the curated subset with descriptions and optional raw schemas.

The `brp_extras/agent_tools` endpoint validates every published entry against the live
`RemoteMethods` resource for each request. If any backing method is missing or watching, the
request returns no partial list and its BRP error data identifies the rejected entry through stable
`name`, `method`, and `reason` fields.

Add `BrpExtrasPlugin` to install the catalog endpoint, publish selected entries with
`register_agent_tool`, list them with `brp_list_agent_tools`, and invoke their backing methods with
`brp_execute`.

## License

Dual-licensed under either:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
