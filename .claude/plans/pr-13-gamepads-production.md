# Production — pr-13-gamepads

> **Status: PRODUCTION — running.** Maintainer fixes on top of PR #13 (simulated gamepads, by johanhelsing), landed as one green PR.

## Production Context

- **Source plans:** `.claude/plans/pr-13-gamepads.md` — split on 2026-10-06 by the showrunner (one unit, promoted from the review session)
- **Repository:** `/home/natepiano/rust/bevy_brp`
- **Merge branch:** `prod/pr-13-gamepads` — every unit merges here; only the showrunner pushes it. It starts as `main` (`7c1b6d94`) with the PR head `37f0223d` merged in (`b54ba886`).
- **Showrunner checkout:** `/home/natepiano/rust/bevy_brp`, on the merge branch
- **Showrunner session:** bevy_brp
- **Log:** `.claude/plans/pr-13-gamepads-production.log` — git-excluded; one line per event
- **User zone:** America/Los_Angeles — every time the showrunner reports is in this zone only, never UTC (user, 2026-10-02)
- **Updates:** every 15 minutes; each update reports every unit in full
- **Merge tests:** `bevy_brp_extras`, `bevy_brp_mcp`
- **Capacity:** 32 cores, 60 GB memory (about 28 GB free, shared with the hana productions); 1 unit

## Units

| Unit | Plan | Worktree | Branch | Session | Port | Owns |
| --- | --- | --- | --- | --- | --- | --- |
| gamepad-unit | `.claude/plans/pr-13-gamepads-gamepad-unit.md` | `/home/natepiano/rust/bevy_brp_gamepad` | `unit/gamepad` | tmux `bevy_brp-pr-evaluation`; Claude and remote-control name `bevy_brp pr evaluation` | 20250 | the whole workspace (only unit) |

## Hub files

| File | Owner unit | Other units that touch it |
| --- | --- | --- |
| — | — | — |

## Gates

| Gate | Waiting | Waits on | Clears when |
| --- | --- | --- | --- |
| — | — | — | — |

## Close-out

- With the user's OK: push `prod/pr-13-gamepads` to the contributor's PR branch `johanhelsing:simulated-gamepad` (maintainer edit, a fast-forward of `37f0223d`, never force). The contributor sees it.
- Wait for the PR's CI to pass on that push.
- The user merges PR #13, or tells the showrunner to.
- Switch `/home/natepiano/rust/bevy_brp` back to `main` and fast-forward it to the merged PR.
- With the user's OK: delete the local review branch `pr-13-review`.

## Production rules

- The contributor's commit `37f0223d` is never rewritten; every fix goes on top as maintainer commits. Source: the review plan of 2026-10-04, after the user ruled out a back-and-forth with the contributor.
- `<PromoteMain/>` does not run: `main` gets this work only through PR #13's merge, so the PR records it. The smoke launch and Mac run scripts build hana and do not apply here. Showrunner call, 2026-10-06.
- Never `cargo install --path mcp`: the global `~/.cargo/bin/bevy_brp_mcp` serves other sessions and productions. The end-to-end phase runs a binary built in the unit's worktree, through a strict MCP config the showrunner relaunches the unit director with. Showrunner call, 2026-10-06.
