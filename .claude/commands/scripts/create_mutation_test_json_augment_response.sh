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
EXCLUSION_FILE="/Users/natemccoy/rust/bevy_brp/.claude/commands/scripts/mutation_test_excluded_types.json"
EXCLUDED_TYPES=""
if [ -f "$EXCLUSION_FILE" ]; then
    # Extract type_name values from JSON and create pipe-separated regex
    EXCLUDED_TYPES=$(jq -r '.excluded_types[].type_name' "$EXCLUSION_FILE" 2>/dev/null | tr '\n' '|' | sed 's/|$//')
fi

# Create the augmented JSON using jq
# This preserves ALL fields from the original BRP response and adds test metadata
jq --arg excluded "$EXCLUDED_TYPES" '
# Process the response, preserving everything and adding test metadata
if .type_guide then
    # Handle the brp_all_type_guides format
    .type_guide |= map(
        # Skip excluded types entirely
        if ($excluded != "" and .type != null and (.type | test($excluded))) then
            empty
        else
            # Preserve ALL original fields and add test metadata
            . + {
                # Add test tracking fields
                "batch_number": null,
                "test_status": (
                    # Auto-pass types that only have spawn support
                    if (.mutation_paths == null or .mutation_paths == [] or
                        (.mutation_paths | type == "array" and length == 1 and .[0].path == "")) then
                        "passed"
                    else
                        "untested"
                    end
                ),
                "fail_reason": ""
            }
        end
    )
elif .result.type_guide then
    # Handle wrapped format (if it exists)
    .result.type_guide |= map(
        # Skip excluded types entirely
        if ($excluded != "" and .type != null and (.type | test($excluded))) then
            empty
        else
            # Preserve ALL original fields and add test metadata
            . + {
                # Add test tracking fields
                "batch_number": null,
                "test_status": (
                    # Auto-pass types that only have spawn support
                    if (.mutation_paths == null or .mutation_paths == [] or
                        (.mutation_paths | type == "array" and length == 1 and .[0].path == "")) then
                        "passed"
                    else
                        "untested"
                    end
                ),
                "fail_reason": ""
            }
        end
    )
else
    # Handle direct array format (fallback)
    map(
        if ($excluded != "" and .type != null and (.type | test($excluded))) then
            empty
        else
            . + {
                "batch_number": null,
                "test_status": (
                    if (.mutation_paths == null or .mutation_paths == [] or
                        (.mutation_paths | type == "array" and length == 1 and .[0].path == "")) then
                        "passed"
                    else
                        "untested"
                    end
                ),
                "fail_reason": ""
            }
        end
    )
end
' "$FILEPATH" > "$TARGET_FILE"

if [ $? -eq 0 ]; then
    echo "Successfully augmented BRP response to $TARGET_FILE"

    # Display statistics about the augmented file
    TYPE_COUNT=$(jq -r '
        if .type_guide then
            .type_guide | length
        elif .result.type_guide then
            .result.type_guide | length
        else
            length
        end
    ' "$TARGET_FILE")

    UNTESTED_COUNT=$(jq -r '
        if .type_guide then
            [.type_guide[] | select(.test_status == "untested")] | length
        elif .result.type_guide then
            [.result.type_guide[] | select(.test_status == "untested")] | length
        else
            [.[] | select(.test_status == "untested")] | length
        end
    ' "$TARGET_FILE")

    PASSED_COUNT=$(jq -r '
        if .type_guide then
            [.type_guide[] | select(.test_status == "passed")] | length
        elif .result.type_guide then
            [.result.type_guide[] | select(.test_status == "passed")] | length
        else
            [.[] | select(.test_status == "passed")] | length
        end
    ' "$TARGET_FILE")

    echo "Statistics:"
    echo "  Total types: $TYPE_COUNT"
    echo "  Untested: $UNTESTED_COUNT"
    echo "  Auto-passed: $PASSED_COUNT"
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
