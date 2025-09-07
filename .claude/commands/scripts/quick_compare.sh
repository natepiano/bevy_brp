#!/bin/bash
# Quick comparison wrapper script

TMPDIR="${TMPDIR:-/var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T}"

echo "🔍 Mutation Path Quick Comparison Tool"
echo "======================================"
echo

# Check if baseline exists
if [ ! -f "$TMPDIR/all_types_baseline.json" ]; then
    echo "⚠️  No baseline found. Creating one from latest backup..."
    if [ -f "$TMPDIR/all_types_latest_backup.json" ]; then
        cp "$TMPDIR/all_types_latest_backup.json" "$TMPDIR/all_types_baseline.json"
        echo "✅ Baseline created from all_types_latest_backup.json"
    else
        echo "❌ No suitable file to create baseline. Run after generating all_types.json"
        exit 1
    fi
fi

# Check if current file exists
if [ ! -f "$TMPDIR/all_types.json" ]; then
    echo "❌ No current all_types.json found"
    exit 1
fi

# Compare against baseline
echo "📊 Comparing current vs baseline..."
echo
python3 "$(dirname "$0")/compare_mutations.py" "$TMPDIR/all_types_baseline.json" "$TMPDIR/all_types.json"

# If previous exists, also compare incremental
if [ -f "$TMPDIR/all_types_previous.json" ]; then
    echo
    echo "📊 Comparing current vs previous run..."
    echo
    python3 "$(dirname "$0")/compare_mutations.py" "$TMPDIR/all_types_previous.json" "$TMPDIR/all_types.json"
fi

echo
echo "💡 Tip: To set a new baseline, run:"
echo "   cp $TMPDIR/all_types.json $TMPDIR/all_types_baseline.json"
