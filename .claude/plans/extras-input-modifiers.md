# Input modifiers and no cursor warp in bevy_brp_extras

> **Status: IMPLEMENTATION PLAN — phased, direct-ready.** Click and drag take keyboard modifiers, and simulated pointer moves no longer move the OS cursor.

> **As-built disposition: create** — `docs/as-built/input-modifiers.md`

## Why

Hana's C105 agent run (2026-11-15) found two gaps an agent has that a user does not:

1. A Shift+click needs two BRP calls sent together: `send_keys` ShiftLeft held 1.5 s, and `click_mouse`. A drag with a modifier held cannot be timed at all.
2. Each `move_mouse` and each drag frame logs `ERROR bevy_winit::system: could not set cursor position: ... cursor position can be set only for locked cursor.` on Wayland. Hover still works.

Cause of gap 2: `support::send_motion_events` (`extras/src/mouse/support.rs:98`) and `drag::process_drag_operations` (`extras/src/mouse/drag.rs:184`, `:217`) call `Window::set_cursor_position`. That marks `Window` changed. bevy_winit 0.20.1 `changed_windows` (`system.rs:426`) then sees `physical_cursor_position` differ from its `CachedWindow` and calls `winit_window.set_cursor_position`, which warps the OS cursor. Wayland refuses a warp unless the cursor is locked. Real winit moves do not warp, because bevy_winit refreshes `CachedWindow` after its own events (`state.rs:449`).

The user approved changes to bevy_brp_extras with the full SDLC (2026-11-15). Publishing is the user's step.

## Delegation Context

- **Project:** bevy_brp workspace — `extras` (crate `bevy_brp_extras`, a Bevy plugin with extra BRP methods) and `mcp` (crate `bevy_brp_mcp`, the MCP server that exposes them as tools).
- **Stack:** Rust 2024, Bevy 0.20.1, bevy_remote 0.20.1, serde, schemars; `mcp_macros` derives `ParamStruct` and `ResultStruct`.
- **Layout:**
  - `extras/src/mouse/` — click, drag, cursor move, button release, shared support, tests in `mod.rs`.
  - `extras/src/keyboard/` — key events (`events.rs`), key code parsing (`key_code.rs`), timed key release (`keys.rs`).
  - `mcp/src/brp_tools/mouse.rs` — `MouseButtonWrapper`.
  - `mcp/src/brp_tools/tools/brp_extras_click_mouse.rs`, `brp_extras_drag_mouse.rs` — tool params.
  - `mcp/help_text/brp_extras_click_mouse.txt`, `brp_extras_drag_mouse.txt`, `brp_extras_move_mouse.txt` — tool descriptions.
  - `.claude/integration_tests/mouse.md` — agentic mouse test.
- **Key files:**
  - `extras/src/mouse/click.rs` — `ClickMouseRequest`, `click_mouse_handler`.
  - `extras/src/mouse/drag.rs` — `DragMouseRequest`, `DragOperation`, `DragState`, `process_drag_operations`.
  - `extras/src/mouse/button.rs` — `TimedButtonRelease`, `process_timed_button_releases`.
  - `extras/src/mouse/support.rs` — `send_timed_button_press`, `send_motion_events`, `resolve_window`.
  - `extras/src/mouse/cursor.rs` — `move_mouse_handler`, `SimulatedCursorPosition`.
  - `extras/src/mouse/mod.rs` — `MousePlugin` systems.
  - `extras/src/mouse/tests.rs` — the mouse unit tests (Phase 1 moves them here from `mod.rs`).
  - `extras/src/keyboard/events.rs` — `create_keyboard_events` (builds a `KeyboardInput` for a `KeyCode`).
  - `extras/src/window_event.rs` — `write_input_event` (writes the message and its `WindowEvent`).
  - `extras/CHANGELOG.md`, `mcp/CHANGELOG.md`, `extras/README.md`.
- **Test lanes:** `extras` — unit tests in `extras/src/mouse/tests.rs` (`#[cfg(test)]`), and `extras/tests/`; `mcp` — unit tests beside the tool files.
- **Build:** `bash ~/.claude/scripts/direct/verify.sh check bevy_brp_extras` and `… check bevy_brp_mcp`
- **Test:** `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_extras` and `… test bevy_brp_mcp`
- **Lint:** `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_extras` and `… lint bevy_brp_mcp`
- **Style:** run-end /clippy style-only auto-proceed
- **Invariants:**
  - Existing requests without `modifiers` behave exactly as now. The field is optional on the wire.
  - Timed holds run on `Time<Real>`, as the 0.23.0 fix requires.
  - `window.cursor_position()` returns the simulated position after a simulated move, focused or not.
  - No commit on `main`. No crate publish.

## Phases

### Phase 1 — Click and drag hold keyboard modifiers  · status: done

#### As-built

- `brp_extras/click_mouse` and `brp_extras/drag_mouse` take an optional `modifiers` list (`Shift`, `Control`, `Alt`, `Super`). The modifiers are pressed before the button, held for every drag frame, and released one frame after the button. Requests without `modifiers` behave as before.
- `Modifier` (`extras/src/mouse/modifier.rs`) derives `Hash`; `Modifier::key() -> KeyCodeWrapper` gives the left key. `deserialize_modifiers` drops duplicates, keeps first-occurrence order, and gives INVALID_PARAMS that names `modifiers` for an unknown name.
- `HeldModifiers` counts holders, so overlapping requests do not release a modifier early. `ModifierEvents` writes keyboard events directly. `PendingModifierRelease` is processed in `process_pending_modifier_releases`, which runs before `button::process_timed_button_releases`.
- The `DragInput` SystemParam: `button()` and `motion()` write the events and the `Window` cursor position in one system, so motion deltas match cursor positions in both schedule orders. `DragState::ReleaseModifiers` ends the drag.
- Responses echo `modifiers` only when the list is not empty.

**Files:**
- `extras/src/mouse/modifier.rs` — modifier type, holder counts, modifier events, pending releases.
- `extras/src/mouse/drag.rs` — `DragInput`, drag states, modifiers on drag.
- `extras/src/mouse/button.rs` — `TimedButtonRelease.modifiers`; spawns `PendingModifierRelease`.
- `extras/src/mouse/support.rs` — `send_timed_button_press(..., modifiers)`.
- `extras/src/mouse/click.rs` — request and response `modifiers`.
- `extras/src/mouse/mod.rs` — `HeldModifiers` resource, system order, `mod tests`.
- `extras/src/mouse/tests.rs` — the mouse unit tests (moved from `mod.rs`) and the modifier, overlap and motion-order tests.
- `extras/src/keyboard/{events.rs,key_code.rs,mod.rs}` — `create_keyboard_events` and `KeyCodeWrapper` are `pub(crate)`.

**Gotchas:** Drag events are written directly in the same system as the cursor write; deferred writes reorder them against `sync_cursor_position`. `ButtonInput<KeyCode>` has no window, so holder counts must be per modifier, not per window.

**Ruled out:** Converting `ClickMouseResult.result` / `DragMouseResult.result` `Option<Value>` into a semantic acknowledgement — the `ResultStruct` macro shapes every tool's result.

### Phase 2 — Simulated moves do not warp the OS cursor, and a drag starts without a jump  · status: done

#### Work Order

**Goal:** simulated cursor writes in `move_mouse` and `drag_mouse` do not mark `Window` changed, so bevy_winit does not warp the OS cursor on each move and Wayland logs no `could not set cursor position` error per move. Another `Window` change can still cause the one documented warp (Known limit below). A drag moves the cursor to `start` on a frame before its button press, so the jump from the old cursor point is never motion with the button held. A modifier that two overlapping requests hold in different windows stays held until the last request releases it.

**Spec:**
- Add `support::record_cursor_position(mut window: Mut<'_, Window>, position: Vec2)`. It calls `window.bypass_change_detection().set_cursor_position(Some(position))`. Both callers have a `Mut<Window>`: `Query<&mut Window>::get_mut` and `World::get_mut::<Window>`.
- Use it at the two call sites: `support.rs:105` (`send_motion_events`) and `drag.rs:190` (`DragInput::motion`).
- Keep the writes in `DragInput::motion` together: `MouseMotion`, `CursorMoved` and the `Window` position, in one system, with no deferred flush between them.
- Rewrite the doc comment on `send_motion_events`: the position is written for `window.cursor_position()` readers (OrbitCam, UI hit tests), and change detection is bypassed so bevy_winit does not warp the OS cursor.
- Known limit, for the doc comment and the as-built: bevy_winit's `CachedWindow` keeps the old cursor position. When some other `Window` field changes later, bevy_winit may warp once to the last simulated position. On Wayland that is one error line for that change, not one per move.
- Drag start (showrunner add-on, 2026-11-16): add `DragState::PositionStart` before `Pressed`. In `PositionStart`, `DragInput::motion` moves the cursor to `start` (the `CursorMoved` and the `MouseMotion` with the jump delta from the current cursor). No modifier and no button is held during this move. In the next Update, `Pressed` sends the modifier presses, then the button press. Measured in Hana on 2026-11-16: a middle-drag from (900,500) with the cursor at (492,348) turned the orbit by 708 px and 152 px of motion, not by the 300 px drag.
- Modifier holders across windows: `ButtonInput<KeyCode>` has no window, so a per-window count lets one window's release clear a modifier that another window still holds. Change `HeldModifiers` (`modifier.rs:72`) to count holders per `Modifier`, not per `(Entity, Modifier)`. Send a press when a count goes from 0 to 1 and a release when it goes from 1 to 0, in the window of the request that changes the count.
- `DragOperation.window` (`drag.rs:93`): store the resolved `Entity`, not `Option<Entity>`. `drag_mouse_handler` already resolves it (`drag.rs:123`). Remove the `resolve_window_entity` fallback in `process_drag_operations` (`drag.rs:205`). The optional window id stays only on the wire (`DragMouseRequest.window`).
- Tests in `extras/src/mouse/tests.rs`. Clear the setup change ticks before each measurement:
  1. `move_mouse_does_not_mark_the_window_changed` — after `move_mouse`, `cursor_position()` is the new position, and `Ref<Window>::is_changed()` is false after the tick that ran the handler. The cursor write keeps existing change flags and adds no new one.
  2. `drag_frames_do_not_mark_the_window_changed` — the same for every frame of a 3-frame drag.
  3. `cursor_position_follows_the_drag` — `cursor_position()` equals each interpolated drag point. This is regression cover: it can pass before the change.
  4. `drag_start_jump_is_not_motion_while_the_button_is_held` — with the cursor at one point and a drag that starts at another, no `MouseMotion` that carries the jump is written on or after the button-press frame; the motion summed while the button is held equals `end - start`.
  5. `modifier_stays_held_until_the_last_window_releases_it` — an overlapping Control click in one window and Control drag in another. Read `ButtonInput<KeyCode>` in `PreUpdate` after `InputSystems`. Control stays pressed until the last holder releases it.
- Update the existing tests for the new state:
  - `drag_with_control_holds_control_for_every_drag_frame`: Control is held from the button-press frame through the button-release frame. Test the `PositionStart` move separately: no modifier is held on it.
  - Raise `DRAG_COMPLETION_OVERHEAD_FRAMES` (`tests.rs:47`) by one for `PositionStart`. Keep the overlapping-click assertions and the final modifier release check.

**Files:**
- `extras/src/mouse/support.rs` — `record_cursor_position`, call site, doc comment.
- `extras/src/mouse/drag.rs` — the call site, `DragState::PositionStart`, `DragOperation.window: Entity`.
- `extras/src/mouse/modifier.rs` — `HeldModifiers` counts per `Modifier`.
- `extras/src/mouse/tests.rs` — tests.

**Workers:** 1 writer + 1 tester — the same split as Phase 1.
- `impl` — `support.rs`, `drag.rs`, `modifier.rs`; hub: none.
- `test` — `tests.rs`: the five tests and the updates to the existing drag, schedule-order and modifier tests.

**State:** none

**Constraints from prior phases:**
- `DragInput` (`drag.rs:155`) is a `SystemParam`. `DragInput::motion` (`drag.rs:174-190`) writes `MouseMotion`, `CursorMoved` and the `Window` position directly, in one system. `DragInput::button` writes the button event.
- `drag_motion_deltas_match_cursor_positions_in_both_system_orders` must keep passing in both schedule orders. Add no deferred flush.
- `DragState` is `Pressed`, `Dragging`, `Released`, `ReleaseModifiers`. `DragOperation` has `modifiers`.
- `modifier.rs`: `HeldModifiers` with `HeldModifiers::update`; `ModifierEvents` (press, release, send) writes keyboard events directly; `PendingModifierRelease` runs in `process_pending_modifier_releases`, ordered before `button::process_timed_button_releases`.
- The tests use a paused-clock helper that registers `MousePlugin` and `InputPlugin`. Observers read input in `PreUpdate` after `InputSystems`.

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_extras` — record each baseline result. Tests 1, 2, 4 and 5 fail before the change; test 3 is regression cover. After the change all five pass, and every Phase 1 modifier, overlap, paused-clock and motion test passes.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_extras` — clean.
- Live check (unit director): launch `test-app` example `extras_plugin` (debug) on natedev Wayland on a free port, send 20 `brp_extras/move_mouse` and one `brp_extras/drag_mouse` by curl, and confirm the log has no `could not set cursor position` line. Before the change the same steps log one line per move.
- Platform check (unit director, showrunner's ask 2026-11-15): with the OS cursor no longer warped, hover and a jack drag in Hana still work on Wayland, X11 (`env -u WAYLAND_DISPLAY`) and macOS. Use a Hana built against this branch through a local `[patch.crates-io]` in a copies tree. Name any platform that cannot be tested.

### Phase 3 — MCP tools, docs and change notes  · status: todo

#### Work Order

**Goal:** the `brp_extras_click_mouse` and `brp_extras_drag_mouse` MCP tools expose `modifiers`, and the docs and change notes describe both changes.

**Spec:**
- `mcp/src/brp_tools/mouse.rs`: add `Modifier { Shift, Control, Alt, Super }` (the same name as the extras type) with `Serialize, Deserialize, JsonSchema`, doc comment per variant. It wraps no Bevy type, so it takes no `Wrapper` suffix.
- `ClickMouseParams` and `DragMouseParams`: add
  ```rust
  /// Keyboard modifiers held for the whole click or drag (Shift, Control, Alt, Super)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub modifiers: Vec<Modifier>,
  ```
  An empty list means no modifiers. Do not use `Option<Vec<_>>`. The JsonSchema lists the four names.
- Help text: add a modifier example to `brp_extras_click_mouse.txt` (`{"button": "Left", "modifiers": ["Shift"]}` — Shift+click to add to a selection) and to `brp_extras_drag_mouse.txt`. Say that the modifier keys are pressed with the button and released one frame after it. In `brp_extras_drag_mouse.txt`, also say that a drag first moves to `start` on its own frame, with no button or modifier held. In `brp_extras_move_mouse.txt`, say that a move does not move the OS cursor.
- Tests beside the tool files:
  - Params with `modifiers` serialize to the extras wire form. Params without it leave the field out.
  - Both MCP results keep the modifier echo from the extras response. Duplicate modifiers keep the extras first-occurrence order.
  - The MCP params reject an unknown name (`Hyper`) with an error that names the four valid values.
- Create `.claude/integration_tests/input_modifiers.md` for the `extras_plugin` example (`mouse.md` needs the `mouse_test` example, so it is not the place). Steps: Shift+click, Control+drag, the keyboard messages the example logs, the response echoes, and a direct BRP call with `Hyper` that returns INVALID_PARAMS naming `modifiers` and `Hyper`.
- `.claude/scripts/integration_tests/mouse_test.sh` (showrunner add-on, 2026-11-16): the script reads flat response fields, but the `mouse_test` example nests them, so 58 checks fail on the Phase 1 tree. Make the script read the nested fields. Done when `mouse_test.sh <port>` passes against the `mouse_test` example.
- `extras/CHANGELOG.md` and `mcp/CHANGELOG.md` under `[Unreleased]`: `### Added` modifiers on click and drag; `### Fixed` the Wayland cursor error on simulated moves.
- `extras/README.md` mouse section: one line for `modifiers`.
- `docs/as-built/input-modifiers.md`: written by the as-built step at the end of the run.

**Files:**
- `mcp/src/brp_tools/mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_click_mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_drag_mouse.rs`
- `mcp/help_text/brp_extras_click_mouse.txt`, `brp_extras_drag_mouse.txt`, `brp_extras_move_mouse.txt`
- `.claude/integration_tests/input_modifiers.md`
- `.claude/scripts/integration_tests/mouse_test.sh`
- `extras/CHANGELOG.md`, `mcp/CHANGELOG.md`, `extras/README.md`

**Workers:** 2 writers — code and docs split by file.
- `impl` — the three `mcp/src` files, the `Modifier` type and the tool tests; hub: `mcp/src/brp_tools/mouse.rs`.
- `test` — opens as impl: the help text, `input_modifiers.md`, `mouse_test.sh`, change notes and README.

**State:** none

**Constraints from prior phases:**
- The extras wire field is `modifiers: [String]` with values `Shift`, `Control`, `Alt`, `Super`. Unknown values are an INVALID_PARAMS error that names `modifiers`. The extras drop duplicates and keep first-occurrence order. Responses echo `modifiers` only when it is not empty.
- The MCP result types carry the raw extras response, so the echo needs a test, not a new field.
- Simulated moves bypass `Window` change detection (Phase 2).
- A drag moves to `start` on its own frame first, with no button or modifier held. Modifiers are pressed with the button and released one frame after it (Phase 2). A modifier held in two windows stays held until the last holder releases it.

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_mcp` — the new tests pass.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_mcp` — clean.
- Unit director: `mouse_test.sh <port>` passes against the `mouse_test` example on a free port.
- After the user reloads the MCP server: run `input_modifiers.md`, and record the app, port, requests, responses and keyboard log lines.

## After the phases

1. Run the project style review, then lint and tests for both crates.
2. Push the branch with /validate_and_push. Never push to `main`.
3. Report to the showrunner. The release and the crate publish are the user's.
4. After the release: a Hana change moves `bevy_brp_extras` from 0.23.0 to the new version, and the C105 how-to drops the two-call Shift+click.
