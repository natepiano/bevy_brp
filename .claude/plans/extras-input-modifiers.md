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

#### As-built

- `support::record_cursor_position(Mut<Window>, Vec2)` writes the cursor position through `bypass_change_detection`. `send_motion_events` and `DragInput::motion` use it, so extras' own cursor writes do not mark `Window` changed and bevy_winit does not warp the OS cursor for them. `window.cursor_position()` readers still see each simulated position.
- `DragState::PositionStart` comes before `Pressed`: the drag moves the cursor to `start` on its own frame, then presses its modifiers and button on the next frame. The jump from the old cursor point is never motion with the button held.
- `HeldModifiers(HashMap<Modifier, u32>)` keeps one holder count per modifier across all windows. A press goes out on 0 → 1 and a release on 1 → 0, in the window of the request that changes the count. A modifier stays held until the last holder releases it.
- `DragOperation.window` is the resolved `Entity`; the optional window id exists only on the wire (`DragMouseRequest.window`).
- `DRAG_COMPLETION_OVERHEAD_FRAMES` is 6.

**Files:**
- `extras/src/mouse/support.rs` — `record_cursor_position`, motion events.
- `extras/src/mouse/drag.rs` — drag state machine with `PositionStart`; `DragOperation.window: Entity`.
- `extras/src/mouse/modifier.rs` — `HeldModifiers` holder counts per `Modifier`.
- `extras/src/mouse/tests.rs` — `move_mouse_does_not_mark_the_window_changed`, `drag_frames_do_not_mark_the_window_changed`, `cursor_position_follows_the_drag`, `drag_start_jump_is_not_motion_while_the_button_is_held`, `modifier_stays_held_until_the_last_window_releases_it`.

**Binds later work:** help text and changelogs state that extras does not mark the Window changed, and that another per-frame Window writer can still warp. The modifier is released one frame after the button of the last holder. A drag positions itself "before this drag presses its modifiers and button".

**Gotchas:**
- bevy_winit's `CachedWindow` is `pub(crate)` and keeps the old cursor position. Any later `Window` change can warp once to the last simulated position.
- Bevy 0.20.1 `TextInputPlugin` (bevy_ui_widgets `text_input.rs:649`, in `DefaultPlugins` through `UiWidgetsPlugins`) writes `ime_enabled` every PreUpdate with no compare. In an app with it, the Window is changed every frame, so the warp and the Wayland `could not set cursor position` error happen on every simulated move (upstream row B5). Measured on natedev Wayland: the `extras_plugin` example logged 41 errors for 20 moves and one 20-frame drag.
- Drag motion, `CursorMoved` and the Window position are written in one system with no deferred flush; `drag_motion_deltas_match_cursor_positions_in_both_system_orders` guards this.
- X11 and macOS are untested for this phase.

**Ruled out:** a drag that releases buttons or modifiers another request holds before its start move — the hold belongs to the other request.

### Phase 3 — MCP tools, docs and change notes  · status: todo

#### Work Order

**Goal:** the `brp_extras_click_mouse` and `brp_extras_drag_mouse` MCP tools expose `modifiers`, and the docs and change notes describe both changes.

**Spec:**
- `mcp/src/brp_tools/mouse.rs`: add `Modifier { Shift, Control, Alt, Super }` (the same name as the extras type) with `Clone, Serialize, Deserialize, JsonSchema` (both params types derive `Clone`), doc comment per variant. It wraps no Bevy type, so it takes no `Wrapper` suffix.
- `ClickMouseParams` and `DragMouseParams`: add
  ```rust
  /// Keyboard modifiers held for the whole click or drag (Shift, Control, Alt, Super)
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub modifiers: Vec<Modifier>,
  ```
  An empty list means no modifiers. Do not use `Option<Vec<_>>`. The JsonSchema lists the four names.
- MCP schema (`mcp/src/tool/parameters.rs`): today `handle_array_type` (:334) reads only the item `type`, and `add_string_array_property` (:198) writes items as plain `string`, so the registered tool schema drops the four names. Carry the item `enum` (from an inline enum or a resolved item `$ref`) through to the registered schema. Done when the registered `brp_extras_click_mouse` and `brp_extras_drag_mouse` schemas have an optional `modifiers` array whose items list `Shift`, `Control`, `Alt`, `Super`.
- Help text: add a modifier example to `brp_extras_click_mouse.txt` (`{"button": "Left", "modifiers": ["Shift"]}` — Shift+click to add to a selection) and to `brp_extras_drag_mouse.txt`. Say that the modifier keys are pressed with the button. Say that a modifier is released one frame after the button of the last request that holds it, so overlapping requests in any window share the hold. In `brp_extras_click_mouse.txt`, replace "press and immediate release" (line 1): the button is held for the click duration (default 100 ms), then released. In `brp_extras_drag_mouse.txt`, also say that a drag first moves to `start` on its own frame, before this drag presses its modifiers and button. In `brp_extras_move_mouse.txt`, say that extras writes the cursor position without marking the `Window` changed, so a move by itself does not warp the OS cursor; another per-frame `Window` writer can still cause a warp and, on Wayland, one `could not set cursor position` error per move. Name Bevy 0.20.1's `TextInputPlugin` (bevy_ui_widgets, in `DefaultPlugins`) as such a writer.
- Tests beside the tool files:
  - Params with `modifiers` serialize to the extras wire form. Params without it leave the field out.
  - Both MCP results keep the modifier echo from the extras response. MCP serialization forwards a duplicate list as given; it adds no duplicate filter of its own.
  - The MCP params reject an unknown name (`Hyper`) with an error that names the four valid values.
  - The registered schema test for the `modifiers` enum items (see MCP schema above).
- Create `.claude/integration_tests/input_modifiers.md` for the `extras_plugin` example (`mouse.md` needs the `mouse_test` example, so it is not the place). Steps: Shift+click, Control+drag, the keyboard messages the example logs, the response echoes, a request with `["Control","Shift","Control"]` whose response echoes `["Control","Shift"]`, and a direct BRP call with `Hyper` that returns INVALID_PARAMS naming `modifiers` and `Hyper`.
- `.claude/scripts/integration_tests/mouse_test.sh` (showrunner add-on, 2026-11-16): the script reads flat response fields, but the `mouse_test` example nests them, so 58 checks fail on the Phase 1 tree. Make the script read the nested fields. The example also reports named states, not booleans: `ButtonPress::{Pressed, Released}` (`mouse_test.rs:93`) and `GizmoState::{Active, Inactive}` (`mouse_test.rs:114`); the script compares booleans at `mouse_test.sh:363` and `:545`. Compare the named states. Keep every existing check. Done when `mouse_test.sh <port>` passes against the `mouse_test` example.
- `extras/CHANGELOG.md` and `mcp/CHANGELOG.md` under `[Unreleased]`: `### Added` modifiers on click and drag; `### Fixed` simulated moves no longer mark the `Window` changed, so extras no longer causes an OS cursor warp (and a Wayland `could not set cursor position` error) per move. State the condition: an app with another per-frame `Window` writer, such as Bevy 0.20.1's `TextInputPlugin`, still warps. The as-built document states the same condition.
- `extras/README.md` mouse section: one line for `modifiers`.
- `docs/as-built/input-modifiers.md`: written by the as-built step at the end of the run.

**Files:**
- `mcp/src/brp_tools/mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_click_mouse.rs`, `mcp/src/brp_tools/tools/brp_extras_drag_mouse.rs`
- `mcp/src/tool/parameters.rs` — array item enums in the registered schema, and its tests
- `mcp/help_text/brp_extras_click_mouse.txt`, `mcp/help_text/brp_extras_drag_mouse.txt`, `mcp/help_text/brp_extras_move_mouse.txt`
- `.claude/integration_tests/input_modifiers.md`
- `.claude/scripts/integration_tests/mouse_test.sh`
- `extras/CHANGELOG.md`, `mcp/CHANGELOG.md`, `extras/README.md`

**Workers:** 2 writers — code and docs split by file.
- `impl` — the four `mcp/src` files, the `Modifier` type, the schema change and the tool and schema tests; hub: `mcp/src/brp_tools/mouse.rs`, `mcp/src/tool/parameters.rs`.
- `test` — opens as impl: the help text, `input_modifiers.md`, `mouse_test.sh`, change notes and README.

**State:** none

**Constraints from prior phases:**
- The extras wire field is `modifiers: [String]` with values `Shift`, `Control`, `Alt`, `Super`. Unknown values are an INVALID_PARAMS error that names `modifiers`. The extras drop duplicates and keep first-occurrence order. Responses echo `modifiers` only when it is not empty.
- The MCP result types carry the raw extras response, so the echo needs a test, not a new field.
- Simulated moves bypass `Window` change detection (Phase 2, `support::record_cursor_position`).
- Measured 2026-11-16 (Phase 2 live check, Wayland): the `extras_plugin` example and Hana both still log one `could not set cursor position` per cursor write. Cause: Bevy 0.20.1 `TextInputPlugin` (`bevy_ui_widgets` `text_input.rs:649`) writes `window.ime_enabled` every frame, so the `Window` is changed every frame and bevy_winit warps to each simulated position. extras cannot reach bevy_winit's `CachedWindow` (`pub(crate)`). Upstream issue B5 in hanadocs `bevy-issues.md`; Hana removes the plugin from its own plugin set.
- A drag moves to `start` on its own frame first, before this drag presses its modifiers and button (`DragState::PositionStart`). It does not release a key or button that another request holds. Modifiers are pressed with the button. `HeldModifiers` counts holders per `Modifier` (`modifier.rs:95`): the release goes out one frame after the button of the last holder, in any window.

**Acceptance gate:**
- `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_mcp` — the new tests pass, including the registered schema test for the `modifiers` enum items.
- `bash ~/.claude/scripts/direct/verify.sh lint bevy_brp_mcp` — clean.
- Unit director: `mouse_test.sh <port>` passes against the `mouse_test` example on a free port.
- After the user reloads the MCP server: run `input_modifiers.md`, and record the app, port, requests, responses and keyboard log lines.

## After the phases

1. Run the project style review, then lint and tests for both crates.
2. Mac follow-up (showrunner ruling 2026-11-16): when the builds showrunner says the bevy_brp Mac setting is on main, the unit director runs `bash ~/.claude/scripts/direct/verify.sh test bevy_brp_extras --mac "<reason>"`. A failure becomes a repair checkpoint. The Mac hover and drag stay with the user through the Phase 2 Change-Review line. X11 stays untested (a warp moves the user's real pointer); the report says so.
3. Push the branch with /validate_and_push. Never push to `main`.
4. Report to the showrunner. The release and the crate publish are the user's.
5. After the release: a Hana change moves `bevy_brp_extras` from 0.23.0 to the new version, and the C105 how-to drops the two-call Shift+click.
