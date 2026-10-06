# PR #13 simulated gamepads: maintainer fixes

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Maintainer commits on top of PR #13 (`37f0223d`, simulated gamepads): API fixes, one shared request-helper module, and a first end-to-end run of `extras_gamepad`.
> **Production: pr-13-gamepads** — unit `gamepad-unit`; production doc `.claude/plans/pr-13-gamepads-production.md`

## Delegation Context

- **Project:** bevy_brp workspace. `bevy_brp_extras` (`extras/`) is the Bevy plugin that adds the `brp_extras/*` BRP methods. `bevy_brp_mcp` (`mcp/`) is the MCP server whose `brp_extras_*` tools call them. The worktree is `/home/natepiano/rust/bevy_brp_gamepad` on branch `unit/gamepad`. It starts from `prod/pr-13-gamepads` (`b54ba886`: current `main` with PR head `37f0223d` merged in). Every path below is relative to that worktree.
- **Project started:** 2026-10-06T16:28:51.025+00:00
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

### Phase 1 — Gamepad API fixes  · status: done

#### As-built

- `brp_extras/send_gamepad_button` taps. `SendGamepadButtonRequest { gamepad: u64, button: GamepadButton, #[serde(default)] duration_ms: Option<u32> }` presses at `1.0` and spawns a `TimedGamepadButtonRelease` timed on the real clock. `duration_ms` defaults to `DEFAULT_GAMEPAD_DURATION_MS` (100) and errors above `MAX_GAMEPAD_DURATION_MS` (60 000); `0` still shows the button down for one frame. `SendGamepadButtonResponse { gamepad: u64, button: GamepadButton, duration_ms: u32 }` always carries the duration.
- `brp_extras/set_gamepad_button` holds. `SetGamepadButtonRequest { gamepad: u64, button: GamepadButton, value: f32 }` requires `value` in `[0.0, 1.0]` ("Button value {value} is outside [0.0, 1.0]"), spawns no release, and the value stays until set again. `SetGamepadButtonResponse { gamepad: u64, button: GamepadButton, value: f32 }`.
- Send and set cancel any pending release for that `(gamepad, button)`; disconnect cancels every release for the pad. All three call the private `cancel_pending_button_releases(world, gamepad, ReleaseCancellation)`, with `ReleaseCancellation::{AllButtons, Button(b)}`.
- Every connection, button and axis write, and every timed release, reaches both `RawGamepadEvent` and its typed message, as `bevy_gilrs` writes them. Handlers use the private `write_raw_gamepad_event<T>(world: &mut World, event: T) where T: Clone + Message, RawGamepadEvent: From<T>`; `process_timed_gamepad_button_releases` writes both through `MessageWriter`s.
- `disconnect_gamepad_handler` removes `SimulatedGamepad`; Bevy removes `Gamepad` and keeps the entity. Every later send, set, axis or disconnect on it returns `INVALID_PARAMS` "Entity {bits} is not a connected simulated gamepad (never connected, or disconnected); connect one with brp_extras/connect_gamepad".
- `process_timed_gamepad_button_releases` queries `Query<Option<&Gamepad>, With<SimulatedGamepad>>`: no marker despawns the release; marker without `Gamepad` waits, which covers a tap sent in the connect frame (the system runs `.before(InputSystems)`); a present pad releases once `Gamepad::pressed` has reported the button and the timer is finished.
- The four gamepad handlers and `SimulatedGamepad` are private. `remote_methods` stays `pub(crate)` and returns 5 methods, among them `METHOD_SET_GAMEPAD_BUTTON`. `plugin.rs` imports `GamepadPlugin` by name and calls `gamepad::remote_methods` by path.
- MCP: `GamepadButtonWrapper` (19 named buttons) and `GamepadAxisWrapper` (6 named axes) type the `button` and `axis` params; Bevy's `Other(u8)` is excluded, and an unknown name fails with serde's "unknown variant … expected one of" list. `SendGamepadButtonParams` has no `value`. Tool `brp_extras_set_gamepad_button` (`SetGamepadButtonParams`, `SetGamepadButtonResult`) is `AdditiveIdempotent`; `brp_extras_disconnect_gamepad` is `DestructiveIdempotent`.
- `build_parameters_from` takes a field's `description` from the property before its resolved `$defs` schema, and writes `"enum": [...]` on any string property whose resolved schema is a string-const `oneOf` or `{type: "string", enum: [...]}`. `MouseButtonWrapper` and both gamepad wrappers list their names.

**Files:**
- `extras/src/gamepad.rs` — handlers, timed release system, private helpers `write_raw_gamepad_event` and `cancel_pending_button_releases`, inline tests
- `extras/src/constants.rs`, `extras/src/plugin.rs`, `extras/src/lib.rs`, `extras/README.md`, `extras/CHANGELOG.md` — method constant, plugin import, `## Gamepad` crate docs, method list
- `mcp/src/brp_tools/gamepad.rs` — `GamepadButtonWrapper`, `GamepadAxisWrapper`
- `mcp/src/brp_tools/tools/brp_extras_send_gamepad_button.rs`, `brp_extras_set_gamepad_button.rs`, `brp_extras_set_gamepad_axis.rs` — typed params and results; `mcp/src/brp_tools/mod.rs`, `tools/mod.rs` — re-exports
- `mcp/src/tool/name.rs`, `mcp/src/tool/parameters.rs` — tool registration and annotations; enum lists and field descriptions in tool schemas; tests
- `mcp/help_text/brp_extras_send_gamepad_button.txt`, `brp_extras_set_gamepad_button.txt`, `brp_extras_disconnect_gamepad.txt`, `mcp/README.md`, `mcp/CHANGELOG.md` — tap vs hold; later calls fail after disconnect
- `.claude/integration_tests/extras_gamepad.md` — steps 3–4 (set hold, send tap, cancelled release), 6 (errors), 7 (disconnected pad rejects input); `.claude/agents/integration-tester.md` — set-button tool listed

**Binds later work:** `cancel_pending_button_releases` and `ReleaseCancellation` stay in `extras/src/gamepad.rs` (Shared request helpers). End-to-end extras_gamepad run executes spec steps 3–4 (tap vs hold, observed through `GamepadInputHistory`) and step 7 (a disconnected pad rejects input).

**Gotchas:**
- A timed release gates on `Gamepad::pressed` (digital), not the analog value; after a partial set below the press threshold, a 0 ms tap would otherwise never be seen pressed.
- Toolchain clippy (rust 1.99) denies `clippy::assert_is_empty`; empty checks are written `assert_eq!(x, Vec::<T>::new())`.
- Tests read `Messages<…>` with `MessageCursor::default()`: the paused virtual clock leaves message buffers unswapped.

**Ruled out:** a `TracingLevel` enum list — `SetTracingLevelParams.level` is a plain `String`, so the schema has no enum to list.

### Phase 2 — Shared request helpers  · status: done

#### As-built

`extras/src/brp_request.rs` (crate-private) owns request parsing, the `INVALID_PARAMS` error and response serialization for the mouse, keyboard and gamepad handlers. No local copy remains in those modules.
- `pub(crate) enum EmptyParamsPolicy { Allow, Reject }` — `Allow` parses `None` params as `{}`. `connect_gamepad` and `double_tap_gesture` use `Allow`; every other handler uses `Reject`.
- `pub(crate) fn parse_request<T: serde::de::DeserializeOwned>(params: Option<Value>, empty_params_policy: EmptyParamsPolicy) -> Result<T, BrpError>` — `MISSING_REQUEST_PARAMETERS_MESSAGE` for `None` under `Reject`, `"Failed to parse parameters: {e}"` for a serde error. Keyboard parse errors use this wording (formerly "Invalid request format: …"); no test asserts either text.
- `pub(crate) const fn invalid_params(message: String) -> BrpError` — `INVALID_PARAMS`, `data: None`. Every `INVALID_PARAMS` error in `mouse/`, `keyboard/` and `gamepad.rs` is built through it.
- `pub(crate) fn serialize_response<T: Serialize>(response: T, handler_name: &str) -> BrpResult` — on failure logs `warn!` and returns `INTERNAL_ERROR`.
- Call sites import the module (`use crate::brp_request;`) and call `brp_request::parse_request` and its siblings; only `EmptyParamsPolicy` is imported by name.

**Files:**
- `extras/src/brp_request.rs` — the shared request module; registered by `mod brp_request;` in `extras/src/lib.rs`
- `extras/src/mouse/support.rs` — mouse helpers only (`resolve_window_entity`, `send_timed_button_press`, `send_motion_events`, `resolve_window`)
- `extras/src/mouse/{button,click,cursor,drag,gestures,scroll}.rs`, `extras/src/keyboard/{keys,typing}.rs`, `extras/src/gamepad.rs` — call sites

**Binds later work:** every validation message in mouse, keyboard and gamepad is byte-for-byte unchanged apart from the keyboard parse-error wording; the `simulated_gamepad` text ("not a connected simulated gamepad …") and the range and duration messages are asserted by the gamepad unit tests and by `.claude/integration_tests/extras_gamepad.md`.

**Ruled out:** routing `screenshot/request.rs` (`from_params`, "Invalid screenshot request") and `window_title.rs` (single-field extraction) through `brp_request` — they build their own `INVALID_PARAMS` errors with their own messages and are not copies of these helpers.

### Phase 3 — End-to-end extras_gamepad run  · status: done

#### As-built

`.claude/integration_tests/extras_gamepad.md` passes all 15 checks through an MCP binary built from this worktree against `extras_plugin` on port 20250. Steps: connect (2); hold and release (3); timed release, with East at the 100 ms default, North at 300 ms, then West sent for 3000 ms with a `set_gamepad_button` on West in the same batch (4); axis, where -0.75 arrives as about -0.737 after Bevy's dead zone (5); five error cases (6); set cancels the timed release, checking after the 3 s window that West is still held and `last_released` is still `"North"`, then releasing West (7); disconnect and post-disconnect rejection (8).

**Files:**
- `.claude/integration_tests/extras_gamepad.md` — the gamepad integration spec, steps 1–8 above.

**Gotchas:**
- Worktree build without touching the global binary: `cargo build -p bevy_brp_mcp --bin bevy_brp_mcp` in the worktree, then a session started with `--strict-mcp-config --mcp-config .claude/transient/mcp-unit.json` (git-ignored), a copy of the global `brp` entry with `command` set to the worktree's `target/debug/bevy_brp_mcp`. The server keeps the name `brp`, so tools stay `mcp__brp__*` as `.claude/agents/integration-tester.md` lists them. Never `cargo install --path mcp`: `~/.cargo/bin/bevy_brp_mcp` serves other sessions.
- Never run `/integration_tests` for this spec: its `cleanup_stale_test_processes.sh` runs `pkill -x extras_plugin` against every session's app, and it assigns ports from the 20100 pool. Its DedicatedAppPrompt (`.claude/commands/integration_tests.md`) runs by hand on port 20250 instead, with `brp_launch` `path` set to the worktree (that builds the worktree's `test-app` whatever the session cwd), and shutdown touches only that port.
- Two MCP calls reach the app about 640 ms apart even in one batch; a step that needs a call inside a timed window uses a window of seconds and checks the outcome after the window has passed.
- `GamepadInputHistory` is shared across every pad in the app; a rerun after a run that left a button held needs a fresh app launch.

**Ruled out:** a 300 ms hold for the cancel check — shorter than the latency between two MCP calls, so it tests a re-press, not a cancel.

