# Reset Mutation Test

Reset all mutation test metadata to initial state, clearing all batch numbers, test results, and failure reasons.

## What This Does

1. Applies auto-pass logic to determine which types should be automatically marked as "passed":
   - Types with no mutation paths
   - Types with only root path that is not_mutable
   - Types with only root path with no examples
2. Marks all other types as "untested"
3. Clears all batch_number assignments (sets to null)
4. Clears all fail_reason fields

## Execution

```bash
python3 .claude/scripts/mutation_test/initialize_test_metadata.py --file .claude/transient/all_types.json --reset-all
```

## After Reset

The mutation test can be run from the beginning, processing all untested types in sequential batches.
