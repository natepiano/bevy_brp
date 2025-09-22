#!/bin/bash

# Promote current mutation test file to baseline with expected changes reset
# Usage: create_mutation_test_json_promote_baseline.sh

set -e

# Safety check - ensure we're in the right directory
if [[ ! -f ".claude/commands/create_mutation_test_json.md" ]]; then
    echo "❌ Not in bevy_brp root directory. Please cd to the project root."
    exit 1
fi

# Create timestamp for backups
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Backup current expected changes before reset
if [[ -f ".claude/transient/create_mutation_test_json_expected_changes.json" ]]; then
    # Ensure archive directory exists
    mkdir -p .claude/transient/archive
    cp .claude/transient/create_mutation_test_json_expected_changes.json \
       .claude/transient/archive/expected_changes_${TIMESTAMP}.json
    echo "📦 Archived current expected changes to: .claude/transient/archive/expected_changes_${TIMESTAMP}.json"
fi

# Mark current version as the good baseline
cp .claude/transient/all_types.json .claude/transient/all_types_baseline.json

# Create timestamped backup of the promoted baseline
cp .claude/transient/all_types.json .claude/transient/all_types_good_${TIMESTAMP}.json

# Reset expected changes file to template state
# ID 0 is reserved as an example that comparison scripts will ignore
cat > .claude/transient/create_mutation_test_json_expected_changes.json << 'EOF'
{
  "expected_changes": [
    {
      "id": 0,
      "name": "EXAMPLE ENTRY - Ignored by comparison scripts",
      "pattern_type": "EXAMPLE",
      "pattern_match": {
        "description": "This is a template entry showing the expected structure",
        "field": "example_field",
        "min_occurrences": 1,
        "min_types_affected": 1,
        "exact_types_affected": null,
        "description_contains": null
      },
      "description": "This entry (id: 0) is ignored by comparison scripts and serves as documentation for the expected_changes structure. When actual changes are detected after promotion, add them with id >= 1"
    }
  ]
}
EOF

echo "✅ Version marked as good baseline"
echo "✅ Expected changes file reset to template"
echo "📝 Baseline promoted at: ${TIMESTAMP}"