# BRP Extras Input Tests

## Objective
Validate brp_extras input methods: send_keys and type_text.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Keyboard Input Tests
- Test default duration: `mcp__brp__brp_extras_send_keys` with `["KeyA", "Space"]`
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Test modifier combinations: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Test boundary conditions:
  - Short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
  - Zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Test error condition: `{"keys": ["KeyE"], "duration_ms": 70000}` (should fail)

### 3. Text Input Tests
- Clear the `TextInputContent` resource: `mcp__brp__world_insert_resources` with resource `extras_plugin::TextInputContent` and value `{"text": ""}`
- Test basic typing: `mcp__brp__brp_extras_type_text` with `{"text": "hello"}`
- Verify text appears: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello"}`
- Test sequential typing: `mcp__brp__brp_extras_type_text` with `{"text": " world"}`
- Verify concatenation: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world"}`
- Test special characters: `mcp__brp__brp_extras_type_text` with `{"text": "!@#"}`
- Verify special chars: `mcp__brp__world_get_resources` with resource `extras_plugin::TextInputContent`, should return `{"text": "hello world!@#"}`
- Test unmappable characters: `mcp__brp__brp_extras_type_text` with text containing unmappable chars
- Verify skipped array is populated in response

### 4. Invalid Key Code Test
- Execute send_keys with invalid key code
- Verify appropriate error response

## Expected Results
- Keyboard input works with various durations
- Modifier key combinations function correctly
- Duration boundaries are enforced properly
- Text typing works sequentially and accumulates correctly
- Special characters are typed properly
- Unmappable characters are reported as skipped
- Invalid inputs return appropriate errors

## Failure Criteria
STOP if: Any input method fails unexpectedly, keyboard input doesn't work, or text typing doesn't accumulate properly.
