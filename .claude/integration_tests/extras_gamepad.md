# BRP Extras Gamepad Tests

## Objective
Validate the brp_extras simulated gamepad methods: connect, button tap, hold and release, axis, disconnect, and error handling. Verify that the input reaches the Bevy app by reading the `GamepadInputHistory` resource, which the app fills from Bevy's processed gamepad messages.

**NOTE**: The extras_plugin app is already running on the specified port - focus on testing brp_extras functionality, not app management.

## Test Steps

### 1. Runner-Managed App Context
- The `extras_plugin` app is already running on the assigned `{{PORT}}`
- Do not launch or shutdown the app in this test

### 2. Connect
- `mcp__brp__brp_extras_connect_gamepad` with `{"name": "Spec pad"}`
- Response `result.gamepad` is the entity; call it `G` below
- Verify: `mcp__brp__world_query` with `Name`, filtered `with` `bevy_input::gamepad::Gamepad`, lists entity `G` named `"Spec pad"`
  - The app spawns its own `Gamepad` test entities too; only check that `G` is among them
- `Gamepad` itself does not serialize over BRP (its button maps have non-string keys), so read state through `GamepadInputHistory`, not `world_get_components`

### 3. Hold and Release
- `mcp__brp__brp_extras_set_gamepad_button` with `{"gamepad": G, "button": "South", "value": 1.0}`
- Verify: `mcp__brp__world_get_resources` with resource `extras_plugin::GamepadInputHistory`
  - `pressed_buttons` should be `["South"]`
- Set `{"gamepad": G, "button": "South", "value": 0.0}` with the same tool
- Verify: `pressed_buttons` is `[]` and `last_released` is `"South"`

### 4. Timed Release
- `mcp__brp__brp_extras_send_gamepad_button` with `{"gamepad": G, "button": "East"}` (default 100 ms)
- Within 1 second, verify: `pressed_buttons` is `[]` and `last_released` is `"East"`
- Send `{"gamepad": G, "button": "North", "duration_ms": 300}` with the same tool; within 1 second, verify `pressed_buttons` is `[]` and `last_released` is `"North"`
- Send `{"gamepad": G, "button": "West", "duration_ms": 300}`, then immediately call `mcp__brp__brp_extras_set_gamepad_button` with `{"gamepad": G, "button": "West", "value": 1.0}`; after 1 second verify `pressed_buttons` still contains `"West"` because set cancelled the pending release
- Release West with `mcp__brp__brp_extras_set_gamepad_button` and `{"gamepad": G, "button": "West", "value": 0.0}` before continuing

### 5. Axis
- `mcp__brp__brp_extras_set_gamepad_axis` with `{"gamepad": G, "axis": "LeftStickX", "value": -0.75}`
- Verify: `last_axis` is `"LeftStickX"` and `last_axis_value` is between -0.8 and -0.7
  - Bevy's default dead zone rescales the raw value, so -0.75 arrives as about -0.737

### 6. Error Conditions (no resource verification needed)
- Excessive duration: `{"gamepad": G, "button": "South", "duration_ms": 70000}` should fail
- Unknown button: `{"gamepad": G, "button": "Bogus"}` should fail and list the valid names
- Out-of-range button value: `mcp__brp__brp_extras_set_gamepad_button` with `{"gamepad": G, "button": "South", "value": 1.5}` should fail
- Out-of-range axis: `{"gamepad": G, "axis": "LeftStickX", "value": 1.5}` should fail
- Not a simulated gamepad: `{"gamepad": 1, "button": "South"}` should fail naming entity `1`

### 7. Disconnect
- `mcp__brp__brp_extras_disconnect_gamepad` with `{"gamepad": G}`
- Verify: the `world_query` from step 2 no longer lists `G`
- `mcp__brp__brp_extras_send_gamepad_button`, `mcp__brp__brp_extras_set_gamepad_button`, and `mcp__brp__brp_extras_set_gamepad_axis` on `G` each fail with an error containing `not a connected simulated gamepad`

## Expected Results
- A simulated gamepad becomes a `Gamepad` the app sees
- Button presses, releases and axis changes arrive as Bevy gamepad messages (verified via `GamepadInputHistory`)
- A sent button taps for 100 ms by default; a set button stays down until changed
- A disconnected pad rejects input
- Invalid inputs return errors

## Failure Criteria
STOP if: Any gamepad method fails unexpectedly, `GamepadInputHistory` doesn't reflect the input, a timed button never releases, or invalid inputs are accepted.
