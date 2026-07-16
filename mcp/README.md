# About

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp/mcp#license)
[![Crates.io](https://img.shields.io/crates/d/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![CI](https://github.com/natepiano/bevy_brp/workflows/CI/badge.svg)](https://github.com/natepiano/bevy_brp/actions)

A Model Context Protocol (MCP) server that enables AI coding assistants to launch, inspect, and mutate Bevy applications via the Bevy Remote Protocol (BRP). This tool bridges the gap between coding agents and Bevy by providing comprehensive BRP integration as an MCP server.

## Bevy Compatibility

| bevy        | bevy_brp_mcp    |
|-------------|-----------------|
| 0.19        | 0.22.1          |
| 0.18        | 0.19.0          |
| 0.17        | 0.17.2          |
| 0.16        | 0.1             |

## Features

### Core BRP Operations
- **Entity Management**: Spawn, despawn, query
- **Component Operations**: Get, insert, list, remove, and mutate components on entities
- **Resource Management**: Get, insert, list, remove, and mutate resources
- **Query System**: Entity querying with filters
- **Name Discovery**: Find canonical entity IDs with exact, prefix, suffix, or contains matching
- **Hierarchy Operations**: Reparent entities
- **Type Guide**: Get proper JSON formats for BRP operations using the `brp_type_guide` tool, which provides spawn/insert examples and mutation paths for components and resources

### Application Discovery & Management for your Agent
- **App Discovery**: Find and list Bevy applications in your workspace
- **Build Status**: Check which apps are built and ready to run
- **Launch Management**: Start apps with proper asset loading and logging
- **Example Support**: Discover and run Bevy examples from your projects

### Real-time Monitoring
- **Component Watching**: Monitor component changes on specific entities
- **Log Management**: Captures stdout to a temp file and provides a link to your agent for it to read your logs instead of blocking on running your app.
- **Process Status**: Check if apps are running with BRP enabled

### Enhanced BRP Capabilities
requires [bevy_brp_extras](https://crates.io/crates/bevy_brp_extras)
- `brp_extras/screenshot` - Capture the full primary window or an entity crop by ID or unique exact name
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

## Getting Started
First, install via cargo:
`cargo install bevy_brp_mcp`

Configure your MCP server. For Claude Code, add this to your `~/.claude.json` file:

```json
"mcpServers": {
  "brp": {
    "type": "stdio",
    "command": "bevy_brp_mcp",
    "args": [],
    "env": {}
  }
}
```
That's it!

## Usage

### With AI Coding Assistants

bevy_brp_mcp is designed to be used with AI coding assistants that support MCP (e.g., Claude Code). The MCP server provides tools that allow the AI to:

1. Discover and launch your Bevy applications - with logs stored in your temp dir so they can be accessed by the coding assistant.
2. Inspect and modify entity components in real-time
3. Monitor application state and debug issues
4. Take screenshots, send keyboard/mouse input, query diagnostics, and manage application lifecycle (requires `bevy_brp_extras`)

### Setting Up Your Bevy App

For full functionality, your Bevy app should include BRP support:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy::remote::RemotePlugin::default()) // Enable BRP
        .run();
}
```

For enhanced features such as asking the coding agent to take a screenshot or to send keyboard input to your running app, also add [bevy_brp_extras](https://crates.io/crates/bevy_brp_extras):

```rust
use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin) // Enhanced BRP features
        .run();
}
```

In either case you'll need to make sure to enable bevy's "bevy_remote" feature.

### Application-defined BRP methods and agent tools

`rpc_discover` exhaustively lists methods in Bevy's live `RemoteMethods` resource, including
application-defined and built-in methods. Bevy 0.19 reports their names but not descriptions or
parameter/result schemas.

Applications using `BrpExtrasPlugin` can publish a curated subset of existing instant methods for
agents. List those records first, then execute a selected backing method:

```text
cargo run -p bevy_brp_extras --example agent_tool_registration
brp_list_agent_tools(port: 15702)
brp_execute(
    port: 15702,
    method: "example/multiply",
    params: { "value": 6, "factor": 7 }
)
```

`brp_list_agent_tools` returns the public structured `result` with `usage` and `tools`. Each record
in `result.tools` contains an agent-facing name and description, its exact backing BRP method, and
optional raw JSON schemas for that method's JSON-RPC parameters and result. Follow `result.usage`
and invoke the selected method with `brp_execute`:

```json
{
  "port": 15702,
  "method": "example/multiply",
  "params": {
    "value": 6,
    "factor": 7
  }
}
```

`brp_execute` confirms that the selected app reports the method through `rpc.discover` before
forwarding the raw parameters. Catalog records are not native MCP tools. Every published record
names a BRP method, while most registered BRP methods need not be in the curated agent list.

Each catalog request validates all published records against the live `RemoteMethods` resource. If
any backing method is missing or watching, no partial list is returned; the BRP error data identifies
the rejected entry through stable `name`, `method`, and `reason` fields.

### Find entities by name

`world_find_entities_by_name` is an MCP-local convenience tool built from the standard BRP
`world.query` method. It requires `RemotePlugin` and reflected `bevy_ecs::name::Name` components,
but does not require `bevy_brp_extras`.

Names are matched case-sensitively. Set `match_mode` to `exact` (the default), `prefix`, `suffix`,
or `contains`; `*` is always a literal character, not wildcard syntax. Results contain each full
name and canonical `u64` entity ID, sorted by entity ID. Duplicate names return multiple entries.

Use a non-exact mode to discover candidates, then pass the returned canonical entity ID to later
inspection, mutation, watch, or screenshot operations:

```json
{
  "name": "List",
  "match_mode": "suffix",
  "port": 15702
}
```

### Capture screenshots

`brp_extras_screenshot` is one terminal tool for full primary-window images, camera viewports,
entity-ID crops, and unique exact-name crops. Capture the full primary window with:

```json
{
  "path": "/tmp/full.png",
  "port": 15702
}
```

Capture an active camera's physical viewport with:

```json
{
  "camera": 4294967297,
  "path": "/tmp/camera.png",
  "port": 15702
}
```

Capture a known canonical entity ID with:

```json
{
  "entity": 4294967298,
  "path": "/tmp/entity.png",
  "port": 15702
}
```

To screenshot NatesList in one call:

```json
{
  "name": "NatesList",
  "path": "/tmp/nates-list.png",
  "port": 15702
}
```

Use `entity` instead of `name` when the canonical ID is known. With no selector or camera, the tool
captures the full primary window. With only `camera`, it captures that camera's physical viewport.
Supplying both selectors is invalid. With a selector, `camera` chooses the camera used for the
entity crop. `padding` applies only to entity or name captures and defaults to zero physical
pixels. Exact names are case-sensitive and must resolve to one entity. Zero or duplicate matches
return guidance; use `world_find_entities_by_name` for non-exact discovery or to choose among
duplicates.

The tool returns only after `bevy_brp_extras` has captured the final composited camera target,
encoded a complete PNG, and atomically published the requested path. Entity screenshots crop that
composited result, so other layers and partially covered pixels inside the bounds remain visible.
Camera inference is available only when one eligible camera is unambiguous; Bevy has no universal
primary camera.

Success means the PNG is fully encoded and atomically published; it does not assert that scene
content is nonuniform. A minimized, hidden, or fully occluded primary-window surface may
legitimately produce a black image on platforms that stop presenting it. Entity captures reflect
the selected camera target; retained image or other offscreen targets avoid primary-window
presentation dependence when the application is designed to use them.

Generic AABB crops are supported. Complete Bevy UI components use UI bounds when the default-enabled
`bevy_brp_extras` `ui` feature is enabled. That feature gates the extras crate's UI resolver,
imports, and capability; it is not a promise that upstream UI crates vanish from `cargo tree`,
because Bevy 0.19 `bevy_remote` already brings that dependency family transitively. Enabling `ui`
also enables the Bevy text and sprite dependencies required by Bevy UI. Textual UI-tree or
`snapshot` inspection remains a separate capability.

## Example Workflow

1. **Discover**: Use `brp_list_bevy` to find available applications and examples
2. **Launch**: Use `brp_launch` to start your game with proper logging
3. **Inspect**: Use `world_query` or `world_find_entities_by_name` to find entities of interest
4. **Monitor**: Use `world_get_components_watch` to observe entity changes in real-time
5. **Modify**: Use `world_mutate_components` to adjust entity properties
6. **Trigger**: Use `world_trigger_event` to trigger events for your observers
7. **Debug**: Use `read_log` to examine application output
8. **Capture**: Use `brp_extras_screenshot` to document current state
9. **Interact**: Use `brp_extras_send_keys` to send keyboard input for testing
10. **Diagnose**: Use `brp_extras_get_diagnostics` to check FPS and frame time

## Logging

All launched applications create detailed log files in `/tmp/` with names like:
- `bevy_brp_mcp_myapp_1234567890.log` (application logs)
- `bevy_brp_mcp_watch_123_get_456_1234567890.log` (monitoring logs)

Use the log management tools to view and clean up these files.

## License

Dual-licensed under either:
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.
