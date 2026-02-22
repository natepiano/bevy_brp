# About

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp/mcp#license)
[![Crates.io](https://img.shields.io/crates/d/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![CI](https://github.com/natepiano/bevy_brp/workflows/CI/badge.svg)](https://github.com/natepiano/bevy_brp/actions)

A Model Context Protocol (MCP) server that enables AI coding assistants to launch, inspect, and mutate Bevy applications via the Bevy Remote Protocol (BRP). This tool bridges the gap between coding agents and Bevy by providing comprehensive BRP integration as an MCP server.

## Bevy Compatibility

| bevy        | bevy_brp_mcp    |
|-------------|-----------------|
| 0.18        | 0.18.0-0.18.5   |
| 0.17        | 0.17.0-0.17.2   |
| 0.16        | 0.1             |

The bevy_brp_mcp crate follows Bevy's version numbering and releases new versions for each Bevy release.

## Features

### Core BRP Operations
- **Entity Management**: Spawn, despawn, query
- **Component Operations**: Get, insert, list, remove, and mutate components on entities
- **Resource Management**: Get, insert, list, remove, and mutate resources
- **Query System**: Entity querying with filters
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

## Example Workflow

1. **Discover**: Use `list_bevy_apps` to find available applications
2. **Launch**: Use `launch_bevy_app` to start your game with proper logging
3. **Inspect**: Use `world_query` to find entities of interest
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
