# BRP Extras Keyboard Input Tests

## Objective
Validate brp_extras keyboard input methods: send_keys with various durations, modifiers, and error handling. Verify that keyboard events are actually received by the Bevy app by reading the `KeyboardInputHistory` resource after each successful send.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Basic Keyboard Input with Verification
- Test default duration: `mcp__brp__brp_extras_send_keys` with `["KeyA", "Space"]`
- Verify reception: `mcp__brp__world_get_resources` with resource `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyA", "Space"]`
  - `completed` should be `true`

### 3. Custom Duration with Verification
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyH", "KeyI"]`
  - `completed` should be `true`
  - `last_duration_ms` should be present and roughly in the range 600-900

### 4. Modifier Combination with Verification
- Test modifier combination: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain both `"ControlLeft"` and `"KeyA"`
  - `complete_modifiers` should contain `"Ctrl"`
  - `completed` should be `true`

### 5. Boundary Conditions with Verification
- Test short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyB"]`
  - `completed` should be `true`
- Test zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyC"]`
  - `completed` should be `true`

### 6. Error Conditions (no resource verification needed)
- Test excessive duration: `{"keys": ["KeyE"], "duration_ms": 70000}` (should fail)
- Test invalid key code: execute send_keys with invalid key code, verify appropriate error response

## Expected Results
- Keyboard input events are received by the Bevy app (verified via `KeyboardInputHistory` resource)
- Key codes, modifiers, and completion status are correctly tracked
- Duration boundaries are enforced properly
- Invalid inputs return appropriate errors

## Failure Criteria
STOP if: Any keyboard input method fails unexpectedly, `KeyboardInputHistory` doesn't reflect the sent keys, or duration boundaries aren't enforced properly.
