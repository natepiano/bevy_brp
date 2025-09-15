#!/bin/bash

# Mutation Test - Batch Renumbering Script
# Clears and reassigns batch numbers for untested/failed types

set -e

# Check for batch size parameter
if [ $# -ne 1 ]; then
    echo "Usage: $0 <batch_size>"
    echo "Example: $0 50"
    exit 1
fi

BATCH_SIZE="$1"
JSON_FILE="$TMPDIR/all_types.json"

# Check if the JSON file exists
if [ ! -f "$JSON_FILE" ]; then
    echo "Error: $JSON_FILE not found!"
    exit 1
fi

echo "Resetting failed tests to untested..."
# Reset all failed tests to untested and clear fail_reason
# type_guide is an array of type objects
jq '
    .type_guide |= map(
        if .test_status == "failed" then
            .test_status = "untested" |
            .fail_reason = ""
        else
            .
        end
    )
' "$JSON_FILE" > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Clearing existing batch numbers..."
# Clear all batch numbers
jq '
    .type_guide |= map(.batch_number = null)
' "$JSON_FILE" > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Assigning batch numbers to untested types..."
# Assign batch numbers to untested types only
# First, collect untested types and assign batch numbers, then merge back
jq --argjson batch_size "$BATCH_SIZE" '
    # Separate untested and other types
    (.type_guide | map(select(.test_status == "untested"))) as $untested |
    (.type_guide | map(select(.test_status != "untested"))) as $others |

    # Assign batch numbers to untested types
    ($untested | to_entries | map(
        .value + {batch_number: ((.key / $batch_size) | floor + 1)}
    )) as $numbered_untested |

    # Merge back in original order
    .type_guide = (.type_guide | map(
        . as $orig |
        if .test_status == "untested" then
            ($numbered_untested | map(select(.type_name == $orig.type_name)) | first)
        else
            . + {batch_number: null}
        end
    ))
' "$JSON_FILE" > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

# Count statistics
TOTAL=$(jq '.type_guide | length' "$JSON_FILE")
UNTESTED=$(jq '[.type_guide[] | select(.test_status == "untested")] | length' "$JSON_FILE")
FAILED=$(jq '[.type_guide[] | select(.test_status == "failed")] | length' "$JSON_FILE")
PASSED=$(jq '[.type_guide[] | select(.test_status == "passed")] | length' "$JSON_FILE")
MAX_BATCH=$(jq '[.type_guide[] | select(.batch_number != null) | .batch_number] | max // 0' "$JSON_FILE")

echo "✓ Batch renumbering complete!"
echo ""
echo "Statistics:"
echo "  Total types: $TOTAL"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"
echo "  Untested: $UNTESTED"
echo "  Batches to process: $MAX_BATCH"