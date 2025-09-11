#!/bin/bash
# create_mutation_test_json_update_reference.sh
# Updates reference JSON files with current type guides from all_types.json

set -e

REFERENCE_DIR=".claude/commands/reference_json"
ALL_TYPES_FILE="$TMPDIR/all_types.json"

if [ ! -f "$ALL_TYPES_FILE" ]; then
    echo "❌ all_types.json not found at $ALL_TYPES_FILE"
    exit 1
fi

echo "🔄 Updating reference JSON files..."
updated_count=0

for ref_file in "$REFERENCE_DIR"/*.json; do
    if [ -f "$ref_file" ]; then
        # Extract type name from reference file
        type_name=$(jq -r '.type_name // empty' "$ref_file")
        
        if [ -n "$type_name" ]; then
            # Extract type guide from all_types.json and write to reference file
            jq --arg type_name "$type_name" \
               '.type_guide[] | select(.type_name == $type_name)' \
               "$ALL_TYPES_FILE" > "$ref_file"
            
            if [ $? -eq 0 ]; then
                echo "✅ Updated $(basename "$ref_file") for type: $type_name"
                ((updated_count++))
            else
                echo "❌ Failed to update $(basename "$ref_file")"
            fi
        else
            echo "⚠️  No type_name found in $(basename "$ref_file")"
        fi
    fi
done

echo "📊 Updated $updated_count reference files"