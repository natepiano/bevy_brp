# About

[![Crates.io](https://img.shields.io/crates/v/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/natepiano/bevy_brp/mcp#license)
[![Crates.io](https://img.shields.io/crates/d/bevy_brp_mcp.svg)](https://crates.io/crates/bevy_brp_mcp)
[![CI](https://github.com/natepiano/bevy_brp/workflows/CI/badge.svg)](https://github.com/natepiano/bevy_brp/actions)

A Model Context Protocol (MCP) server that enables AI coding assistants to control launch, inspect and mutate Bevy applications via the Bevy Remote Protocol (BRP). This tool bridges the gap between coding agents and Bevy by providing comprehensive BRP integration as an MCP server.

## Bevy Compatibility

| bevy | bevy_brp_mcp |
|------|--------------|
| 0.16 | 0.1          |

The bevy_brp_mcp crate follows Bevy's version numbering and releases new versions for each Bevy release.

## Features

### Core BRP Operations
- **Entity Management**: Spawn, despawn, query
- **Component Operations**: Get, insert, remove, and mutate components on entities
- **Resource Management**: Access and modify global resources
- **Query System**: Advanced entity querying with filters
- **Hierarchy Operations**: Parent-child entity relationships

### Application Discovery & Management
- **App Discovery**: Find and list Bevy applications in your workspace
- **Build Status**: Check which apps are built and ready to run
- **Launch Management**: Start apps with proper asset loading and logging
- **Example Support**: Discover and run Bevy examples from your projects

### Real-time Monitoring
- **Component Watching**: Monitor component changes on specific entities
- **Log Management**: Centralized logging for all launched applications
- **Process Status**: Check if apps are running with BRP enabled

### Enhanced BRP Integration
requires `bevy_brp_extras`

- **Type Discovery**: Get correct JSON formats for BRP operations using type schema introspection
- **Screenshot Capture**: Take screenshots of running Bevy applications
- **Keyboard Input**: Send keyboard input to Bevy applications for testing and automation
- **Key Code Discovery**: List all available keyboard key codes for input operations
- **Graceful Shutdown**: Clean application termination

## Getting started
first, install via cargo:
`cargo install bevy_brp_mcp`

configure your mcp server - for claude code this would be in the `~/.claude.json` file.

```json
"mcpServers": {
  "brp": {
    "type": "stdio",
    "command": "bevy_brp_mcp",
    "args": [],
    "env": {}
  }
},
```
that's it!

## Usage

### With AI Coding Assistants

bevy_brp_mcp is designed to be used with AI coding assistants that support MCP (like Claude). The MCP server provides tools that allow the AI to:

1. Discover and launch your Bevy applications - with logs stored in your temp dir so they can be accessed by the coding assistant.
2. Inspect and modify entity components in real-time
3. Monitor application state and debug issues
4. Take screenshots and manage application lifecycle (requries `bevy_brp_extras`)

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

For enhanced features like screenshots and format discovery, also add [bevy_brp_extras](https://github.com/natepiano/bevy_brp/extras):

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

## Integration with bevy_brp_extras

This crate is designed to work seamlessly with [bevy_brp_extras](https://github.com/natepiano/bevy_brp/extras). When both are used together:

1. Add `BrpExtrasPlugin` to your Bevy app for enhanced BRP features
2. Use `bevy_brp_mcp` with your AI coding assistant
3. Additional methods like screenshot, shutdown, and keyboard input will be automatically available
4. Get proper JSON formats for complex BRP operations using the `brp_type_guide` tool. The value returned from bevy/registry/schema does not tell you exactly what is expected by the brp spawn/insert/mutate calls. The `brp_type_guide` tool provides enhanced format information that will make it easier for your coding agent to know how to use spawn/insert/mutate.

## Example Workflow

1. **Discovery**: Use `list_bevy_apps` to find available applications
2. **Launch**: Use `launch_bevy_app` to start your game with proper logging
3. **Inspect**: Use `world_query` to find entities of interest
4. **Monitor**: Use `world_get_components_watch` to observe entity changes in real-time
5. **Modify**: Use `bevy_mutate_component` to adjust entity properties
6. **Debug**: Use `read_log` to examine application output
7. **Capture**: Use `brp_extras_screenshot` to document current state
8. **Interact**: Use `brp_extras_send_keys` to send keyboard input for testing

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
