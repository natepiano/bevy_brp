#!/bin/bash

# Mutation Test - Batch Renumbering Script for FULL SCHEMA format
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

# Helper function to process types regardless of structure
process_types() {
    local operation="$1"
    
    jq "$operation" "$JSON_FILE"
}

echo "Resetting failed tests to untested..."
# Reset all failed tests to untested and clear fail_reason
# Expect type_guide at root
process_types '
    .type_guide |= map(
        if .test_status == "failed" then
            .test_status = "untested" |
            .fail_reason = ""
        else
            .
        end
    )
' > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Clearing existing batch numbers..."
# Clear all batch numbers
process_types '
    .type_guide |= map(.batch_number = null)
' > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Assigning batch numbers to untested types..."
# Assign batch numbers to untested types only (divide by BATCH_SIZE)
jq --argjson batch_size "$BATCH_SIZE" '
    # Process type_guide at root
    ([.type_guide[] | select(.test_status == "untested")] | to_entries |
     map({key: (.value.type_name // ("index_" + (.key | tostring))),
          value: ((.key / $batch_size) | floor + 1)}) | from_entries) as $batch_map |
    .type_guide |= map(
        if .test_status == "untested" then
            .batch_number = $batch_map[(.type_name // "unknown")]
        else
            .batch_number = null
        end
    )
' "$JSON_FILE" > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

# Count statistics
TOTAL=$(process_types '.type_guide | length')

UNTESTED=$(process_types '[.type_guide[] | select(.test_status == "untested")] | length')

FAILED=$(process_types '[.type_guide[] | select(.test_status == "failed")] | length')

PASSED=$(process_types '[.type_guide[] | select(.test_status == "passed")] | length')

MAX_BATCH=$(process_types '[.type_guide[] | select(.batch_number != null) | .batch_number] | max // 0')

echo "✓ Batch renumbering complete!"
echo ""
echo "Statistics:"
echo "  Total types: $TOTAL"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"  
echo "  Untested: $UNTESTED"
echo "  Batches to process: $MAX_BATCH"