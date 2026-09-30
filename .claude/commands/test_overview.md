# BRP MCP Testing Overview

## Test Applications
- **Primary**: `test-app/examples/extras_plugin.rs` - used for integration and mutation tests
- **Additional**: `test-app/` apps/examples - used to validate listing and launching functionality

## Integration Tests - `/integration_tests`
Validates core BRP operations (spawn, insert, query, mutate, remove, watch, extras features).
Runs 12 tests in parallel with port isolation and automatic cleanup.

**Usage**:
- `/test` - run all tests
- `/test extras` - run single test

## Mutation Testing - Two-step process
Systematically validates ALL mutation paths for ALL registered component types.

**Step 1**: `/create_mutation_test_json` - `fetch_type_guides.py` writes every type guide to `.claude/transient/all_types.json`, then compares it against the baseline

**Step 2**: `/mutation_test` - `run.py` tests spawn/insert and every mutation path across all batches, then reports every failure

## Configuration
- `.claude/config/test_config.json` - integration test config
- `.claude/config/mutation_test_config.json` - mutation test ports and batch sizing
- `.claude/config/mutation_test_excluded_types.json` - mutation test exclusions
- `.claude/transient/all_types_baseline.json` - baseline comparison data
