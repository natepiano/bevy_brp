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

echo "✅ Version marked as good baseline"
echo "📝 Baseline promoted at: ${TIMESTAMP}"
