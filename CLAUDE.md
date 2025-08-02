## Code Interactions
- **CRITICAL**: When we make changes to the MCP tool code, you CANNOT test it until I exit and reinstall it because MCP tools run as subprocesses. This means:
  - ❌ DO NOT attempt to run any tests from `.claude/commands/` after making code changes
  - ❌ DO NOT use any `mcp__brp__*` tools after modifying MCP code
  - ✅ DO run unit tests with `cargo test` - these test the code directly
  - ✅ DO build with `cargo build` to check for compilation errors
  - ⚠️ REMEMBER: The MCP tool you're using is the OLD version until I reinstall
