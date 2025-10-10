# bevy_brp_mcp_macros

Procedural macros for [bevy_brp_mcp](https://crates.io/crates/bevy_brp_mcp).

## Version Alignment

This crate is always versioned identically to `bevy_brp_mcp` to ensure compatibility.

## Macros

- `#[derive(BrpTools)]` - Generates BRP tool implementations from enum variants
- `#[derive(ToolDescription)]` - Generates tool description methods
- `#[derive(ParamStruct)]` - Derives field placement for parameter structs
- `#[derive(ResultStruct)]` - Derives field placement for result structs
- `#[derive(ToolFn)]` - Generates ToolFn trait implementations

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
