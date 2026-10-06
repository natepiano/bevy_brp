# PR #13 simulated gamepads: maintainer fixes

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Maintainer commits on top of PR #13 (`37f0223d`, simulated gamepads): API fixes, one shared request-helper module, and a first end-to-end run of `extras_gamepad`.
> **Production: pr-13-gamepads** — unit `gamepad-unit`; production doc `.claude/plans/pr-13-gamepads-production.md`

## Delegation Context

- **Project:** bevy_brp workspace. `bevy_brp_extras` (`extras/`) is the Bevy plugin that adds the `brp_extras/*` BRP methods. `bevy_brp_mcp` (`mcp/`) is the MCP server whose `brp_extras_*` tools call them. The worktree is `/home/natepiano/rust/bevy_brp_gamepad` on branch `unit/gamepad`. It starts from `prod/pr-13-gamepads` (`b54ba886`: current `main` with PR head `37f0223d` merged in). Every path below is relative to that worktree.
- **Stack:** Rust 2024, Bevy 0.19.1 (`bevy_input` gamepad, `bevy_remote`), serde / serde_json, schemars 1.2 (MCP tool schemas), the `bevy_brp_mcp_macros` derives (`ParamStruct`, `ResultStruct`, `BrpTools`, `ToolDescription`).
- **Layout:**
  - `extras/src/gamepad.rs`: the whole gamepad feature (behind the default `gamepad` cargo feature). It has request and response types, `GamepadPlugin`, `remote_methods`, the four handlers, `process_timed_gamepad_button_releases`, private helpers and an inline `mod tests`.
  - `extras/src/mouse/`, `extras/src/keyboard/`: the patterns to copy (default 100 ms hold, `Time<Real>` releases, writes to two streams).
  - `extras/src/{lib.rs,plugin.rs,constants.rs,window_event.rs}`: crate docs, plugin wiring, method-name constants, the helper that writes to two streams.
  - `mcp/src/brp_tools/tools/brp_extras_*.rs`: one file per tool's params and result. `mcp/src/brp_tools/mouse.rs` holds `MouseButtonWrapper`.
  - `mcp/src/tool/name.rs`: the `ToolName` registry, with annotations, parameter builders and handlers. `mcp/src/tool/parameters.rs` turns a schemars schema into the tool input schema.
  - `mcp/help_text/<tool_name>.txt`: per-tool help, pulled in with `include_str!` by tool name. A missing file fails the build.
  - `.claude/integration_tests/extras_gamepad.md`, `.claude/agents/integration-tester.md`, `.claude/commands/integration_tests.md`: the agentic test spec, the tester agent and its tool list, and the runner.
- **Key files:**
  - `extras/src/gamepad.rs` — gamepad feature. `send_gamepad_button_handler` ~214, both-stream gap ~257 and ~287, `disconnect_gamepad_handler` ~303, `process_timed_gamepad_button_releases` ~341, local `invalid_params`/`parse_request`/`to_value` ~373–394, `simulated_gamepad` ~397, tests ~408.
  - `extras/src/mouse/support.rs` — `EmptyParamsPolicy`, `parse_request`, `serialize_response`, plus mouse-only helpers (`resolve_window`, `send_timed_button_press`, `send_motion_events`, `resolve_window_entity`).
  - `extras/src/mouse/button.rs` — `send_mouse_button` pattern: `DEFAULT_MOUSE_DURATION_MS` (100), `MAX_MOUSE_DURATION_MS` (60 000), a response that always carries `duration_ms`.
  - `extras/src/mouse/{click,cursor,drag,gestures,scroll}.rs` — callers of `support::parse_request` / `support::serialize_response`.
  - `extras/src/keyboard/keys.rs` ~90, `extras/src/keyboard/typing.rs` ~133 — inline copies of the parse logic ("Invalid request format: {e}") and `Ok(json!(…))` responses.
  - `extras/src/window_event.rs` — `write_input_event<T: Clone + Message>`, which writes one event to two channels. It is the model for item 2.
  - `extras/src/plugin.rs` — `use super::gamepad;` ~44, `app.add_plugins(gamepad::GamepadPlugin)` ~367 (the inline path Mend flags), `methods.extend(gamepad::remote_methods(world))` ~490.
  - `extras/src/constants.rs` — `METHOD_*_GAMEPAD*` (each `#[cfg(feature = "gamepad")]`), `MISSING_REQUEST_PARAMETERS_MESSAGE`.
  - `extras/src/lib.rs` — crate docs `## Gamepad` section ~148–176; `mod` list ~240.
  - `mcp/src/brp_tools/tools/brp_extras_{connect,disconnect,send,set}_gamepad_{…}.rs` — gamepad tool params. `button: String` at `brp_extras_send_gamepad_button.rs` ~19, `axis: String` at `brp_extras_set_gamepad_axis.rs` ~19.
  - `mcp/src/brp_tools/tools/brp_extras_send_mouse_button.rs`, `mcp/src/brp_tools/mouse.rs` — the enum-param pattern to copy.
  - `mcp/src/brp_tools/tools/mod.rs`, `mcp/src/brp_tools/mod.rs` — `mod` and `pub use` facade for tool params and results.
  - `mcp/src/tool/name.rs` — enum variants ~348–375, annotations ~630–649 (disconnect is `AdditiveIdempotent` at ~647), `get_parameters` ~843–854, handlers ~937–940, tests ~991.
  - `mcp/src/tool/parameters.rs` — `handle_one_of_schema` ~382, `map_schema_type_to_parameter_type` ~437, `resolve_schema_value` ~497, `build_parameters_from` ~588, `add_string_property` ~178, tests ~664.
  - `mcp/help_text/brp_extras_*gamepad*.txt` — gamepad tool help.
  - `extras/README.md` ~28, `extras/CHANGELOG.md` ~11, `mcp/README.md` ~58–61, `mcp/CHANGELOG.md` ~11 — the PR's doc lines.
  - `test-app/examples/extras_plugin.rs` — `GamepadInputHistory` ~496 and `track_gamepad_input` ~2336. It reads processed `GamepadButtonStateChangedEvent` / `GamepadAxisChangedEvent`, and nothing in it changes.
- **Test lanes:** `bevy_brp_extras` — `extras/tests/` (public-API tests only; the gamepad, mouse and keyboard handlers are private, so their tests live in inline `mod tests`); `bevy_brp_mcp` — none (inline `mod tests`).
- **Build:** `bash ~/.claude/scripts/delegate/verify.sh check bevy_brp_extras`; `bash ~/.claude/scripts/delegate/verify.sh check bevy_brp_mcp`
- **Test:** `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_extras`; `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_mcp`
- **Lint:** `bash ~/.claude/scripts/delegate/verify.sh lint bevy_brp_extras` (lint covers the whole workspace: run it once after a seat's edits, never once per crate; on failure fix the named error wherever it is, then lint once, never re-lint an unchanged tree)
- **Style:** run-end /clippy style-only auto-proceed
- **Invariants:**
  - The contributor's commit `37f0223d` stays as written. Every change is a new commit on `unit/gamepad` (source plan: the PR allows maintainer edits).
  - Simulated pads are fed exactly what `bevy_gilrs` 0.19.1 writes (`gilrs_system.rs`): each connection change goes to `GamepadConnectionEvent` and `RawGamepadEvent::Connection`, each button change to `RawGamepadEvent::Button` and `RawGamepadButtonChangedEvent`, and each axis change to `RawGamepadEvent::Axis` and `RawGamepadAxisChangedEvent`. `RawGamepadEvent` derives `From` for all three payloads. The two raw payloads are `Copy`; `GamepadConnectionEvent` is `Clone`.
  - Gamepad methods accept only entities that carry `SimulatedGamepad`, so a real pad is never driven.
  - Timed releases tick on `Time<Real>`, as keyboard and mouse do. A release is written only once the `Gamepad` reports the button down, so a timed press is always seen down for at least one frame.
  - One rule across the input API: "send" is brief (100 ms default hold, 60 000 ms max), "set" keeps its value until changed. User call in the source plan, flagged easy to undo.
  - Nothing users see on screen changes: these are MCP tools and a Bevy plugin with no UI. No phase takes UX shots, and checkpoint notices read `Shots: none, no visible change`.
  - Never `cargo install --path mcp`. The global `~/.cargo/bin/bevy_brp_mcp` serves other sessions and productions (source plan).
  - App launches use port 20250, debug profile, target `extras_plugin` (production unit row).
  - Out of scope (source plan): porting to the Bevy 0.20 RC branch (it reaches `upd/bevy-release-candidate` through `main`), and the contributor's note that two `agent_tools::registration` tests fail on `main`.

## Phases

### Phase 1 — Gamepad API fixes  · status: todo

#### Work Order

**Goal:** `send_gamepad_button` taps for 100 ms and a new `set_gamepad_button` holds. Every press, axis change and release reaches both raw message streams. A disconnected pad rejects input. The MCP tool schemas list the 19 buttons and 6 axes. Mend is clean, and docs and tests match.

**Spec:**

*Item 1: send taps, set holds (`extras/src/gamepad.rs`, `extras/src/constants.rs`).*
- Add `const DEFAULT_GAMEPAD_DURATION_MS: u32 = 100;` next to `MAX_GAMEPAD_DURATION_MS` at the top of `gamepad.rs`.
- `SendGamepadButtonRequest` becomes `{ gamepad: u64, button: GamepadButton, #[serde(default)] duration_ms: Option<u32> }`. The `value` field goes: send is a full press (`1.0`) like a mouse click. `duration_ms` defaults to `DEFAULT_GAMEPAD_DURATION_MS`, and a value above `MAX_GAMEPAD_DURATION_MS` errors with the existing message. `0` is accepted, and the button is still seen down for one frame.
- `send_gamepad_button_handler`: resolve the pad, validate the duration, and despawn any `TimedGamepadButtonRelease` for this `(gamepad, button)`. Then spawn a new release with `Timer::new(Duration::from_millis(u64::from(duration_ms)), TimerMode::Once)` and write the press (value `1.0`) through the both-streams helper (item 2). The response `SendGamepadButtonResponse { gamepad: u64, button: GamepadButton, duration_ms: u32 }` always carries `duration_ms`, as `SendMouseButtonResponse` does.
- New `SetGamepadButtonRequest { gamepad: u64, button: GamepadButton, value: f32 }`, where `value` is required as in `SetGamepadAxisRequest`. New `SetGamepadButtonResponse { gamepad: u64, button: GamepadButton, value: f32 }`.
- New `set_gamepad_button_handler`. It checks that `value` is in `[0.0, 1.0]` (existing message "Button value {value} is outside [0.0, 1.0]") and despawns any pending release for this `(gamepad, button)`, since a new value cancels a pending release. It writes the value through the both-streams helper and spawns no release, so the value stays until changed.
- `constants.rs`: add `#[cfg(feature = "gamepad")] pub(crate) const METHOD_SET_GAMEPAD_BUTTON: &str = "set_gamepad_button";`, sorted alphabetically between `METHOD_SEND_MOUSE_BUTTON` and `METHOD_SET_GAMEPAD_AXIS`. `remote_methods` returns `[(String, RemoteMethodSystemId); 5]` with the new method.

*Item 2: both message streams (`extras/src/gamepad.rs`).*
- Add a private helper modeled on `window_event::write_input_event`:
  `fn write_raw_gamepad_event<T>(world: &mut World, event: T) where T: Clone + Message, RawGamepadEvent: From<T>`. It writes `RawGamepadEvent::from(event.clone())`, then `event`.
- Use it for every handler write: connect and disconnect (`GamepadConnectionEvent`, replacing the two hand-written lines), send and set button (`RawGamepadButtonChangedEvent`), and axis (`RawGamepadAxisChangedEvent`).
- `process_timed_gamepad_button_releases` gains `mut button_events: MessageWriter<RawGamepadButtonChangedEvent>`. It writes the release to `raw_events` (as `RawGamepadEvent::Button`) and to `button_events`, as `mouse::button::process_timed_button_releases` writes both `MouseButtonInput` and `WindowEvent`.

*Item 3: disconnected pads reject input (`extras/src/gamepad.rs`, `mcp/src/tool/name.rs`).*
- `disconnect_gamepad_handler` removes `SimulatedGamepad` from the entity (`world.entity_mut(gamepad).remove::<SimulatedGamepad>()`). It keeps its current despawn of the pad's pending releases and writes the disconnection through the helper. Bevy still removes `Gamepad` and leaves the entity (with `Name` and `GamepadSettings`).
- `simulated_gamepad` error message becomes: `"Entity {bits} is not a connected simulated gamepad (never connected, or disconnected); connect one with brp_extras/connect_gamepad"`, code `INVALID_PARAMS`. After a disconnect, every later call fails with it: send, set, axis, and a second disconnect.
- `process_timed_gamepad_button_releases` keys pad liveness on the marker, not on `Gamepad`. It despawns a release whose pad no longer carries `SimulatedGamepad`. While the pad carries the marker but has no `Gamepad` yet, the release waits. That is the frame after a connect: the system runs `.before(InputSystems)`, so a tap sent in the connect frame used to lose its release and stick down. The query becomes `gamepads: Query<Option<&Gamepad>, With<SimulatedGamepad>>`. `Err` means despawn the release; `Ok(None)` means keep waiting; `Ok(Some(pad))` means apply the existing "taken and finished" rule.
- `mcp/src/tool/name.rs` ~647: `BrpExtrasDisconnectGamepad` annotation becomes `EnvironmentImpact::DestructiveIdempotent`, as `WorldDespawnEntity`, `WorldRemoveComponents` and `BrpShutdown` are.

*Item 4: typed `button` and `axis` MCP parameters (`mcp/`).*
- New `mcp/src/brp_tools/gamepad.rs`, modeled on `mcp/src/brp_tools/mouse.rs`. It has `#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)] pub enum GamepadButtonWrapper` with Bevy's 19 named variants, each with a one-line doc: `South, East, North, West, C, Z, LeftTrigger, LeftTrigger2, RightTrigger, RightTrigger2, Select, Start, Mode, LeftThumb, RightThumb, DPadUp, DPadDown, DPadLeft, DPadRight`. Next to it goes `pub enum GamepadAxisWrapper` with the 6 named axes: `LeftStickX, LeftStickY, LeftZ, RightStickX, RightStickY, RightZ`. Bevy's `Other(u8)` is left out of both. Unit variants serialize as `"South"`, which is what Bevy's `GamepadButton` / `GamepadAxis` deserialize. Add `mod gamepad;` to `mcp/src/brp_tools/mod.rs`.
- `SendGamepadButtonParams { gamepad: u64, button: GamepadButtonWrapper, duration_ms: Option<u32> (doc: "default: 100ms, max: 60000ms"), port }`. Drop `value`.
- New `mcp/src/brp_tools/tools/brp_extras_set_gamepad_button.rs`: `SetGamepadButtonParams { gamepad: u64, button: GamepadButtonWrapper, value: f32 (doc: "Analog value in [0.0, 1.0]; 1.0 pressed, 0.0 released; stays until set again"), port }` and `SetGamepadButtonResult` (message template "Gamepad button set"). The derives and the `Option<Value>` result field are copied from `brp_extras_set_gamepad_axis.rs`. Change `SendGamepadButtonResult`'s template to "Gamepad button sent".
- `SetGamepadAxisParams.axis: GamepadAxisWrapper`.
- Register `BrpExtrasSetGamepadButton` everywhere `BrpExtrasSetGamepadAxis` appears. In `name.rs` that is the imports, a variant with `#[brp_tool(brp_method = "brp_extras/set_gamepad_button", params = "SetGamepadButtonParams", result = "SetGamepadButtonResult")]` placed after `BrpExtrasSendGamepadButton`, the annotation `"set gamepad button"`, `ToolCategory::Extras`, `EnvironmentImpact::AdditiveIdempotent`, `get_parameters`, and the handler `Arc::new(…)`. It also goes in `mcp/src/brp_tools/tools/mod.rs` (`mod` and `pub use`) and in `mcp/src/brp_tools/mod.rs` (`pub use`).
- Schema fix in `mcp/src/tool/parameters.rs`. Today an enum field never reaches the agent-visible schema as an enum. `resolve_schema_value` swaps the property for its `$defs` entry. The field's own `description` (schemars puts it beside `$ref`) is then replaced by the enum's type doc. `handle_one_of_schema` maps string-const `oneOf` to `ParameterType::String`, and `add_string_property` writes only `type` and `description`. As a result `MouseButtonWrapper`'s five names never appear, and gamepad enums would hit the same gap. The fix is made where the defect lives:
  1. In `build_parameters_from`, take `description` from the property object first and fall back to the resolved schema's.
  2. When the resolved schema is a string enum, collect its values: a `oneOf` whose variants are all `{type: "string", const: …}` gives the consts; a `{type: "string", enum: [...]}` gives `enum`. Emit them as `"enum": [...]` on the string property. Add an `enum_values: Option<Vec<Value>>` argument or a sibling `add_string_enum_property` on `ParameterBuilder`, and leave other property kinds unchanged.
  This also gives `SearchOrder`, `NameMatchMode`, `ScrollUnitWrapper`, `TracingLevel` and `MouseButtonWrapper` their enum lists. That change is additive, and serde already rejects any other value.

*Item 5: Mend.*
- In `gamepad.rs`, make the four handlers and `SimulatedGamepad` private (`fn` / `struct`). Each is used only inside `gamepad.rs`: the plugin reaches them through `remote_methods`. That fixes the five over-visible items. `remote_methods` stays `pub(crate)`. In `plugin.rs`, replace `use super::gamepad;` + `gamepad::GamepadPlugin` with `#[cfg(feature = "gamepad")] use super::gamepad::GamepadPlugin;` and `app.add_plugins(GamepadPlugin)`. Keep `gamepad::remote_methods`, which needs the module import as well, as a function path, following `import-the-module-for-functions`. `verify.sh lint` runs `cargo mend --fix`; it must leave nothing to fix.

*Item 6: docs and tests follow.*
- Help text. `mcp/help_text/brp_extras_send_gamepad_button.txt` changes to: tap for `duration_ms` (default 100), seen down for at least one frame, released on the real clock. Its examples are a tap, a 500 ms hold, and a 0 ms tap, and it points to `brp_extras_set_gamepad_button` for holds and analog values. New `mcp/help_text/brp_extras_set_gamepad_button.txt` covers holding until changed, `value` 0.0 releasing, a half trigger, and cancelling a pending tap release, and it carries the button list. `brp_extras_disconnect_gamepad.txt` adds that later calls on the pad fail, so connect a new one. `brp_extras_set_gamepad_axis.txt` is unchanged except for any wording that names `send_gamepad_button` holds.
- `extras/src/lib.rs` `## Gamepad` docs: `send_gamepad_button` (`duration_ms` default 100), a new `### brp_extras/set_gamepad_button` section (`value` required), and disconnect making later calls fail. `extras/src/plugin.rs` doc list ~65 stays "connect_gamepad and friends".
- `extras/README.md` ~28 and `mcp/README.md` ~58–61 add `set_gamepad_button`; the mcp line for send reads "Tap a simulated gamepad button". `extras/CHANGELOG.md` and `mcp/CHANGELOG.md` `[Unreleased]` lines name all five methods or tools, the send/set rule and typed button and axis names.
- `.claude/agents/integration-tester.md`: add `mcp__brp__brp_extras_set_gamepad_button` to `tools:` after `mcp__brp__brp_extras_send_gamepad_button`.
- `.claude/integration_tests/extras_gamepad.md`, each step using the new API:
  - Step 3 "Hold and Release" uses `mcp__brp__brp_extras_set_gamepad_button` with `value` 1.0 and then 0.0.
  - Step 4 "Timed Release" has three parts. A default `send_gamepad_button` `{"gamepad": G, "button": "East"}` is released within 1 s with `last_released` `"East"`. An explicit `duration_ms: 300` on `"North"` behaves the same way. A `set_gamepad_button` value 1.0 on `"West"` right after a `send` of `"West"` stays down after 1 s, because the pending release was cancelled.
  - Step 6 keeps excessive duration, unknown button (the error lists the valid names, from serde's "unknown variant … expected one of"), out-of-range axis and non-simulated entity. It adds an out-of-range `set_gamepad_button` value of `1.5`.
  - Step 7 adds that after disconnect, `send_gamepad_button`, `set_gamepad_button` and `set_gamepad_axis` on `G` fail with an error containing "not a connected simulated gamepad".
  - Expected Results add "a disconnected pad rejects input".
- Unit tests in `gamepad.rs` `mod tests`:
  - Rename `button_holds_until_set_again` to `set_button_holds_until_set_again` and switch it to `set_gamepad_button_handler`.
  - Switch the two timed tests to `send_gamepad_button_handler`.
  - New tests:
    - `send_button_taps_for_the_default_duration`: pressed after one frame, still pressed at 99 ms real time, released at `DEFAULT_GAMEPAD_DURATION_MS`.
    - `a_new_value_cancels_a_pending_release`: send South, then set South 1.0, then advance 5 000 ms; still pressed.
    - `disconnect_drops_pending_releases`: send with 5 000 ms, disconnect, and no `TimedGamepadButtonRelease` remains.
    - `disconnected_pad_rejects_input`: send, set, axis and disconnect after a disconnect each return `INVALID_PARAMS` containing "not a connected simulated gamepad".
    - `tap_sent_in_the_connect_frame_still_releases`: call connect then send with no frame between; the button is seen pressed, then released after the duration.
    - `button_and_axis_changes_reach_both_raw_streams`: read `Messages<RawGamepadButtonChangedEvent>`, `Messages<RawGamepadAxisChangedEvent>` and `Messages<RawGamepadEvent>` with `MessageCursor::default()` as `mouse/mod.rs` `button_events` does (the paused virtual clock keeps buffers unswapped). A set, a timed release and an axis each appear in both streams.
    - `set_button_rejects_out_of_range_values`.
  - Remove `send` with `value` uses.
  - `mcp` tests in `parameters.rs` `mod tests`: `enum_field_lists_its_variants`, where `build_parameters_from::<SendMouseButtonParams>().build()` has `properties.button.enum == ["Left","Right","Middle","Back","Forward"]` and the field's own description. `gamepad_tool_schemas_list_buttons_and_axes`, where `SetGamepadButtonParams` lists 19 button names and `SetGamepadAxisParams` 6 axis names. In `name.rs` `mod tests`: `disconnect_gamepad_is_destructive_idempotent`.

**Files:**
- `extras/src/gamepad.rs` — items 1, 2, 3, 5; unit tests
- `extras/src/constants.rs` — `METHOD_SET_GAMEPAD_BUTTON`
- `extras/src/plugin.rs` — `GamepadPlugin` import (Mend)
- `extras/src/lib.rs` — `## Gamepad` crate docs
- `extras/README.md` — method list
- `extras/CHANGELOG.md` — `[Unreleased]` line
- `mcp/src/brp_tools/gamepad.rs` — new: `GamepadButtonWrapper`, `GamepadAxisWrapper`
- `mcp/src/brp_tools/mod.rs` — `mod gamepad;`, `pub use` for set-button params and result
- `mcp/src/brp_tools/tools/mod.rs` — `mod` + `pub use` for the new tool file
- `mcp/src/brp_tools/tools/brp_extras_send_gamepad_button.rs` — typed `button`, drop `value`, result template
- `mcp/src/brp_tools/tools/brp_extras_set_gamepad_button.rs` — new tool params and result
- `mcp/src/brp_tools/tools/brp_extras_set_gamepad_axis.rs` — typed `axis`
- `mcp/src/tool/name.rs` — new variant everywhere, disconnect annotation, test
- `mcp/src/tool/parameters.rs` — enum values and field descriptions in the schema, tests
- `mcp/help_text/brp_extras_send_gamepad_button.txt` — tap semantics
- `mcp/help_text/brp_extras_set_gamepad_button.txt` — new
- `mcp/help_text/brp_extras_disconnect_gamepad.txt` — later calls fail
- `mcp/help_text/brp_extras_set_gamepad_axis.txt` — wording only if it names send holds
- `mcp/README.md` — tool list
- `mcp/CHANGELOG.md` — `[Unreleased]` line
- `.claude/agents/integration-tester.md` — tool list
- `.claude/integration_tests/extras_gamepad.md` — spec steps

**Seats:** 2 writers — split by crate: the extras side and the MCP side meet only at JSON method names and field names, which this Spec fixes, so each compiles alone.
- `impl` — `extras/src/gamepad.rs`, `extras/src/constants.rs`, `extras/src/plugin.rs`, `extras/README.md`, `extras/CHANGELOG.md`; hub: `extras/src/lib.rs` (crate docs and `mod` list)
- `test` opens as impl — `mcp/src/brp_tools/gamepad.rs`, `mcp/src/brp_tools/tools/brp_extras_send_gamepad_button.rs`, `mcp/src/brp_tools/tools/brp_extras_set_gamepad_button.rs`, `mcp/src/brp_tools/tools/brp_extras_set_gamepad_axis.rs`, `mcp/src/tool/parameters.rs`, `mcp/help_text/brp_extras_send_gamepad_button.txt`, `mcp/help_text/brp_extras_set_gamepad_button.txt`, `mcp/help_text/brp_extras_disconnect_gamepad.txt`, `mcp/help_text/brp_extras_set_gamepad_axis.txt`, `mcp/README.md`, `mcp/CHANGELOG.md`, `.claude/agents/integration-tester.md`, `.claude/integration_tests/extras_gamepad.md`; hub: `mcp/src/tool/name.rs` (tool registry), `mcp/src/brp_tools/mod.rs`, `mcp/src/brp_tools/tools/mod.rs` (facade re-exports). It opens as impl because the gamepad handlers are private and tested inline in `gamepad.rs`, so `extras/tests/` cannot reach them.

**Constraints from prior phases:** none (Phase 1).

**Acceptance gate:**
- `bash ~/.claude/scripts/delegate/verify.sh check bevy_brp_extras` and `bash ~/.claude/scripts/delegate/verify.sh check bevy_brp_mcp` green.
- `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_extras` green, including every gamepad test named in the Spec.
- `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_mcp` green, including `enum_field_lists_its_variants`, `gamepad_tool_schemas_list_buttons_and_axes`, `disconnect_gamepad_is_destructive_idempotent`.
- `bash ~/.claude/scripts/delegate/verify.sh lint bevy_brp_extras` green once, with Mend reporting nothing (the five visibility items and the inline `gamepad::GamepadPlugin` path are gone).
- No UX checks: nothing users see changes.

### Phase 2 — Shared request helpers  · status: todo

#### Work Order

**Goal:** One crate-level module in `extras` owns request parsing, the `INVALID_PARAMS` error and response serialization. Mouse, keyboard and gamepad call it, and no copy remains.

**Spec:**
- New `extras/src/brp_request.rs` (crate-level, registered as `mod brp_request;` in `lib.rs`, sorted after `mod agent_tools;`). Its items are `pub(crate)`:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub(crate) enum EmptyParamsPolicy { Allow, Reject }`, moved verbatim with its doc from `mouse/support.rs`.
  - `pub(crate) fn parse_request<T: serde::de::DeserializeOwned>(params: Option<Value>, empty_params_policy: EmptyParamsPolicy) -> Result<T, BrpError>`. It is moved from `mouse/support.rs` and built on `invalid_params`. Messages: `MISSING_REQUEST_PARAMETERS_MESSAGE` for `None` under `Reject`; `format!("Failed to parse parameters: {e}")` for a serde error.
  - `pub(crate) const fn invalid_params(message: String) -> BrpError` (`INVALID_PARAMS`, `data: None`), taken from `gamepad.rs`.
  - `pub(crate) fn serialize_response<T: Serialize>(response: T, handler_name: &str) -> BrpResult`, moved from `mouse/support.rs`. It keeps its `warn!` and `INTERNAL_ERROR`.
- `mouse/support.rs` keeps only the mouse helpers (`resolve_window_entity`, `send_timed_button_press`, `send_motion_events`, `resolve_window`). `button.rs`, `click.rs`, `cursor.rs`, `drag.rs`, `gestures.rs` and `scroll.rs` call `brp_request::parse_request` / `brp_request::serialize_response` and import `crate::brp_request::EmptyParamsPolicy`. `use crate::brp_request;` imports the module, not the functions.
- `keyboard/keys.rs` `send_keys_handler` and `keyboard/typing.rs` `type_text_handler` replace their inline `if let Some(params)` blocks with `brp_request::parse_request(params, EmptyParamsPolicy::Reject)?`. They replace `Ok(json!(Response { … }))` with `brp_request::serialize_response(Response { … }, METHOD_SEND_KEYS | METHOD_TYPE_TEXT)`. The serde error text moves from "Invalid request format: {e}" to "Failed to parse parameters: {e}". No test or spec asserts the old text: `rg "Invalid request format"` finds only these two sites. `keyboard/mod.rs` `test_missing_parameters` still passes, because the missing-params message is unchanged.
- `gamepad.rs` deletes its local `invalid_params`, `parse_request` and `to_value`, and calls `brp_request::invalid_params`, `brp_request::parse_request` and `brp_request::serialize_response(…, METHOD_…)`. `connect_gamepad_handler` uses `EmptyParamsPolicy::Allow` in place of its `params.unwrap_or_else(|| Value::Object(Map::default()))` wrapper, and the rest use `Reject`.
- Drop each import that goes unused (`Map`, `INVALID_PARAMS`, `INTERNAL_ERROR`, `MISSING_REQUEST_PARAMETERS_MESSAGE`, `json`) from the files it leaves.
- Scope (author's call): `screenshot/request.rs` (`from_params`, "Invalid screenshot request") and `window_title.rs` (single-field extraction) carry their own messages and are not copies of these helpers, so they stay as they are. No behavior other than the keyboard serde error text changes.

**Files:**
- `extras/src/brp_request.rs` — new shared module
- `extras/src/lib.rs` — `mod brp_request;`
- `extras/src/mouse/support.rs` — remove moved helpers
- `extras/src/mouse/button.rs` — call sites
- `extras/src/mouse/click.rs` — call sites
- `extras/src/mouse/cursor.rs` — call sites
- `extras/src/mouse/drag.rs` — call sites
- `extras/src/mouse/gestures.rs` — call sites
- `extras/src/mouse/scroll.rs` — call sites
- `extras/src/keyboard/keys.rs` — parse and serialize through the module
- `extras/src/keyboard/typing.rs` — parse and serialize through the module
- `extras/src/gamepad.rs` — delete local helpers, call the module

**Seats:** 2 writers — split by module group. Both write against the signatures fixed above, and the tree compiles once both land.
- `impl` — `extras/src/brp_request.rs`, `extras/src/mouse/support.rs`, `extras/src/mouse/button.rs`, `extras/src/mouse/click.rs`, `extras/src/mouse/cursor.rs`, `extras/src/mouse/drag.rs`, `extras/src/mouse/gestures.rs`, `extras/src/mouse/scroll.rs`; hub: `extras/src/lib.rs` (`mod` list)
- `test` opens as impl — `extras/src/keyboard/keys.rs`, `extras/src/keyboard/typing.rs`, `extras/src/gamepad.rs`. It opens as impl because this is a pure refactor with no new behavior to test, and `extras/tests/` cannot reach private handlers.

**Constraints from prior phases:**
- Phase 1 left `gamepad.rs` with five handlers (connect, disconnect, send button, set button, set axis), the private helper `write_raw_gamepad_event`, and local `invalid_params` / `parse_request` / `to_value` that this phase removes. The handlers and `SimulatedGamepad` are private; `remote_methods` is `pub(crate)`. Method constants `METHOD_CONNECT_GAMEPAD`, `METHOD_DISCONNECT_GAMEPAD`, `METHOD_SEND_GAMEPAD_BUTTON`, `METHOD_SET_GAMEPAD_BUTTON` and `METHOD_SET_GAMEPAD_AXIS` exist in `constants.rs` under `#[cfg(feature = "gamepad")]`.
- The `simulated_gamepad` error text ("not a connected simulated gamepad …") and the range and duration messages are asserted by Phase 1 tests and by `.claude/integration_tests/extras_gamepad.md`. Keep them byte for byte.

**Acceptance gate:**
- `bash ~/.claude/scripts/delegate/verify.sh check bevy_brp_extras` green.
- `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_extras` green, with the gamepad, mouse and keyboard test modules all passing unchanged.
- `bash ~/.claude/scripts/delegate/verify.sh lint bevy_brp_extras` green once.
- `rg -n "fn parse_request|fn invalid_params|fn serialize_response|fn to_value" extras/src` lists only `extras/src/brp_request.rs`.

### Phase 3 — End-to-end extras_gamepad run  · status: todo

#### Work Order

**Goal:** The `extras_gamepad` integration spec passes every step through an MCP binary built from `unit/gamepad`, run against `extras_plugin` on port 20250.

**Spec:**
1. Build the worktree's MCP binary, run by the unit director in the background in `/home/natepiano/rust/bevy_brp_gamepad`: `cargo build -p bevy_brp_mcp --bin bevy_brp_mcp` → `/home/natepiano/rust/bevy_brp_gamepad/target/debug/bevy_brp_mcp`. This raw cargo line is the one exception to the verify.sh-only rule, because verify.sh has no binary-build verb. **Never `cargo install --path mcp`**: the global `~/.cargo/bin/bevy_brp_mcp` serves other sessions and productions.
2. Write `/home/natepiano/rust/bevy_brp_gamepad/.claude/transient/mcp-unit.json` (`.claude/transient/` is git-ignored). Its content copies the global `brp` entry in `~/.claude.json` (`type: stdio`, empty `args` and `env`) with only `command` changed:
   `{"mcpServers":{"brp":{"type":"stdio","command":"/home/natepiano/rust/bevy_brp_gamepad/target/debug/bevy_brp_mcp","args":[],"env":{}}}}`
3. End the turn with `— blocked: waiting on the showrunner: relaunch with the worktree MCP binary`. The showrunner relaunches the unit director with `--strict-mcp-config --mcp-config /home/natepiano/rust/bevy_brp_gamepad/.claude/transient/mcp-unit.json`. The server keeps the name `brp`, so every tool stays `mcp__brp__*`, which is what `.claude/agents/integration-tester.md` lists. After the relaunch, confirm the new binary is live: `mcp__brp__brp_list_agent_tools` or any tool listing shows `brp_extras_set_gamepad_button`. If it does not, report the mismatch to the showrunner and stop.
4. Launch: `mcp__brp__brp_launch` with `target_name: "extras_plugin"`, `port: 20250`, `profile: "debug"`, `search_order: "example"`, `path: "/home/natepiano/rust/bevy_brp_gamepad"`. The `path` builds this worktree's `test-app`, not the primary checkout's. Verify with `mcp__brp__brp_status(app_name: "extras_plugin", port: 20250)` = `running_with_brp`, retrying through `.claude/scripts/integration_tests/launch_retry.sh <attempt>` up to 5 times. Then set the window title with `mcp__brp__brp_extras_set_window_title` to `"extras_gamepad test - extras_plugin - port 20250"`.
5. Do **not** run `/integration_tests` itself. Its single-test path first runs `cleanup_stale_test_processes.sh`, which `pkill -x extras_plugin`s every session's app, and it takes ports from the 20100 pool. Run its `DedicatedAppPrompt` (`.claude/commands/integration_tests.md` ~201–265) by hand instead. Dispatch one `integration-tester` agent (model opus) with `[TEST_NAME]=extras_gamepad`, `[ASSIGNED_PORT]=20250`, `[APP_NAME]=extras_plugin`, `[TEST_FILE]=.claude/integration_tests/extras_gamepad.md`, and `[TEST_OBJECTIVE]` = the spec's `## Objective` text. If it returns no output, re-dispatch once.
6. Cleanup: `mcp__brp__brp_shutdown(app_name: "extras_plugin", port: 20250)`. Touch no other port or process.
7. A failing step is this unit's own change (Phase 1 or Phase 2). Fix it in the file where the defect lives, whether that is a Phase 1 or Phase 2 file or the spec when the spec itself is wrong. Re-run `verify.sh test` for the touched crate, rebuild step 1, and ask the showrunner for another relaunch with the step 3 line only if MCP code changed (extras or test-app changes need only an app relaunch on 20250). Then re-run steps 4–6. The phase is done on one run with every step passed.

**Files:**
- `.claude/integration_tests/extras_gamepad.md` — the spec under test; edited only if a step itself is wrong
- `extras/src/gamepad.rs` — fixed here only if a step fails on extras behavior
- `mcp/src/tool/parameters.rs` — fixed here only if a step fails on tool schema or param handling

**Seats:** 1 writer — nothing splits. The unit director runs the build, relaunch, launch and spec. A seat writes only a fix a failing step traces to.
- `impl` — any file a failing step traces to, starting with the three listed
- `test` opens as impl — idle unless two failures trace to disjoint crates, in which case it takes the `mcp/` one

**Constraints from prior phases:**
- Phase 1: the tools are `brp_extras_connect_gamepad`, `brp_extras_send_gamepad_button` (`gamepad`, `button`, optional `duration_ms`, default 100, max 60 000), `brp_extras_set_gamepad_button` (`gamepad`, `button`, required `value` in [0.0, 1.0]), `brp_extras_set_gamepad_axis` (`gamepad`, `axis`, `value` in [-1.0, 1.0]) and `brp_extras_disconnect_gamepad`. `button` and `axis` are enums: an unknown name fails at the MCP layer with serde's "unknown variant … expected one of …". After disconnect every call on the pad fails with "not a connected simulated gamepad". The spec and `.claude/agents/integration-tester.md` already carry `set_gamepad_button`.
- Phase 2: request errors from all extras input methods read "Failed to parse parameters: …" / "Missing request parameters".
- `GamepadInputHistory` (`test-app/examples/extras_plugin.rs`) reads processed Bevy gamepad messages. The default dead zone turns an axis value of -0.75 into about -0.737.

**Acceptance gate:**
- The integration-tester report for `extras_gamepad` shows `Test Status: Completed` and `Failed: 0` on port 20250. It is quoted in the phase report.
- `mcp__brp__brp_status(app_name: "extras_plugin", port: 20250)` reports the app stopped after cleanup.
- If any fix landed, `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_extras` and/or `bash ~/.claude/scripts/delegate/verify.sh test bevy_brp_mcp` (whichever crate was touched) are green, plus `bash ~/.claude/scripts/delegate/verify.sh lint bevy_brp_extras` once.
- No UX shots: nothing users see changes.
