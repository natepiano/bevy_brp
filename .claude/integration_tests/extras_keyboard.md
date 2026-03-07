# BRP Extras Keyboard Input Tests

## Objective
Validate brp_extras keyboard input methods: send_keys with various durations, modifiers, and error handling.

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

### 3. Invalid Key Code Test
- Execute send_keys with invalid key code
- Verify appropriate error response

## Expected Results
- Keyboard input works with various durations
- Modifier key combinations function correctly
- Duration boundaries are enforced properly
- Invalid inputs return appropriate errors

## Failure Criteria
STOP if: Any keyboard input method fails unexpectedly, or duration boundaries aren't enforced properly.
