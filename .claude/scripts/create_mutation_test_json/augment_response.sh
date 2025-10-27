#!/bin/bash

# Augment full BRP response with test metadata
# This script preserves the ENTIRE BRP response structure while adding test tracking fields
# Usage: ./create_mutation_test_json_augment_response.sh [FILEPATH] [TARGET_FILE] [MODE]
#
# MODE (optional):
#   - preserve (default): Preserve test results from existing TARGET_FILE
#   - init/initialize: Start fresh with all types untested or auto-passed

FILEPATH="$1"
TARGET_FILE="$2"
MODE="${3:-preserve}"  # Third argument, defaults to "preserve"

if [ -z "$FILEPATH" ] || [ -z "$TARGET_FILE" ]; then
    echo "Usage: $0 <source_json_file> <target_json_file> [mode]"
    echo "  mode: preserve (default) | init | initialize"
    exit 1
fi

if [ ! -f "$FILEPATH" ]; then
    echo "Error: Source file $FILEPATH does not exist"
    exit 1
fi

# If preserving and target file exists, read previous test results
PREVIOUS_RESULTS="{}"
if [[ "$MODE" != "init" && "$MODE" != "initialize" && -f "$TARGET_FILE" ]]; then
    echo "Preserving test results from existing $TARGET_FILE"
    # Extract test results: {type_name: {batch_number, test_status, fail_reason}}
    PREVIOUS_RESULTS=$(jq -r '.type_guide | to_entries | map({(.key): {batch_number: .value.batch_number, test_status: .value.test_status, fail_reason: .value.fail_reason}}) | add' "$TARGET_FILE" 2>/dev/null || echo "{}")
else
    if [[ "$MODE" == "init" || "$MODE" == "initialize" ]]; then
        echo "Initializing fresh test results (mode: $MODE)"
    else
        echo "No existing file to preserve from - initializing fresh test results"
    fi
fi

# Create the augmented JSON using jq
# This preserves ALL fields from the original BRP response and adds test metadata
jq --argjson previous "$PREVIOUS_RESULTS" '
# Process the response, preserving everything and adding test metadata
# type_guide is an object with type names as keys
.type_guide |= with_entries(
    . as $entry |
    # Check if we have previous test results for this type
    if $previous[$entry.key] then
            # Preserve previous test results
            {
                key: $entry.key,
                value: ($entry.value + {
                    # Add the type field from the key
                    "type": $entry.key,
                    # Preserve previous test tracking fields
                    "batch_number": $previous[$entry.key].batch_number,
                    "test_status": $previous[$entry.key].test_status,
                    "fail_reason": $previous[$entry.key].fail_reason
                })
            }
        else
            # New type - initialize with placeholder metadata
            # The initialize_test_metadata.py script will apply proper auto-pass logic
            {
                key: $entry.key,
                value: ($entry.value + {
                    # Add the type field from the key
                    "type": $entry.key,
                    # Add placeholder test tracking fields
                    "batch_number": null,
                    "test_status": "untested",
                    "fail_reason": ""
                })
            }
        end
)
' "$FILEPATH" > "$TARGET_FILE"

if [ $? -eq 0 ]; then
    echo "Successfully augmented BRP response to $TARGET_FILE"

    # Always apply auto-pass logic using the standalone script
    # This ensures a single source of truth for auto-pass logic
    if [[ "$MODE" == "init" || "$MODE" == "initialize" ]]; then
        echo "Applying auto-pass logic (init mode - all types reset)..."
        python3 .claude/scripts/mutation_test/initialize_test_metadata.py --file "$TARGET_FILE" --reset-all
    else
        echo "Applying auto-pass logic (preserve mode - only new types)..."
        python3 .claude/scripts/mutation_test/initialize_test_metadata.py --file "$TARGET_FILE"
    fi

    # Calculate comprehensive statistics about the augmented file
    STATS_JSON=$(jq -r '
        (.type_guide // .result.type_guide // .) as $types |
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
