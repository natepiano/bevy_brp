#!/bin/bash
# Validates that the user is in the correct bevy_brp root directory
# Used by create_mutation_test_json.md command

if [[ ! -f ".claude/commands/create_mutation_test_json.md" ]]; then
    echo "❌ Not in bevy_brp root directory. Please cd to the project root."
    exit 1
fi

echo "✅ Confirmed in bevy_brp root directory"