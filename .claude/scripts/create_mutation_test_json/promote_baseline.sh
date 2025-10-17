#!/bin/bash

# Promote current mutation test file to baseline
# Usage: promote_baseline.sh

set -e

# Safety check - ensure we're in the right directory
if [[ ! -f ".claude/commands/create_mutation_test_json.md" ]]; then
    echo "❌ Not in bevy_brp root directory. Please cd to the project root."
    exit 1
fi

# Create timestamp for backups
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Mark current version as the good baseline
cp .claude/transient/all_types.json .claude/transient/all_types_baseline.json

# Create timestamped backup of the promoted baseline
cp .claude/transient/all_types.json .claude/transient/all_types_good_${TIMESTAMP}.json

# Clean up old backups, keeping only the 2 most recent
cd .claude/transient
# Find all good backups, sort by name (which is chronological due to timestamp format),
# skip the 2 newest, and delete the rest
ls -1 all_types_good_*.json 2>/dev/null | sort -r | tail -n +3 | while read -r old_file; do
    rm -f "${old_file}"
    echo "🗑️  Removed old backup: ${old_file}"
done
cd - > /dev/null

echo "✅ Version marked as good baseline"
echo "📝 Baseline promoted at: ${TIMESTAMP}"
