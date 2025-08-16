## Purpose
The purpose of the the bevy_brp_mcp create is to provide mcp protocol bsaed access to a running evy app/game using the Bevy Remote Protocol (brp).  As such a key goal is to have the running mcp server act as an educational to you, the coding agent, in order to properly work with the bevy remote protocol. The primary purpose is the api access but the user wants me to keep in mind that the secondary goal, about educating the agentic coder, with information about how to successfully utilize the available commands.

## Workspace Structure and Purpose
This is a Rust workspace with 4 crates serving distinct roles: `mcp` (MCP server for AI agents), `extras` (Bevy plugin for enhanced BRP methods), `mcp_macros` (procedural macros for code generation), and `test-app` (testing application). The separation enables providing tests bevy apps and examples as well as agentic tests that don't need to be published to crates.io.  The `mcp` crate is published to crates.io as `bevy_brp_mcp`. The `extras` crate is published to crates.io as `bevy_brp_extras`. The `mcp_macros` crate is published to crates.io as `bevy_brp_mcp_macros`. The `test-app` crate and the agentic tests under .claude are not published.

## Meta-Programming Architecture
The `mcp_macros` crate provides 4 key derive macros (`BrpTools`, `ToolDescription`, `ParamStruct`, `ResultStruct`) that automatically generate tool implementations from enum variants and struct definitions. This eliminates boilerplate and ensures consistency - tools are defined declaratively with attributes like `#[brp_tool(brp_method = "bevy/spawn")]` rather than manually implementing handlers in `mcp/src/brp_tools/brp_type_schema/tool.rs`.

## Agentic Test Framework
Tests in `.claude/commands/tests/` are not bash scripts but structured test specifications in markdown files. They validate BRP operations through a specialized test runner (`.claude/commands/test.md`) that prompts you to run tests using parallel execution (7 tests at once) with port isolation, log verification, and shutdown validation.

## Tool Naming and BRP Protocol Mapping
Tools follow a consistent naming pattern where `ToolName` enum variants (e.g., `BevySpawn`) map to snake_case MCP tool names (`bevy_spawn`) and BRP methods (`bevy/spawn`). The `#[brp_tool]` attribute connects MCP tools to BRP protocol methods, with parameter/result types automatically deriving field placement for proper JSON serialization between MCP and BRP layers.

## MCP Tool Execution Constraints
**CRITICAL**: After modifying MCP tool code, you cannot test it until the user exits and reinstalls because MCP tools run as subprocesses. The tool in use is always the OLD version until reinstalled. Only unit tests (`cargo test`) and compilation checks work immediately.
