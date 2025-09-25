#!/bin/bash
# Compare a specific mutation path between baseline and current
# Usage: compare_mutation_path.sh "TypeName" "mutation.path"

TYPE="$1"
PATH="$2"

if [[ -z "$TYPE" ]] || [[ -z "$PATH" ]]; then
    echo "ERROR: Usage: compare_mutation_path.sh \"TypeName\" \"mutation.path\""
    exit 1
fi

# Extract data fields
.claude/scripts/get_mutation_path.sh "$TYPE" "$PATH" .claude/transient/all_types_baseline.json | jq '.data' > /tmp/mp_baseline.json
.claude/scripts/get_mutation_path.sh "$TYPE" "$PATH" .claude/transient/all_types.json | jq '.data' > /tmp/mp_current.json

# Check if different
if cmp -s /tmp/mp_baseline.json /tmp/mp_current.json; then
    echo "IDENTICAL"
else
    echo "DIFFERENT"
    echo "=== BASELINE ==="
    cat /tmp/mp_baseline.json
    echo "=== CURRENT ==="
    cat /tmp/mp_current.json
fi