# Mouse Input Integration Tests (Optimized)

## Objective
Validate comprehensive mouse input simulation on both primary and secondary windows, ensuring all operations (cursor movement, button presses, scrolling, gestures) are window-specific and independent.

## Setup Requirements

- **App**: `mouse_test` example
- **Profile**: debug
- **Plugin**: bevy_brp_extras required
- **Resource**: `mouse_test::MouseStateTracker`

## Test Strategy

All mouse operations must be tested on **both windows** to verify:
1. Operations work correctly on each window
2. Each window maintains independent state
3. Actions on one window don't affect the other window's state

**Optimization Strategy**: Batch related operations and verify once at checkpoints rather than after every individual operation.

## Test Scenarios

### 1. Window Discovery

**Description**: Identify primary and secondary window entities

**Steps**:
1. Query all windows using `world.query` with filter for `bevy_window::window::Window`
2. Identify primary window (has `bevy_window::window::PrimaryWindow` component)
3. Identify secondary window (has `mouse_test::SecondaryWindow` component)
4. Store both entity IDs for use in subsequent tests

**Expected**:
- Two windows found
- Primary and secondary windows have different entity IDs
- Both windows have `Window` component

### 2. Basic Cursor Movement - Both Windows

**Description**: Test cursor movement on both windows with absolute and delta positioning

**Steps**:
1. Move cursor to primary window position (200, 150)
2. Move cursor to secondary window position (100, 100)
3. Move cursor in primary by delta (+50, +30)
4. Move cursor in secondary by delta (+20, +10)
5. Query `MouseStateTracker` ONCE to verify:
   - `primary_window_position` is (250, 180) - from (200,150) + delta (50,30)
   - `secondary_window_position` is (120, 110) - from (100,100) + delta (20,10)
   - `cursor_window` matches secondary window entity (last moved)

**Expected**:
- Both windows track positions independently
- Delta movements work correctly
- Cursor window tracking shows last moved window

### 3. Mouse Buttons - Both Windows

**Description**: Test all 5 mouse buttons on both windows

**Steps**:
1. Move cursor to primary window (150, 150)
2. Send all 5 button presses sequentially with 100ms duration each:
   - Left button press
   - Right button press
   - Middle button press
   - Back button press
   - Forward button press
3. Query tracker to verify all 5 button timestamps are non-zero
4. Wait 150ms for all buttons to release
5. Move cursor to secondary window (80, 80)
6. Send Left button press with 100ms duration
7. Wait 150ms
8. Query tracker ONCE to verify:
   - All buttons work (timestamps updated in step 3)
   - Button states are global (work from either window)
   - Latest press was on secondary window

**Expected**:
- All 5 buttons work correctly
- Button states are global (not per-window)
- Timestamps update on press

### 4. Scroll Operations - Both Windows

**Description**: Test scrolling with line and pixel units on both windows

**Steps**:
1. Move cursor to primary window
2. Scroll Y by 5.0 lines
3. Scroll X by 3.0 lines
4. Move cursor to secondary window
5. Scroll Y by 4.0 lines
6. Scroll X by -2.0 lines
7. Move cursor to primary window
8. Scroll Y by 100.0 pixels
9. Move cursor to secondary window
10. Scroll Y by 50.0 pixels
11. Query tracker ONCE to verify:
    - `primary_scroll_x_total` = 3.0
    - `primary_scroll_y_total` = 105.0 (5 lines + 100 pixels)
    - `primary_scroll_unit` = "Pixel" (last unit used)
    - `secondary_scroll_x_total` = -2.0
    - `secondary_scroll_y_total` = 54.0 (4 lines + 50 pixels)
    - `secondary_scroll_unit` = "Pixel" (last unit used)

**Expected**:
- Scroll accumulates correctly on both windows
- Each window maintains independent scroll state
- Both line and pixel units work correctly

### 5. Gestures - Both Windows (macOS)

**Description**: Test pinch, rotation, and double-tap gestures on both windows

**Steps**:
1. Move cursor to primary window
2. Send pinch gesture with delta 2.5
3. Send pinch gesture with delta -1.0
4. Send rotation gesture with delta 0.5
5. Send double tap gesture
6. Move cursor to secondary window
7. Send pinch gesture with delta 3.0
8. Send pinch gesture with delta 0.5
9. Send rotation gesture with delta 0.3
10. Send double tap gesture
11. Query tracker ONCE to verify:
    - `primary_pinch_total` = 1.5 (2.5 - 1.0)
    - `primary_rotation_total` = 0.5
    - `primary_double_tap_timestamp` > 0
    - `secondary_pinch_total` = 3.5 (3.0 + 0.5)
    - `secondary_rotation_total` = 0.3
    - `secondary_double_tap_timestamp` > 0
    - All gestures independent per window

**Expected**:
- Gestures accumulate correctly
- Each window maintains independent gesture state
- Cursor window determines gesture target

**Note**: This test may skip on non-macOS platforms

### 6. Click Operations - Both Windows

**Description**: Test single and double clicks with position tracking using both `click_mouse` and `double_click_mouse` methods

**Steps**:
1. Move cursor to primary window (200, 200)
2. Use `click_mouse` to click left button (default 100ms duration)
3. Wait 600ms (for timestamps to age out)
4. Move cursor to primary window (250, 175)
5. Use `double_click_mouse` with left button and 100ms delay between clicks
6. Wait 150ms for completion
7. Move cursor to secondary window (100, 100)
8. Use `click_mouse` to click left button (default 100ms duration)
9. Wait 600ms
10. Move cursor to secondary window (150, 120)
11. Use `double_click_mouse` with left button and 100ms delay
12. Wait 150ms for completion
13. Query tracker ONCE to verify:
    - `primary_click_position` = (250, 175) - last click on primary
    - `primary_doubleclick_position` = (250, 175)
    - `secondary_click_position` = (150, 120) - last click on secondary
    - `secondary_doubleclick_position` = (150, 120)
    - Both click and doubleclick timestamps are recent
    - Click positions match cursor positions at time of click

**Expected**:
- Clicks detected on button release
- Click positions captured correctly
- Double-clicks detected (two clicks within 400ms)
- Click tracking is per-window independent

### 7. Drag Operations - Both Windows

**Description**: Test drag operations on both windows

**Steps**:
1. Drag on primary window with left button from (100, 100) to (300, 200) over 20 frames
2. Drag on secondary window with left button from (50, 50) to (150, 150) over 20 frames
3. Query tracker ONCE to verify:
   - `primary_window_position` = (300, 200)
   - `secondary_window_position` = (150, 150)
   - Both windows show final drag positions

**Expected**:
- Smooth interpolation during drags
- Each window maintains independent cursor position
- Final positions match drag endpoints

### 8. Picking Validation - Both Windows

**Description**: Verify simulated mouse events flow through Bevy's picking system via the `WindowEvent` channel, triggering observers on pickable 3D cuboid meshes

**Steps**:
1. Move cursor to primary window center (300, 200) — over the cuboid
2. Use `click_mouse` with left button
3. Wait 150ms for release
4. Query `MouseStateTracker` to verify:
   - `primary_picking_click_count` = 1
   - `primary_picking_gizmo_active` = true
5. Wait 600ms (age out double-click window so next click starts fresh)
6. Move cursor to primary window center (300, 200) again
7. Use `double_click_mouse` with left button and 100ms delay
8. Wait 250ms for completion
9. Query `MouseStateTracker` to verify:
   - `primary_picking_doubleclick_count` >= 1
   - `primary_picking_gizmo_active` = true (yellow gizmo now)
10. Move cursor away from cuboid on primary window (50, 50) — hits background plane
11. Use `click_mouse` with left button
12. Wait 150ms
13. Query `MouseStateTracker` to verify:
    - `primary_picking_gizmo_active` = false (deselected via background click)
14. Move cursor to secondary window center (300, 200) — over the cuboid
15. Use `click_mouse` with left button
16. Wait 150ms
17. Query `MouseStateTracker` to verify:
    - `secondary_picking_click_count` = 1
    - `secondary_picking_gizmo_active` = true
18. Wait 600ms (age out double-click window)
19. Move cursor to secondary window center (300, 200) again
20. Use `double_click_mouse` with left button and 100ms delay
21. Wait 250ms
22. Query `MouseStateTracker` to verify:
    - `secondary_picking_doubleclick_count` >= 1
    - `secondary_picking_gizmo_active` = true
23. Move cursor away from cuboid on secondary window (50, 50) — hits background
24. Use `click_mouse` with left button
25. Wait 150ms
26. Query `MouseStateTracker` to verify:
    - `secondary_picking_gizmo_active` = false

**Expected**:
- Simulated mouse events propagate through `WindowEvent` channel to picking system
- Observer-based click detection works on cuboid meshes
- Single click produces green gizmo outline
- Double click produces yellow gizmo outline
- Clicking background plane removes gizmo (deselection)
- Both windows have independent picking state
- `primary_picking_click_count` and `secondary_picking_click_count` update independently

### 9. Complete State Verification

**Description**: Final comprehensive state check

**Steps**:
1. Query complete `MouseStateTracker` resource
2. Verify all fields are present and have expected types:
   - Window positions (Vec2) match test operations
   - Scroll totals (f32) reflect accumulated values
   - Gesture totals (f32) reflect accumulated values
   - Click/doubleclick positions (Vec2) match last operations
   - Timestamps (f32) are recent for recent operations
   - Button states (bool) all false (released)
   - `cursor_window` matches last moved window
   - Picking fields (both windows):
     - `primary_picking_click_count` (u32) > 0
     - `primary_picking_doubleclick_count` (u32) > 0
     - `primary_picking_gizmo_active` (bool) = false (deselected in test)
     - `secondary_picking_click_count` (u32) > 0
     - `secondary_picking_doubleclick_count` (u32) > 0
     - `secondary_picking_gizmo_active` (bool) = false (deselected in test)

**Expected**:
- All fields present and correctly typed
- Values reflect all operations performed
- Picking state reflects final deselected state on both windows
- No unexpected state changes

## Success Criteria

- All cursor movements work independently on both windows
- All scroll operations are window-specific
- All gestures are window-specific (attributed via cursor position)
- Button states are global but work from either window
- No cross-contamination between windows
- Drag operations work on both windows independently
- Picking system responds to simulated input on both windows
- Gizmo outlines appear/disappear correctly based on cuboid/background clicks
- Gesture tests pass on macOS (may skip on other platforms)
- No app crashes or hangs

## Optimization Notes

**Tool Call Reduction**:
- Original test: ~109 tool calls
- Optimized test: ~50-55 tool calls (including picking validation)

**Key Optimizations**:
1. **Batched operations**: Group related operations before verifying
2. **Single verification points**: Query tracker once per test section instead of after each operation
3. **Removed redundant checks**: Trust tool success responses instead of immediate re-verification
4. **Combined test scenarios**: Merged related tests (e.g., cursor movement + delta in one test)

**Trade-offs**:
- Slightly less granular failure isolation (can't tell which specific operation in a batch failed)
- Still validates all functionality comprehensively
- Much faster execution and lower token usage
- Easier to maintain and understand

## Notes

- **Window Specificity**: Cursor position, scroll, gestures, clicks, and double-clicks are per-window
- **Global State**: Button presses are global (not per-window)
- **Click Detection**: Clicks detected on button release; position captured from cursor at time of click
- **Double-Click Detection**: Two clicks within 400ms trigger double-click; both events recorded with positions
- **Platform-specific**: Gesture tests are macOS-specific
- **Timing**: Allow small tolerance (±50ms) for duration-based tests
- **Cursor Window**: Gestures use `cursor_window` to determine target since gesture events lack window fields
- **Scroll Events**: Scroll uses `event.window` field directly
