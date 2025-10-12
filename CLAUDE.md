## Purpose
The purpose of the the bevy_brp_mcp create is to provide mcp protocol bsaed access to a running evy app/game using the Bevy Remote Protocol (brp).  As such a key goal is to have the running mcp server act as an educational to you, the coding agent, in order to properly work with the bevy remote protocol. The primary purpose is the api access but the user wants me to keep in mind that the secondary goal, about educating the agentic coder, with information about how to successfully utilize the available commands.

## Workspace Structure and Purpose
This is a Rust workspace with 4 crates serving distinct roles: `mcp` (MCP server for AI agents), `extras` (Bevy plugin for enhanced BRP methods), `mcp_macros` (procedural macros for code generation), and `test-app` (testing application). The separation enables providing tests bevy apps and examples as well as agentic tests that don't need to be published to crates.io.  The `mcp` crate is published to crates.io as `bevy_brp_mcp`. The `extras` crate is published to crates.io as `bevy_brp_extras`. The `mcp_macros` crate is published to crates.io as `bevy_brp_mcp_macros`. The `test-app` crate and the agentic tests under .claude are not published.

## Meta-Programming Architecture
The `mcp_macros` crate provides 4 key derive macros (`BrpTools`, `ToolDescription`, `ParamStruct`, `ResultStruct`) that automatically generate tool implementations from enum variants and struct definitions. This eliminates boilerplate and ensures consistency - tools are defined declaratively with attributes like `#[brp_tool(brp_method = "world.spawn_entity")]` rather than manually implementing handlers in `mcp/src/brp_tools/brp_type_guide/tool.rs`.

## Agentic Test Framework
Tests in `.claude/commands/tests/` are not bash scripts but structured test specifications in markdown files. They validate BRP operations through a specialized test runner (`.claude/commands/test.md`) that prompts you to run tests using parallel execution (7 tests at once) with port isolation, log verification, and shutdown validation.

## Tool Naming and BRP Protocol Mapping
Tools follow a consistent naming pattern where `ToolName` enum variants (e.g., `WorldSpawnEntity`) map to snake_case MCP tool names (`world_spawn_entity`) and BRP methods (`world.spawn_entity`). The `#[brp_tool]` attribute connects MCP tools to BRP protocol methods, with parameter/result types automatically deriving field placement for proper JSON serialization between MCP and BRP layers.

## MCP Tool Execution Constraints
**CRITICAL**: After modifying MCP tool code, you cannot test it until the user reloads the MCP server because MCP tools run as subprocesses. The tool in use is always the OLD version until reloaded. You need to follow <McpEditProcedure/>.

## getting an example to test with
- use `mcp__brp__brp_launch_example` with extras_plugin if you want to test something

## folders
.claude/bug_reports/ - for bugs we want to track
./claude/commands/ - slash commands
./claude/plans/ - implementation plans
./claude/scripts/ - scripts used by slash commands
./claude/tests/ - predefined test instructions used by ./claude/commands/integration_tests.md
./claude/transient/ - transient .gitignore'd test output
./claude/types/ - transient .gitignore'd test output

<context_window_management>
Your context window will be automatically compacted as it approaches its limit, allowing you to continue working indefinitely from where you left off. Therefore, do not stop tasks early due to token budget concerns. As you approach your token budget limit, save your current progress and state to memory before the context window refreshes. Always be as persistent and autonomous as possible and complete tasks fully, even if the end of your budget is approaching. Never artificially stop any task early regardless of the context remaining.
</context_window_management>

<do_not_act_before_instructions>
Do not jump into implementatation or changes files unless clearly instructed to make changes. When the user's intent is ambiguous, default to providing information, doing research, and providing recommendations rather than taking action. Only proceed with edits, modifications, or implementations when the user explicitly requests them.
</do_not_act_before_instructions>

<use_parallel_tool_calls>
If you intend to call multiple tools and there are no dependencies between the tool calls, make all of the independent tool calls in parallel. Prioritize calling tools simultaneously whenever the actions can be done in parallel rather than sequentially. For example, when reading 3 files, run 3 tool calls in parallel to read all 3 files into context at the same time. Maximize use of parallel tool calls where possible to increase speed and efficiency. However, if some tool calls depend on previous calls to inform dependent values like the parameters, do NOT call these tools in parallel and instead call them sequentially. Never use placeholders or guess missing parameters in tool calls.
</use_parallel_tool_calls>
