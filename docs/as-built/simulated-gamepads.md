# Simulated Gamepads

## What it is

`bevy_brp_extras` adds five BRP methods: `brp_extras/connect_gamepad`, `send_gamepad_button`,
`set_gamepad_button`, `set_gamepad_axis` and `disconnect_gamepad`. `bevy_brp_mcp` exposes them as
the `brp_extras_*gamepad*` tools. Together they let an agent connect a gamepad that does not exist
and drive it. The pad needs no gilrs and no hardware. It is fed the same messages `bevy_gilrs` writes
for a real pad, so the app's input managers, menus and join screens treat it as real. This makes
gamepad paths testable on a headless or CI machine.

## How it works

**Wiring.** The feature sits behind the `gamepad` cargo feature (`gamepad = ["bevy/gamepad"]`),
which is on by default. Every gamepad item is `#[cfg(feature = "gamepad")]`.
- `extras/src/plugin.rs` runs `app.add_plugins(GamepadPlugin)`, and `register_extras_methods`
  extends its method list with `gamepad::remote_methods(world)`.
- `remote_methods(world: &mut World) -> [(String, RemoteMethodSystemId); 5]` registers each handler
  as `RemoteMethodSystemId::Instant` under `{EXTRAS_COMMAND_PREFIX}{METHOD_*}`.

**Handlers.** All five are private and have the signature
`fn(In<Option<Value>>, &mut World) -> BrpResult`. Each one parses its request, then resolves
`gamepad: u64` through `simulated_gamepad(world, bits) -> Result<Entity, BrpError>`. That function
uses `Entity::try_from_bits` and requires the private marker `SimulatedGamepad`.

| Method | Request | Effect | Response |
|---|---|---|---|
| `connect_gamepad` | `name: Option<String>` (default `SIMULATED_GAMEPAD_NAME` = "Simulated gamepad (BRP)"); `EmptyParamsPolicy::Allow` | spawns `(SimulatedGamepad, Name)` and writes `GamepadConnectionEvent::Connected { name, vendor_id: None, product_id: None }` | `{gamepad}` (`Entity::to_bits`) |
| `send_gamepad_button` (tap) | `gamepad`, `button: GamepadButton`, `duration_ms: Option<u32>` (default `DEFAULT_GAMEPAD_DURATION_MS` 100; above `MAX_GAMEPAD_DURATION_MS` 60 000 is an error) | cancels any pending release for that button, spawns a `TimedGamepadButtonRelease`, writes the button at `1.0` | `{gamepad, button, duration_ms}`, always with the duration |
| `set_gamepad_button` (hold) | `gamepad`, `button`, `value: f32` in `[0.0, 1.0]` | cancels any pending release for that button and writes `value`; nothing releases it, it stays until set again | `{gamepad, button, value}` |
| `set_gamepad_axis` | `gamepad`, `axis: GamepadAxis`, `value` in `[-1.0, 1.0]` | writes `value`, which stays until set again; there are no timed axes | `{gamepad, axis, value}` |
| `disconnect_gamepad` | `gamepad` | cancels every pending release for the pad, removes `SimulatedGamepad`, writes `GamepadConnection::Disconnected` | `{gamepad}` |

**Writing both streams.** `write_raw_gamepad_event<T>(world, event) where T: Clone + Message,
RawGamepadEvent: From<T>` writes the event in its `RawGamepadEvent` form and as its own typed
message:
- connection: `GamepadConnectionEvent` and `RawGamepadEvent::Connection`
- button: `RawGamepadButtonChangedEvent` and `RawGamepadEvent::Button`
- axis: `RawGamepadAxisChangedEvent` and `RawGamepadEvent::Axis`

**Data flow.** `bevy_remote` runs handlers in `RemoteLast`. Their messages are consumed in the next
frame's `PreUpdate`, inside `InputSystems`:
- `gamepad_connection_system` reads `GamepadConnectionEvent`. On connect it inserts `Name` and
  `Gamepad`. On disconnect it removes `Gamepad` and keeps the entity alive.
- `gamepad_event_processing_system` runs after it and reads `RawGamepadEvent`. It applies the pad's
  `GamepadSettings` filters, updates `Gamepad`, and writes the processed
  `GamepadButtonStateChangedEvent`, `GamepadButtonChangedEvent`, `GamepadAxisChangedEvent` and
  `GamepadEvent`.

**Timed release.**
- `TimedGamepadButtonRelease { gamepad: Entity, button: GamepadButton, timer: Timer }`
  (`TimerMode::Once`) is spawned on its own entity.
- `GamepadPlugin` schedules `process_timed_gamepad_button_releases` in `PreUpdate`
  `.before(InputSystems)`.
- Each frame the system ticks every timer by `Res<Time<Real>>::delta()`, then looks up the pad in
  `Query<Option<&Gamepad>, With<SimulatedGamepad>>`:
  - `Err` (no marker): despawn the release.
  - `Ok(None)` (marker present, Bevy has not inserted `Gamepad` yet, e.g. a tap sent in the connect
    frame): wait.
  - `Ok(Some(pad))`, once the timer is finished **and** `pad.pressed(button)`: write
    `RawGamepadButtonChangedEvent { value: 0.0 }` to both `MessageWriter<RawGamepadEvent>` and
    `MessageWriter<RawGamepadButtonChangedEvent>`, then despawn.
- `cancel_pending_button_releases(world, gamepad, ReleaseCancellation)` despawns pending releases.
  `ReleaseCancellation::Button(b)` is used by send and set; `ReleaseCancellation::AllButtons` by
  disconnect.

**Shared request module.** The crate-private `extras/src/brp_request.rs` serves every mouse,
keyboard and gamepad handler. Call sites `use crate::brp_request;` and import only
`EmptyParamsPolicy` by name.
- `enum EmptyParamsPolicy { Allow, Reject }`. `Allow` parses `None` params as `{}` and is used only
  by `connect_gamepad` and `double_tap_gesture`.
- `parse_request<T: DeserializeOwned>(params: Option<Value>, policy) -> Result<T, BrpError>`. Under
  `Reject`, missing params give `MISSING_REQUEST_PARAMETERS_MESSAGE` ("Missing request
  parameters"). Serde errors give `"Failed to parse parameters: {e}"`.
- `const fn invalid_params(message: String) -> BrpError` builds `INVALID_PARAMS` with `data: None`.
- `serialize_response<T: Serialize>(response, handler_name) -> BrpResult` logs `warn!` and returns
  `INTERNAL_ERROR` if serialization fails.

**MCP side.**
- Tools `brp_extras_connect_gamepad`, `brp_extras_send_gamepad_button`,
  `brp_extras_set_gamepad_button`, `brp_extras_set_gamepad_axis` and `brp_extras_disconnect_gamepad`
  are `ToolName` variants in `mcp/src/tool/name.rs`, each with `brp_method`, params and result
  attributes. Every params struct carries `port: Port` (default 15702).
- Annotations: connect and send are `AdditiveNonIdempotent`; set button and set axis are
  `AdditiveIdempotent`; disconnect is `DestructiveIdempotent`.
- `mcp/src/brp_tools/gamepad.rs` defines `GamepadButtonWrapper` (19 named buttons) and
  `GamepadAxisWrapper` (6 named axes). They type the `button` and `axis` params, and their variant
  names are Bevy's serde names. Bevy's `Other(u8)` is not offered. An unknown name fails at the MCP
  boundary with serde's "unknown variant … expected one of …" list.
- `SendGamepadButtonParams` has no `value` field. Its `duration_ms` is skipped when `None`, so the
  extras default applies.
- Enum listing happens in `build_parameters_from` (`mcp/src/tool/parameters.rs`):
  - It resolves a field's `$ref` into `$defs`.
  - It takes the field's `description` from the property first (the struct field's doc comment),
    then from the resolved def, then from the field name.
  - For string params it passes `string_enum_values(resolved)` to `add_string_property`, which
    writes `"enum": [...]`. Two schema shapes qualify: a `oneOf` of string `const`s (enums with
    per-variant docs), or `{type: "string", enum: [...]}`. A `oneOf` that is not all string consts
    yields no list.
  - `MouseButtonWrapper` and both gamepad wrappers get their lists this way, and tests assert them.
- `#[tool_description(path = "../../help_text")]` pulls in `mcp/help_text/<tool_name>.txt` for each
  tool. A missing file fails the build.

**Key files**
- `extras/src/gamepad.rs`: types, `GamepadPlugin`, `remote_methods`, handlers, the release system,
  helpers (`write_raw_gamepad_event`, `cancel_pending_button_releases`, `simulated_gamepad`) and the
  inline `mod tests`.
- `extras/src/brp_request.rs`: shared parse, error and serialize helpers.
- `extras/src/constants.rs`: the `METHOD_*_GAMEPAD*` names and the "gamepad constants" block
  (`DEFAULT_GAMEPAD_DURATION_MS`, `MAX_GAMEPAD_DURATION_MS`, `SIMULATED_GAMEPAD_NAME`). Keyboard and
  mouse keep their duration constants in module-local `constants.rs` files.
- `extras/src/plugin.rs`: plugin and method registration. `extras/src/lib.rs`: the `## Gamepad`
  crate docs, `mod brp_request;` and the cfg-gated `mod gamepad;`.
- `mcp/src/brp_tools/gamepad.rs`: the wrappers.
  `mcp/src/brp_tools/tools/brp_extras_{connect_gamepad,disconnect_gamepad,send_gamepad_button,set_gamepad_button,set_gamepad_axis}.rs`:
  params and results.
- `mcp/src/tool/name.rs`: registry, annotations, parameter builders, handlers.
  `mcp/src/tool/parameters.rs`: schema flattening and enum listing.
- `mcp/help_text/brp_extras_*gamepad*.txt`: per-tool help, including the tap vs hold split.
- `test-app/examples/extras_plugin.rs`: `GamepadInputHistory` (`pressed_buttons` across all pads,
  `last_released`, `last_axis`, `last_axis_value`), filled by `track_gamepad_input` from Bevy's
  processed messages.
- `.claude/integration_tests/extras_gamepad.md`: the end-to-end spec, registered in
  `.claude/config/integration_tests.json`; the gamepad tools are listed in
  `.claude/agents/integration-tester.md`. It covers connect, hold and release, timed release, axis,
  five error cases, set cancelling a timed release, and disconnect with rejection.

## Invariants

- **Feed the pad exactly what `bevy_gilrs` writes** (0.19.1, `gilrs_system.rs`). Every connection,
  button and axis change, timed releases included, goes to both `RawGamepadEvent` and its typed
  message. A new write path uses `write_raw_gamepad_event` or writes both streams. On a Bevy upgrade,
  re-check what `bevy_gilrs` writes.
- **Only entities carrying `SimulatedGamepad` are accepted.** A real pad is never driven. Disconnect
  removes the marker, so a disconnected entity is rejected from then on. A new `connect` always
  makes a new entity.
- **Timed releases tick on `Time<Real>`.**
- **A release is written only after `Gamepad::pressed` reports the button down,** so a timed press,
  even at 0 ms, is seen down for at least one frame. The release system stays in `PreUpdate`
  `.before(InputSystems)`.
- **Any new value written to a button cancels that button's pending release, and disconnect cancels
  all of the pad's releases.** A new path that writes a button value must do the same; otherwise a
  stale release fires later and writes 0.0 over the new value.
- **"Send" is brief, "set" persists.** Send defaults to 100 ms, allows up to 60 000 ms and always
  returns `duration_ms`. Set and axis values stay until changed. Keyboard and mouse use the same
  100 / 60 000 numbers.
- **Mouse, keyboard and gamepad handlers parse, build `INVALID_PARAMS` errors and serialize only
  through `brp_request`.** No local copies. Use `EmptyParamsPolicy::Allow` only when every param is
  optional. `screenshot/request.rs` and `window_title.rs` build their own `INVALID_PARAMS` errors
  with their own messages; they are not copies of these helpers.
- **Error texts asserted by unit tests or the spec:**
  - "Entity {bits} is not a connected simulated gamepad (never connected, or disconnected); connect
    one with brp_extras/connect_gamepad"
  - "Button value {v} is outside [0.0, 1.0]"
  - "Axis value {v} is outside [-1.0, 1.0]"
  - "Duration exceeds maximum: {d}ms > 60000ms"
- **Module surface.** Handlers, `SimulatedGamepad`, `TimedGamepadButtonRelease` and
  `ReleaseCancellation` are private. `GamepadPlugin` is `pub(super)` and `remote_methods` is
  `pub(crate)`. Adding a method changes the `[_; 5]` array length.
- **MCP wrapper names match Bevy's `GamepadButton` / `GamepadAxis` serde names,** because MCP
  forwards them unchanged. Every MCP tool needs its help text file.

## Calibration and gotchas

- **Frame timing.** A handler's change shows up from the next frame's `PreUpdate`. A connected pad is
  a `Gamepad` from the next frame, so any per-pad state the app spawns arrives one frame after that.
- **Tap timing.** A 0 ms tap is down for exactly one frame and released the next. A default tap
  measured on the real clock, starting after the press frame, is still down at 99 ms and up at
  100 ms.
- **Partial set, then tap.** Bevy's default `ButtonSettings` press at 0.75 and release at 0.65. A set
  value between those thresholds does not change the digital state, and 0.5 is analog only. The
  release gate uses `Gamepad::pressed` (digital), not the analog value: after a partial set below the
  press threshold, an analog gate would let a 0 ms tap release before its press was ever seen.
- **Tapping a held button.** The gate checks the button's current state, not whether this tap's
  press has landed. Tapping a button already held by `set` releases it once the timer finishes,
  possibly in the same frame the tap's press is processed.
- **Axis dead zone.** Bevy's default dead zone rescales axis values: -0.75 arrives as about -0.737,
  and the spec accepts -0.8 to -0.7. A raw value of -1.0 reads back as exactly -1.0.
- **Lingering entities.** Disconnect leaves the entity alive with `Gamepad` removed, as Bevy does for
  real pads. Each connect/disconnect cycle leaves one such entity.
- **Reading pad state.** `Gamepad` does not serialize over BRP (its button maps have non-string
  keys), so state must be read through an app-side resource. The test-app uses
  `GamepadInputHistory`. That resource is shared across every pad, including the test-app's own
  `Gamepad` test entities, so a rerun after a run that left a button held needs a fresh app launch.
- **MCP call ordering.** Two MCP calls reach the app about 640 ms apart even in one batch, and calls
  in one batch can reach the app in either order. Order-dependent calls are issued one at a time,
  use windows of seconds, and check the outcome after the window. The spec therefore sends West for
  5000 ms, makes `set_gamepad_button` on West the next call on its own, and opens its cancel check
  by making sure at least 6 seconds have passed since the West send.
- **Unit test harness.**
  - Tests read `Messages<…>` with `MessageCursor::default()`, because the paused virtual clock leaves
    message buffers unswapped.
  - The first `app.update()` records the start instant without advancing `Time<Real>`.
  - The toolchain's clippy (rust 1.99) denies `clippy::assert_is_empty`, so empty checks are written
    `assert_eq!(x, Vec::<T>::new())`.
- **Keyboard parse errors** read "Failed to parse parameters: {e}". No test asserts that wording.
- **Known limit: the `integration-tester` agent type.** Claude Code loads it from the checkout the
  session was launched in. A session launched in another checkout, one whose tester lacks the
  gamepad tools, cannot run this spec through it. Run the spec through a general-purpose agent given
  this checkout's `.claude/agents/integration-tester.md` body plus the DedicatedAppPrompt from
  `.claude/commands/integration_tests.md`, against an MCP binary built from this checkout.
- **Testing MCP changes from a worktree.**
  - Build with `cargo build -p bevy_brp_mcp --bin bevy_brp_mcp`.
  - Start a session with `--strict-mcp-config --mcp-config <file>`, where the file is a copy of the
    global `brp` entry with `command` pointed at the worktree's `target/debug/bevy_brp_mcp`. Keep the
    server name `brp` so tools stay `mcp__brp__*`.
  - Never `cargo install --path mcp` for this: the global binary serves other sessions.
- **Do not run `/integration_tests` for this spec on a shared machine.** Its
  `cleanup_stale_test_processes.sh` runs `pkill -x extras_plugin` against every session's app, and it
  takes ports from the 20100 pool. Run the DedicatedAppPrompt by hand on a dedicated port, with
  `brp_launch` `path` set to the worktree.

## Why it is this way

- **Tap vs hold split.** One convention for the whole input API: "send" is a brief, self-releasing
  tap, matching `send_keys` and `send_mouse_button`, and "set" keeps its value until changed. A tap
  cannot leave a button stuck if the caller never sends a release. Set covers long holds and analog
  pressure.
- **Real-clock timer.** Apps pause `Time<Virtual>`, for example on a pause menu. A tap delivered
  while paused must still come back up. Keyboard and mouse holds already work this way.
- **Gating the release on `Gamepad::pressed`.** A release written in the same frame as its press
  would be processed in the same `gamepad_event_processing_system` run, and nothing downstream would
  ever see the button down. Gating on the digital state rather than the analog value covers the
  partial-set case above. The `Ok(None)` wait lets a tap sent in the connect frame, before Bevy
  inserts `Gamepad`, still release.
- **Removing the marker on disconnect.** Bevy's processing skips entities without `Gamepad`, so
  input sent to a disconnected pad would be dropped without any error. Removing the marker turns it
  into a clear `INVALID_PARAMS` error instead.
- **Private handlers.** The handlers are reached only through `remote_methods`, so the module's
  surface is `GamepadPlugin` plus its method table, and no other crate code can mark or drive a pad.
  Handler tests therefore live in the inline `mod tests` and call handlers directly with
  `In(Some(json))`, since `extras/tests/` sees only public API.
- **Writing both raw and typed messages.** This matches `bevy_gilrs` exactly. Bevy's
  `gamepad_connection_system` reads the typed `GamepadConnectionEvent`, which is what inserts
  `Gamepad`; `gamepad_event_processing_system` reads `RawGamepadEvent`. Nothing in Bevy 0.19.1 reads
  the typed `RawGamepadButtonChangedEvent` / `RawGamepadAxisChangedEvent` streams; they are written
  so any consumer of them sees the simulated pad as it would a real one.
- **One shared request module.** Mouse, keyboard and gamepad each had their own copy of the parse,
  error and serialize logic. One module keeps behavior and messages consistent.
- **Typed MCP wrappers with listed enums.** Agents see the valid names in the tool schema, and typos
  fail before reaching the app. The raw BRP method still accepts whatever Bevy's serde form accepts.
