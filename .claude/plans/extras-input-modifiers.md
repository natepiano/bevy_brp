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

#### Work Order

**Goal:** `brp_extras/click_mouse` and `brp_extras/drag_mouse` accept `modifiers`, and the app sees each modifier key held for the whole click or drag.

**Spec:**
- New type in `extras/src/mouse/modifier.rs`:
  ```rust
  /// A keyboard modifier held for a whole click or drag.
  #[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq)]
  pub(super) enum Modifier { Shift, Control, Alt, Super }
  impl Modifier {
      /// The key the app sees pressed. Left keys, as a user most often presses them.
      pub(super) const fn key_code(self) -> KeyCode { /* ShiftLeft, ControlLeft, AltLeft, SuperLeft */ }
  }
  ```
- `ClickMouseRequest` and `DragMouseRequest` get `#[serde(default)] modifiers: Vec<Modifier>`. Remove duplicates, keep the first order. The responses echo `modifiers`, and leave the field out when it is empty.
- Press order: on the frame the click or drag button is pressed, write a `KeyboardInput` Pressed for each modifier before the `MouseButtonInput` Pressed. Use `keyboard::events::create_keyboard_events` (make it `pub(crate)`) and `window_event::write_input_event`, with the click's or drag's window entity.
- Release order: release the mouse button first. Release the modifiers on the next frame, so an app that acts on the button release still sees the modifier held.
  - Click: `TimedButtonRelease` gets `modifiers: Vec<Modifier>`. When its timer ends it releases the button and, if `modifiers` is not empty, spawns a one-frame `PendingModifierRelease { modifiers, window }` that the next `Update` releases and despawns.
  - Drag: add `DragState::ReleaseModifiers` after `Released`. `Released` releases the button and moves to `ReleaseModifiers` when `modifiers` is not empty; `ReleaseModifiers` releases the keys and despawns.
- `send_mouse_button` and `double_click_mouse` do not change (no consumer).
- Fail-first tests in `extras/src/mouse/tests.rs`, written before the code and failing on the old code:
  1. `click_with_shift_holds_shift_across_the_click` — the `KeyboardInput` ShiftLeft Pressed comes no later than the button press, and its Released comes on the frame after the button release.
  2. `drag_with_control_holds_control_for_every_drag_frame` — ControlLeft is pressed in `ButtonInput<KeyCode>` on each frame that the drag sends a `CursorMoved`, and released after the button release.
  3. `click_without_modifiers_sends_no_keyboard_input` — the old wire request gives no `KeyboardInput`.
  4. `duplicate_modifiers_press_once` — `["Shift","Shift"]` gives one press and one release.
  5. `unknown_modifier_is_rejected` — `["Hyper"]` gives an invalid-params error that names the field.
  6. `modifier_holds_while_the_virtual_clock_is_paused` — reuse `app_with_paused_virtual_clock`.

**Files:**
- `extras/src/mouse/modifier.rs` — new: `Modifier`, `PendingModifierRelease`, the press and release helpers.
- `extras/src/mouse/mod.rs` — `mod modifier;`, register the release system, and replace the inline `#[cfg(test)] mod tests { … }` block with `#[cfg(test)] mod tests;`.
- `extras/src/mouse/tests.rs` — new: the existing mouse tests, moved from `mod.rs` unchanged, and the six new tests.
- `extras/src/mouse/click.rs` — request and response field, press order.
- `extras/src/mouse/drag.rs` — request field, `DragOperation.modifiers`, `ReleaseModifiers` state.
- `extras/src/mouse/button.rs` — `TimedButtonRelease.modifiers`.
- `extras/src/mouse/support.rs` — `send_timed_button_press` takes the modifiers.
- `extras/src/keyboard/events.rs` — `create_keyboard_events` visibility.

**Workers:** 1 writer + 1 tester — the tests move to their own file, because berth gives each file one owner.
- `impl` — every file above except `tests.rs`; hub: `extras/src/mouse/mod.rs` (module and system registration, and the `mod tests;` line).
- `test` — `extras/src/mouse/tests.rs`: the moved tests and the six new tests, from the Spec alone.

**State:** none

**Constraints from prior phases:**

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_extras` — the six new tests pass, and they failed before the change.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_extras` — clean.

### Phase 2 — Simulated moves do not warp the OS cursor, and a drag starts without a jump  · status: todo

#### Work Order

**Goal:** `move_mouse` and `drag_mouse` set `Window`'s cursor position without bevy_winit warping the OS cursor, so Wayland logs no `could not set cursor position` error. A drag moves the cursor to `start` on a frame before its button press, so the jump from the old cursor point is never motion with the button held.

**Spec:**
- Add `support::record_cursor_position(window: &mut Mut<Window>, position: Vec2)`. It calls `window.bypass_change_detection().set_cursor_position(Some(position))`.
- Use it at the three call sites: `support.rs:98` (`send_motion_events`, which takes `world.get_mut::<Window>`), `drag.rs:184` and `drag.rs:217`.
- Rewrite the doc comment on `send_motion_events`: the position is written for `window.cursor_position()` readers (OrbitCam, UI hit tests), and change detection is bypassed so bevy_winit does not warp the OS cursor.
- Known limit, for the doc comment and the as-built: bevy_winit's `CachedWindow` keeps the old cursor position. When some other `Window` field changes later, bevy_winit may warp once to the last simulated position. On Wayland that is one error line for that change, not one per move.
- Fail-first tests in `extras/src/mouse/tests.rs`:
  1. `move_mouse_does_not_mark_the_window_changed` — after `move_mouse`, `cursor_position()` is the new position, and `Ref<Window>::is_changed()` is false after the tick that ran the handler (clear change ticks first).
  2. `drag_frames_do_not_mark_the_window_changed` — the same for every frame of a 3-frame drag.
  3. `cursor_position_follows_the_drag` — `cursor_position()` equals each interpolated drag point.
  4. `drag_start_jump_is_not_motion_while_the_button_is_held` — with the cursor at one point and a drag that starts at another, no `MouseMotion` that carries the jump is written on or after the button-press frame; the motion summed while the button is held equals `end - start`.
- Drag start (showrunner add-on, 2026-11-16): the drag writes its move to `start` (the `CursorMoved` and the `MouseMotion` with the jump delta) on a frame before the `MouseButtonInput` Pressed. Measured in Hana on 2026-11-16: a middle-drag from (900,500) with the cursor at (492,348) turned the orbit by 708 px and 152 px of motion, not by the 300 px drag.

**Files:**
- `extras/src/mouse/support.rs` — `record_cursor_position`, call site, doc comment.
- `extras/src/mouse/drag.rs` — the call sites, and a move to `start` before the press (a new first `DragState`).
- `extras/src/mouse/tests.rs` — tests.

**Workers:** 1 writer + 1 tester — the same split as Phase 1.
- `impl` — `support.rs`, `drag.rs`; hub: none.
- `test` — the four tests in `extras/src/mouse/tests.rs`.

**State:** none

**Constraints from prior phases:** Phase 1 adds `DragState::ReleaseModifiers` and `DragOperation.modifiers`. The `drag.rs` call sites stay in the `Pressed` and `Dragging` arms.

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_extras` — the four new tests pass, and they failed before the change.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_extras` — clean.
- Live check (unit director): launch `test-app` example `extras_plugin` (debug) on natedev Wayland on a free port, send 20 `brp_extras/move_mouse` and one `brp_extras/drag_mouse` by curl, and confirm the log has no `could not set cursor position` line. Before the change the same steps log one line per move.
- Platform check (unit director, showrunner's ask 2026-11-15): with the OS cursor no longer warped, hover and a jack drag in Hana still work on Wayland, X11 (`env -u WAYLAND_DISPLAY`) and macOS. Use a Hana built against this branch through a local `[patch.crates-io]` in a copies tree. Name any platform that cannot be tested.

### Phase 3 — MCP tools, docs and change notes  · status: todo

#### Work Order

**Goal:** the `brp_extras_click_mouse` and `brp_extras_drag_mouse` MCP tools expose `modifiers`, and the docs and change notes describe both changes.

**Spec:**
- `mcp/src/brp_tools/mouse.rs`: add `ModifierWrapper { Shift, Control, Alt, Super }` with `Serialize, Deserialize, JsonSchema`, doc comment per variant.
- `ClickMouseParams` and `DragMouseParams`: add
  ```rust
  /// Keyboard modifiers held for the whole click or drag (Shift, Control, Alt, Super)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub modifiers: Option<Vec<ModifierWrapper>>,
  ```
- Help text: add a modifier example to `brp_extras_click_mouse.txt` (`{"button": "Left", "modifiers": ["Shift"]}` — Shift+click to add to a selection) and to `brp_extras_drag_mouse.txt`. Say that the modifier keys are pressed with the button and released one frame after it. In `brp_extras_move_mouse.txt`, say that a move does not move the OS cursor.
- Tests beside the tool files: params with `modifiers` serialize to the extras wire form; params without it leave the field out.
- `.claude/integration_tests/mouse.md`: add a Shift+click step and a Control+drag step against `extras_plugin`, checking the keyboard messages the example logs.
- `extras/CHANGELOG.md` and `mcp/CHANGELOG.md` under `[Unreleased]`: `### Added` modifiers on click and drag; `### Fixed` the Wayland cursor error on simulated moves.
- `extras/README.md` mouse section: one line for `modifiers`.
- `docs/as-built/input-modifiers.md`: written by the as-built step at the end of the run.

**Files:**
- `mcp/src/brp_tools/mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_click_mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_drag_mouse.rs`
- `mcp/help_text/brp_extras_click_mouse.txt`, `brp_extras_drag_mouse.txt`, `brp_extras_move_mouse.txt`
- `.claude/integration_tests/mouse.md`
- `extras/CHANGELOG.md`, `mcp/CHANGELOG.md`, `extras/README.md`

**Workers:** 2 writers — code and docs split by file.
- `impl` — the three `mcp/src` files and their tests; hub: `mcp/src/brp_tools/mouse.rs`.
- `test` — opens as impl: the help text, integration test, change notes and README.

**State:** none

**Constraints from prior phases:** the extras wire field is `modifiers: [String]` with values `Shift`, `Control`, `Alt`, `Super`. Unknown values are an invalid-params error. Responses echo `modifiers` only when it is not empty. Simulated moves bypass `Window` change detection (Phase 2).

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_mcp` — the new tests pass.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_mcp` — clean.
- After the user reloads the MCP server: the `mouse.md` integration test passes.

## After the phases

1. Run the project style review, then lint and tests for both crates.
2. Push the branch with /validate_and_push. Never push to `main`.
3. Report to the showrunner. The release and the crate publish are the user's.
4. After the release: a Hana change moves `bevy_brp_extras` from 0.23.0 to the new version, and the C105 how-to drops the two-call Shift+click.
