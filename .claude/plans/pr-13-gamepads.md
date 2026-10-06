# PR #13 — simulated gamepads: maintainer fixes

Source: the review of [natepiano/bevy_brp#13](https://github.com/natepiano/bevy_brp/pull/13)
("Add simulated gamepads", by johanhelsing, head `37f0223d` on
`johanhelsing:simulated-gamepad`, base `main`), made 2026-10-04 in the session
"bevy_brp pr evaluation". The user did not want a review back-and-forth with the
contributor ("there are way more issues than i want to deal with in a back and forth"),
so the fixes land as maintainer commits on top of the contributor's commit. The PR allows
maintainer edits. The contributor's commit stays as written.

The work starts from `prod/pr-13-gamepads`, which is current `main` with the PR head
merged in (`b54ba886`). The PR is based on `e5075db7`; `main` has 4 newer commits, none in
the PR's files.

## Why the approach is kept

- It feeds input in where real hardware does: it writes the same messages `bevy_gilrs`
  writes, so the `Gamepad` component, dead zones, settings, processed events and input
  managers all run as for a real pad.
- A marker component limits the methods to simulated pads, so it never fights a real pad.
- Timed releases run on the real clock (the same fix keyboard and mouse got), and a
  release is held back until Bevy has registered the press, so a timed press always shows
  as down for at least one frame. Both have tests.
- It carries over to Bevy 0.20: the connection messages match `bevy_gilrs`, and Bevy's
  gamepad systems are identical in 0.19.1 and 0.20.0-rc.2.

## Fixes

1. **Button default matches mouse and keyboard.** `send_mouse_button` and `send_keys`
   default to a 100 ms tap; `send_gamepad_button` holds the button down indefinitely
   (`extras/src/gamepad.rs` ~220), so an agent that learned the mouse tool leaves gamepad
   buttons stuck down. Change: `send_gamepad_button` taps for 100 ms by default, like the
   mouse tool; a new `set_gamepad_button` keeps its value until changed, matching
   `set_gamepad_axis`. One rule for the whole API: "send" is brief, "set" stays. This is a
   maintainer call on an unreleased API; easy to undo.
2. **Both message streams.** `bevy_gilrs` writes each change to `RawGamepadEvent` and also
   to `RawGamepadButtonChangedEvent` / `RawGamepadAxisChangedEvent`; the PR writes only the
   first (`gamepad.rs` ~257, ~287). Write both, as the mouse code already does for its two
   streams.
3. **Disconnected pads reject input.** Disconnect leaves the marker on the entity, so later
   button and axis calls report success while Bevy drops the input (no `Gamepad`
   component). Remove the marker on disconnect so later calls fail with a "disconnected"
   error, and drop the pad's pending releases. Relabel the disconnect tool
   `DestructiveIdempotent` (`mcp/src/tool/name.rs` ~647), as despawn, remove and shutdown
   are.
4. **Typed `button` and `axis` parameters.** The tool parameters accept any string
   (`brp_extras_send_gamepad_button.rs` ~19, `brp_extras_set_gamepad_axis.rs` ~19). Use
   enums, as the mouse tool's `MouseButtonWrapper` does, so the 19 buttons and 6 axes appear
   in the tool schema the agent sees.
5. **Mend.** CI's Mend Check fails on five over-visible `pub(crate)` items and one inline
   `gamepad::GamepadPlugin` path; `cargo mend --fix` fixes all six.
6. **Docs and tests follow.** Help text, crate docs, READMEs, CHANGELOGs, unit tests and the
   integration spec (`.claude/integration_tests/extras_gamepad.md`) match the changes. New
   tests: a new value cancels a pending release; disconnect drops pending releases.

## Cleanup (separate commit)

The request helpers `parse_request`, `invalid_params` and `to_value` are copied in mouse
(`mouse/support.rs`), keyboard and now gamepad. Move them into one shared crate-level
module in `extras`. It touches code outside the PR's feature, so it is its own commit.

## End-to-end check

Run the PR's integration spec `extras_gamepad` (never yet run through the agent runner)
against an MCP binary built from this branch. Never `cargo install --path mcp`: the global
`~/.cargo/bin/bevy_brp_mcp` serves other sessions and productions. The unit director asks
the showrunner to relaunch it with a strict MCP config naming its worktree's built binary as
the `brp` server.

## Out of scope

- Porting to the Bevy 0.20 RC branch: it reaches `upd/bevy-release-candidate` through `main`.
- The contributor's note that two `agent_tools::registration` tests fail on `main`: CI
  passes them and the review did not reproduce it.
