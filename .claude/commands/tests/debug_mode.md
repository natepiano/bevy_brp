# Debug Mode Tests

## Objective
Validate debug mode functionality for both BRP MCP layer and bevy_brp_extras plugin.

## Test Steps

### 1. Enable BRP MCP Debug Mode
- Execute `mcp__brp__brp_set_debug_mode` with `enabled: true`
- Verify debug mode is enabled successfully

### 2. Test MCP Debug Output
- Execute any brp_extras operation (e.g., `mcp__brp__brp_extras_discover_format`)
- Verify response contains `brp_mcp_debug_info` field
- Check debug info includes parameter extraction, method resolution details

### 3. Disable BRP MCP Debug Mode
- Execute `mcp__brp__brp_set_debug_mode` with `enabled: false`
- Verify debug mode is disabled

### 4. Enable BRP Extras Debug Mode Only
- Execute `mcp__brp__brp_extras_set_debug_mode` with `enabled: true`
- Verify extras debug mode is enabled

### 5. Test Extras Debug Output
- Execute `mcp__brp__brp_extras_discover_format` with types array
- Verify response contains `brp_extras_debug_info` field
- Verify NO `brp_mcp_debug_info` field present (MCP debug is off)

### 6. Test Both Debug Modes Together
- Enable both debug modes
- Execute brp_extras operation
- Verify both `brp_mcp_debug_info` and `brp_extras_debug_info` are present

### 7. Disable All Debug Modes
- Disable both debug modes
- Execute operation and verify no debug fields present

## Expected Results
- ✅ Debug modes toggle correctly
- ✅ MCP debug info appears when enabled
- ✅ Extras debug info appears when enabled  
- ✅ Debug modes work independently
- ✅ Combined debug output is clean (no duplication)
- ✅ Debug output is disabled when modes are off

## Failure Criteria
STOP if: Debug mode toggles fail, debug output is malformed, or debug modes interfere with each other.