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

**Step 1**: `/create_mutation_test_json` - discovers types, determines spawn support and mutation paths, creates `.claude/transient/all_types.json`

**Step 2**: `/mutation_test` - tests spawn/insert and every mutation path in parallel batches, stops on first failure

## Configuration
- `.claude/config/test_config.json` - integration test config
- `.claude/config/mutation_test_known_issues.json` - mutation test exclusions
- `.claude/transient/all_types_baseline.json` - baseline comparison data
