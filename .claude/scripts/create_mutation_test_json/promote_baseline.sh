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
if [[ -f ".claude/config/create_mutation_test_json_expected_changes.json" ]]; then
    # Ensure archive directory exists
    mkdir -p .claude/transient/archive
    cp .claude/config/create_mutation_test_json_expected_changes.json \
       .claude/transient/archive/expected_changes_${TIMESTAMP}.json
    echo "📦 Archived current expected changes to: .claude/transient/archive/expected_changes_${TIMESTAMP}.json"
fi

# Mark current version as the good baseline
cp .claude/transient/all_types.json .claude/transient/all_types_baseline.json

# Create timestamped backup of the promoted baseline
cp .claude/transient/all_types.json .claude/transient/all_types_good_${TIMESTAMP}.json

# Reset expected changes file to template state
# Copy the ENUM_VARIANT_QUALIFIED_NAMES pattern from template as it's commonly needed
cp .claude/config/expected_changes_template.json temp_template.json
cat > .claude/config/create_mutation_test_json_expected_changes.json << 'EOF'
{
  "expected_changes": [
    {
      "id": 1,
      "pattern_type": "ENUM_VARIANT_QUALIFIED_NAMES",
      "description": "Enum variants changed from simple names to fully qualified names (e.g. 'Additive' -> 'BloomCompositeMode::Additive')",
      "change_type": "value_changed",
      "path_regex": "mutation_paths\\..*\\.examples\\[\\d+\\]\\.applicable_variants\\[\\d+\\]$",
      "value_condition": "current.endswith('::' + baseline) and '::' not in baseline and '::' in current"
    }
  ]
}
EOF
rm -f temp_template.json

echo "✅ Version marked as good baseline"
echo "✅ Expected changes file reset to template"
echo "📝 Baseline promoted at: ${TIMESTAMP}"