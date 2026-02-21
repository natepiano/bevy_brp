# Mouse Input Integration Tests

## Objective
Validate comprehensive mouse input simulation on both primary and secondary windows, ensuring all operations (cursor movement, button presses, scrolling, gestures, picking) are window-specific and independent.

## Setup Requirements

- **App**: `mouse_test` example
- **Profile**: debug
- **Plugin**: bevy_brp_extras required
- **Resource**: `mouse_test::MouseStateTracker`

## Execution

This test is executed via a bash script that communicates directly with the BRP JSON-RPC endpoint using `curl`. The script runs all 9 scenarios sequentially and reports pass/fail for each assertion.

**Run the script**:
```bash
bash .claude/scripts/integration_tests/mouse_input_test.sh <PORT>
```

**Interpret results**:
- Exit code `0` = all tests passed
- Exit code `1` = one or more failures
- `[PASS]` lines show successful assertions with actual values
- `[FAIL]` lines show failures with actual vs expected values
- `[SKIP]` lines indicate platform-skipped tests (e.g., gestures on non-macOS)

## Test Scenarios

### 1. Window Discovery
Identify primary and secondary window entities via `world.query`.

### 2. Cursor Movement
Test absolute and delta cursor positioning on both windows independently.
- Primary: move to (200,150), delta (+50,+30) = (250,180)
- Secondary: move to (100,100), delta (+20,+10) = (120,110)

### 3. Mouse Buttons
Test all 5 button types (Left, Right, Middle, Back, Forward) via `send_mouse_button`.
Verify timestamps update on press and all buttons release correctly.

### 4. Scroll Operations
Test Line and Pixel scroll units on both windows.
- Primary: 3.0 X lines, 5.0 Y lines + 100.0 Y pixels = (3.0, 105.0)
- Secondary: -2.0 X lines, 4.0 Y lines + 50.0 Y pixels = (-2.0, 54.0)

### 5. Gestures (macOS only)
Test pinch, rotation, and double-tap gestures on both windows.
- Primary: pinch 2.5 + (-1.0) = 1.5, rotation 0.5
- Secondary: pinch 3.0 + 0.5 = 3.5, rotation 0.3
- Skipped on non-macOS platforms.

### 6. Click Operations
Test single click and double-click with position tracking on both windows.
- Uses `click_mouse` and `double_click_mouse` methods.
- Verifies click/doubleclick positions and timestamps per window.

### 7. Drag Operations
Test drag interpolation on both windows.
- Primary: drag (100,100) to (300,200) over 20 frames
- Secondary: drag (50,50) to (150,150) over 20 frames

### 8. Picking Validation
Verify simulated mouse events flow through Bevy's picking system.
- Click cuboid center (300,200) to trigger picking observer
- Verify click count increments and gizmo activates
- Double-click cuboid for double-click detection
- Click background (50,50) to deselect
- Tests both primary and secondary windows independently

### 9. Final Verification
Comprehensive state check: all buttons released, picking counts > 0 on both windows,
gizmos deselected, scroll totals correct.

## Success Criteria

- All cursor movements work independently on both windows
- All scroll operations are window-specific
- All gestures are window-specific (attributed via cursor position)
- Button states are global but work from either window
- No cross-contamination between windows
- Drag operations work on both windows independently
- Picking system responds to simulated input on both windows
- Gizmo outlines appear/disappear correctly based on cuboid/background clicks
- Gesture tests pass on macOS (skipped on other platforms)
- No app crashes or hangs
- Script completes in < 30 seconds

## Notes

- **Window Specificity**: Cursor position, scroll, gestures, clicks, and double-clicks are per-window
- **Global State**: Button presses are global (not per-window)
- **Click Detection**: Clicks detected on button release; position captured from cursor at time of click
- **Double-Click Detection**: Two clicks within 400ms trigger double-click
- **Platform-specific**: Gesture tests are macOS-specific
- **Cursor Window**: Gestures use `cursor_window` to determine target since gesture events lack window fields
