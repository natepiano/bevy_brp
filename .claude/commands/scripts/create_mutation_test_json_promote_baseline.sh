#!/bin/bash

# Promote current mutation test file to baseline
# Usage: create_mutation_test_json_promote_baseline.sh

set -e

# Mark current version as the good baseline
cp $TMPDIR/all_types.json $TMPDIR/all_types_baseline.json

# Create timestamped backup
cp $TMPDIR/all_types.json $TMPDIR/all_types_good_$(date +%Y%m%d_%H%M%S).json

# Update reference JSON files with new baseline
./.claude/commands/scripts/create_mutation_test_json_update_reference.sh

echo "✅ Version marked as good baseline and reference files updated"