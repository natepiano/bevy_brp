# Integration Test Review TODO

Last updated: 2026-02-25
Branch: pr-5

- [x] 1. Fix prebuild command so build failures cannot be hidden by `| tail -5`.
  - Implemented strict prebuild script: `.claude/scripts/integration_tests/prebuild_workspace.sh`.
  - Updated `.claude/commands/integration_tests.md` to call the script.
- [x] 2. Resolve `type_guide` test conflict with app-managed runner ownership and dynamic ports.
  - Updated `.claude/integration_tests/type_guide.md` to runner-managed app context (no self-launch/shutdown).
  - Replaced hardcoded `20114` with `{{PORT}}` in all BRP call examples.
- [ ] 3. Resolve self-managed template conflict with `wasm` test script-based execution.
- [x] 4. Remove hardcoded `20115` from `get.md` so it follows assigned dynamic port.
  - Replaced all `port: 20115` entries in `.claude/integration_tests/get.md` with `port: {{PORT}}`.
- [x] 5. Replace or soften stale cleanup (`pkill -9`) to reduce collateral process kills.
  - Added `.claude/scripts/integration_tests/cleanup_stale_test_processes.sh` (config-driven app name extraction).
  - Updated `.claude/commands/integration_tests.md` cleanup steps to call the script with `${TEST_CONFIG_FILE}`.
  - Cleanup behavior now does `SIGTERM` first, then `SIGKILL` for remaining stale processes.
- [x] 6. Make objective extraction robust for multi-line or spaced `## Objective` sections.
  - Updated `.claude/scripts/integration_tests/extract_test_objectives.sh` to parse objective sections via `awk`.
  - Handles blank lines after heading, multi-line objective paragraphs, `## Objectives`, and `## Objective:` inline heading forms.
  - Returns one output line per file and emits `N/A` when objective is missing or empty.
