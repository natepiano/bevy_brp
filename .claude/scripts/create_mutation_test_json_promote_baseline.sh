#!/bin/bash

# Promote current mutation test file to baseline
# Usage: create_mutation_test_json_promote_baseline.sh

set -e

# Mark current version as the good baseline
cp .claude/types/all_types.json .claude/types/all_types_baseline.json

# Create timestamped backup
cp .claude/types/all_types.json .claude/types/all_types_good_$(date +%Y%m%d_%H%M%S).json

echo "✅ Version marked as good baseline"