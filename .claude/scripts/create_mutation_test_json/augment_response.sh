#!/bin/bash

# Augment full BRP response with test metadata
# This script preserves the ENTIRE BRP response structure while adding test tracking fields
# Usage: ./create_mutation_test_json_augment_response.sh [FILEPATH] [TARGET_FILE]

FILEPATH="$1"
TARGET_FILE="$2"

if [ -z "$FILEPATH" ] || [ -z "$TARGET_FILE" ]; then
    echo "Usage: $0 <source_json_file> <target_json_file>"
    exit 1
fi

if [ ! -f "$FILEPATH" ]; then
    echo "Error: Source file $FILEPATH does not exist"
    exit 1
fi

# Read excluded types list from JSON file
EXCLUSION_FILE=".claude/scripts/mutation_test_excluded_types.json"
EXCLUDED_TYPES=""
if [ -f "$EXCLUSION_FILE" ]; then
    # Extract type_name values from JSON and create pipe-separated regex
    EXCLUDED_TYPES=$(jq -r '.excluded_types[].type_name' "$EXCLUSION_FILE" 2>/dev/null | tr '\n' '|' | sed 's/|$//')
fi

# Create the augmented JSON using jq
# This preserves ALL fields from the original BRP response and adds test metadata
jq --arg excluded "$EXCLUDED_TYPES" '
# Process the response, preserving everything and adding test metadata
# type_guide is an object with type names as keys
.type_guide |= with_entries(
    . as $entry |
    # Skip excluded types entirely
    if ($excluded != "" and $entry.key != null and ($entry.key | test($excluded))) then
        empty
    else
        # Preserve the key (type name) and augment the value
        {
            key: $entry.key,
            value: ($entry.value + {
                # Add the type field from the key
                "type": $entry.key,
                # Add test tracking fields
                "batch_number": null,
                "test_status": (
                    # Auto-pass only types with no mutation paths or empty root-only paths
                    # Types with examples in root path should be tested
                    if ($entry.value.mutation_paths == null or $entry.value.mutation_paths == {}) then
                        "passed"
                    elif (($entry.value.mutation_paths | type == "object") and ($entry.value.mutation_paths | length == 1) and ($entry.value.mutation_paths | has(""))) then
                        # Check if root path has meaningful examples
                        if ($entry.value.mutation_paths[""].example == {} and
                            ($entry.value.mutation_paths[""].examples == null or $entry.value.mutation_paths[""].examples == [])) then
                            "passed"
                        else
                            "untested"
                        end
                    else
                        "untested"
                    end
                ),
                "fail_reason": ""
            })
        }
    end
)
' "$FILEPATH" > "$TARGET_FILE"

if [ $? -eq 0 ]; then
    echo "Successfully augmented BRP response to $TARGET_FILE"

    # Calculate comprehensive statistics about the augmented file
    STATS_JSON=$(jq -r '
        if .type_guide then
            .type_guide as $types
        elif .result.type_guide then
            .result.type_guide as $types
        else
            . as $types
        end |
        {
            "total_types": ($types | length),
            "spawn_supported": [
                $types | to_entries | .[] |
                select(.value.spawn_format != null)
            ] | length,
            "types_with_mutations": [
                $types | to_entries | .[] |
                select(.value.mutation_paths != null and .value.mutation_paths != {})
            ] | length,
            "total_mutation_paths": [
                $types | to_entries | .[] |
                .value.mutation_paths // {} | keys | .[]
            ] | length,
            "untested_count": [
                $types | to_entries | .[] |
                select(.value.test_status == "untested")
            ] | length,
            "auto_passed_count": [
                $types | to_entries | .[] |
                select(.value.test_status == "passed")
            ] | length
        }
    ' "$TARGET_FILE")

    # Output statistics in both human-readable and JSON format
    echo "$STATS_JSON" | jq -r '
        "Statistics:",
        "  Total types: \(.total_types)",
        "  Spawn-supported types: \(.spawn_supported)",
        "  Types with mutations: \(.types_with_mutations)",
        "  Total mutation paths: \(.total_mutation_paths)",
        "  Untested: \(.untested_count)",
        "  Auto-passed: \(.auto_passed_count)"
    '

    # Save statistics to a companion file for easy parsing
    STATS_FILE="${TARGET_FILE%.json}_stats.json"
    echo "$STATS_JSON" > "$STATS_FILE"
    echo ""
    echo "Statistics saved to: $STATS_FILE"
    echo ""
    echo "The file contains the COMPLETE BRP schema for each type including:"
    echo "  - spawn_format with examples"
    echo "  - mutation_paths with examples for each path"
    echo "  - supported_operations"
    echo "  - reflection_traits"
    echo "  - PLUS test metadata (batch_number, test_status, fail_reason)"
else
    echo "Error: Failed to augment BRP response"
    exit 1
fi
