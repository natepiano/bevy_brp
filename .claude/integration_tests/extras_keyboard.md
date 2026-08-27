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
  - `completion_state` should be `"Completed"`

### 3. Custom Duration with Verification
- Test custom duration: `{"keys": ["KeyH", "KeyI"], "duration_ms": 700}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyH", "KeyI"]`
  - `completion_state` should be `"Completed"`
  - `last_duration_ms` should be present and roughly in the range 600-1500
  - The app measures this by detecting press and release on separate frames, so the value runs
    longer than the requested 700ms by up to a couple of frames. Under the parallel suite the app
    can drop to ~10 FPS, which adds ~200-300ms. The wide upper bound absorbs that frame latency;
    the 600ms lower bound is what proves the custom duration was honored rather than the default.

### 4. Modifier Combination with Verification
- Test modifier combination: `{"keys": ["ControlLeft", "KeyA"], "duration_ms": 500}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain both `"ControlLeft"` and `"KeyA"`
  - `complete_modifiers` should contain `"Ctrl"`
  - `completion_state` should be `"Completed"`

### 5. Boundary Conditions with Verification
- Test short duration: `{"keys": ["KeyB"], "duration_ms": 50}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyB"]`
  - `completion_state` should be `"Completed"`
- Test zero duration: `{"keys": ["KeyC"], "duration_ms": 0}`
- Verify reception: read `extras_plugin::KeyboardInputHistory`
  - `last_keys` should contain `["KeyC"]`
  - `completion_state` should be `"Completed"`

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
