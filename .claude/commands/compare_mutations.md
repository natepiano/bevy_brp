# Compare Mutation Paths

This command compares mutation path discovery between different versions of the bevy_brp_mcp tool to ensure consistency and detect regressions or improvements.

## Quick Usage

Run the quick comparison:
```bash
.claude/commands/scripts/quick_compare.sh
```

## Detailed Comparison

Compare any two files:
```bash
python3 .claude/commands/scripts/compare_mutations.py <old_file> <new_file>
```

Check a specific type across all versions:
```bash
python3 .claude/commands/scripts/check_type.py "bevy_transform::components::transform::Transform"
```

Summarize test results from any JSON file:
```bash
.claude/commands/scripts/summarize_results.sh <json_file>
```

## File Management Process

### 1. Establishing a Baseline (Known Good Version)
When you have a confirmed good version of `all_types.json`:
```bash
# Save as the baseline for future comparisons
cp $TMPDIR/all_types.json $TMPDIR/all_types_baseline.json

# Also create a timestamped backup
cp $TMPDIR/all_types.json $TMPDIR/all_types_$(date +%Y%m%d_%H%M%S).json
```

### 2. Before Generating a New Version
```bash
# Save the current version before overwriting
cp $TMPDIR/all_types.json $TMPDIR/all_types_previous.json
```

### 3. After Generating a New Version
```bash
# The new version is in $TMPDIR/all_types.json
# Run comparison against baseline
python3 .claude/commands/scripts/compare_mutations.py $TMPDIR/all_types_baseline.json $TMPDIR/all_types.json

# Also compare against previous run
python3 .claude/commands/scripts/compare_mutations.py $TMPDIR/all_types_previous.json $TMPDIR/all_types.json
```

## What to Look For

### Expected Changes
- Minor formatting differences (e.g., `[0]` → `.0` for array accessors)
- New mutation paths when types gain reflection support
- Removed paths for types that become non-mutable

### Concerning Changes  
- Large-scale removal of previously working paths
- Fundamental types (Transform, Sprite) losing basic paths
- Inconsistent path formats within the same type

## Validating and Marking a Version as Good

After comparing a new version:

1. **Review the comparison results** - Look for concerning changes listed above
2. **Ask the user for validation**:
   ```
   "The comparison shows [X differences/no differences]. 
   Should I mark this version as the new good baseline?"
   ```
3. **If user confirms it's good**, establish it as the baseline:
   ```bash
   # Mark current version as the good baseline
   cp $TMPDIR/all_types.json $TMPDIR/all_types_baseline.json
   
   # Also create a timestamped backup for historical reference
   cp $TMPDIR/all_types.json $TMPDIR/all_types_good_$(date +%Y%m%d_%H%M%S).json
   
   echo "✅ Version marked as good baseline"
   ```

## Key Files

### Data Files (in $TMPDIR)
- `all_types.json` - Current/latest generated file
- `all_types_baseline.json` - Known good baseline version
- `all_types_previous.json` - Previous run for incremental comparison

### Scripts (in .claude/commands/scripts/)
- `compare_mutations.py` - Main comparison script
- `check_type.py` - Check specific type across versions
- `quick_compare.sh` - Quick comparison wrapper
- `summarize_results.sh` - Summarize test results from JSON files

## Examples

### Full comparison suite
```bash
.claude/commands/scripts/quick_compare.sh
```

### Compare specific files
```bash
python3 .claude/commands/scripts/compare_mutations.py $TMPDIR/all_types_baseline.json $TMPDIR/all_types.json
```

### Check if Transform changed
```bash
python3 .claude/commands/scripts/check_type.py "bevy_transform::components::transform::Transform"
```

### Check test components
```bash
python3 .claude/commands/scripts/check_type.py "extras_plugin::TestMapComponent"
python3 .claude/commands/scripts/check_type.py "extras_plugin::TestArrayField"
```

### Summarize batch results
```bash
.claude/commands/scripts/summarize_results.sh $TMPDIR/batch_results_1.json
.claude/commands/scripts/summarize_results.sh $TMPDIR/all_types.json
```

## Setting a New Baseline

This is now covered in the "Validating and Marking a Version as Good" section above.
Always ask the user before marking a version as the baseline.