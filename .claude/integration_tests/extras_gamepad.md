# BRP Extras Gamepad Tests

## Objective
Validate the brp_extras simulated gamepad methods: connect, button hold and release, timed release, axis, disconnect, and error handling. Verify that the input actually reaches the Bevy app by reading the `GamepadInputHistory` resource, which the app fills from Bevy's processed gamepad messages.

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
- `mcp__brp__brp_extras_send_gamepad_button` with `{"gamepad": G, "button": "South"}`
- Verify: `mcp__brp__world_get_resources` with resource `extras_plugin::GamepadInputHistory`
  - `pressed_buttons` should be `["South"]` (no `duration_ms`, so it stays down)
- Send `{"gamepad": G, "button": "South", "value": 0.0}`
- Verify: `pressed_buttons` is `[]` and `last_released` is `"South"`

### 4. Timed Release
- Send `{"gamepad": G, "button": "East", "duration_ms": 300}`
- Verify immediately: `pressed_buttons` contains `"East"`
- Wait at least 1 second, then verify: `pressed_buttons` is `[]` and `last_released` is `"East"`

### 5. Axis
- `mcp__brp__brp_extras_set_gamepad_axis` with `{"gamepad": G, "axis": "LeftStickX", "value": -0.75}`
- Verify: `last_axis` is `"LeftStickX"` and `last_axis_value` is between -0.8 and -0.7
  - Bevy's default dead zone rescales the raw value, so -0.75 arrives as about -0.737

### 6. Error Conditions (no resource verification needed)
- Excessive duration: `{"gamepad": G, "button": "South", "duration_ms": 70000}` should fail
- Unknown button: `{"gamepad": G, "button": "Bogus"}` should fail and list the valid names
- Out-of-range axis: `{"gamepad": G, "axis": "LeftStickX", "value": 1.5}` should fail
- Not a simulated gamepad: `{"gamepad": 1, "button": "South"}` should fail naming entity `1`

### 7. Disconnect
- `mcp__brp__brp_extras_disconnect_gamepad` with `{"gamepad": G}`
- Verify: the `world_query` from step 2 no longer lists `G`

## Expected Results
- A simulated gamepad becomes a `Gamepad` the app sees
- Button presses, releases and axis changes arrive as Bevy gamepad messages (verified via `GamepadInputHistory`)
- A button without `duration_ms` stays down; one with it is released on its own
- Invalid inputs return errors

## Failure Criteria
STOP if: Any gamepad method fails unexpectedly, `GamepadInputHistory` doesn't reflect the input, a timed button never releases, or invalid inputs are accepted.
