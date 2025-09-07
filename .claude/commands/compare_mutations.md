# Compare Mutation Paths

This command compares mutation path discovery between different versions of the bevy_brp_mcp tool to ensure consistency and detect regressions or improvements.

## Quick Usage

Run the quick comparison:
```bash
$TMPDIR/quick_compare.sh
```

## Detailed Comparison

Compare any two files:
```bash
python3 $TMPDIR/compare_mutations.py <old_file> <new_file>
```

Check a specific type across all versions:
```bash
python3 $TMPDIR/check_type.py "bevy_transform::components::transform::Transform"
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
python3 $TMPDIR/compare_mutations.py $TMPDIR/all_types_baseline.json $TMPDIR/all_types.json

# Also compare against previous run
python3 $TMPDIR/compare_mutations.py $TMPDIR/all_types_previous.json $TMPDIR/all_types.json
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

## Key Files

### Data Files (in $TMPDIR)
- `all_types.json` - Current/latest generated file
- `all_types_baseline.json` - Known good baseline version
- `all_types_previous.json` - Previous run for incremental comparison

### Scripts (in $TMPDIR)
- `compare_mutations.py` - Main comparison script
- `check_type.py` - Check specific type across versions
- `quick_compare.sh` - Quick comparison wrapper

## Examples

### Full comparison suite
```bash
$TMPDIR/quick_compare.sh
```

### Compare specific files
```bash
python3 $TMPDIR/compare_mutations.py $TMPDIR/all_types_baseline.json $TMPDIR/all_types.json
```

### Check if Transform changed
```bash
python3 $TMPDIR/check_type.py "bevy_transform::components::transform::Transform"
```

### Check test components
```bash
python3 $TMPDIR/check_type.py "extras_plugin::TestMapComponent"
python3 $TMPDIR/check_type.py "extras_plugin::TestArrayField"
```

## Setting a New Baseline

When you confirm the current version is good:
```bash
cp $TMPDIR/all_types.json $TMPDIR/all_types_baseline.json
echo "✅ New baseline set from current all_types.json"
```