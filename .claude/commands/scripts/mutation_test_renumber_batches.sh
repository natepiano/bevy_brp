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
# Handle both wrapped (type_guide) and direct array formats
process_types '
    if .type_guide then
        .type_guide |= map(
            if .test_status == "failed" then
                .test_status = "untested" |
                .fail_reason = ""
            else
                .
            end
        )
    elif .result.type_guide then
        .result.type_guide |= map(
            if .test_status == "failed" then
                .test_status = "untested" |
                .fail_reason = ""
            else
                .
            end
        )
    else
        map(
            if .test_status == "failed" then
                .test_status = "untested" |
                .fail_reason = ""
            else
                .
            end
        )
    end
' > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Clearing existing batch numbers..."
# Clear all batch numbers
process_types '
    if .type_guide then
        .type_guide |= map(.batch_number = null)
    elif .result.type_guide then
        .result.type_guide |= map(.batch_number = null)
    else
        map(.batch_number = null)
    end
' > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

echo "Assigning batch numbers to untested types..."
# Assign batch numbers to untested types only (divide by BATCH_SIZE)
jq --argjson batch_size "$BATCH_SIZE" '
    if .type_guide then
        # Process wrapped format with type_guide at root
        ([.type_guide[] | select(.test_status == "untested")] | to_entries | 
         map({key: (.value.type_name // .value.type // ("index_" + (.key | tostring))), 
              value: ((.key / $batch_size) | floor + 1)}) | from_entries) as $batch_map |
        .type_guide |= map(
            if .test_status == "untested" then
                .batch_number = $batch_map[(.type_name // .type // "unknown")]
            else
                .batch_number = null
            end
        )
    elif .result.type_guide then
        # Process wrapped format with result.type_guide
        ([.result.type_guide[] | select(.test_status == "untested")] | to_entries | 
         map({key: (.value.type_name // .value.type // ("index_" + (.key | tostring))), 
              value: ((.key / $batch_size) | floor + 1)}) | from_entries) as $batch_map |
        .result.type_guide |= map(
            if .test_status == "untested" then
                .batch_number = $batch_map[(.type_name // .type // "unknown")]
            else
                .batch_number = null
            end
        )
    else
        # Process direct array format
        ([.[] | select(.test_status == "untested")] | to_entries | 
         map({key: (.value.type // ("index_" + (.key | tostring))), 
              value: ((.key / $batch_size) | floor + 1)}) | from_entries) as $batch_map |
        map(
            if .test_status == "untested" then
                .batch_number = $batch_map[.type]
            else
                .batch_number = null
            end
        )
    end
' "$JSON_FILE" > "${JSON_FILE}.tmp" && mv "${JSON_FILE}.tmp" "$JSON_FILE"

# Count statistics (handle both formats)
TOTAL=$(process_types '
    if .type_guide then
        .type_guide | length
    elif .result.type_guide then
        .result.type_guide | length
    else
        length
    end
')

UNTESTED=$(process_types '
    if .type_guide then
        [.type_guide[] | select(.test_status == "untested")] | length
    elif .result.type_guide then
        [.result.type_guide[] | select(.test_status == "untested")] | length
    else
        [.[] | select(.test_status == "untested")] | length
    end
')

FAILED=$(process_types '
    if .type_guide then
        [.type_guide[] | select(.test_status == "failed")] | length
    elif .result.type_guide then
        [.result.type_guide[] | select(.test_status == "failed")] | length
    else
        [.[] | select(.test_status == "failed")] | length
    end
')

PASSED=$(process_types '
    if .type_guide then
        [.type_guide[] | select(.test_status == "passed")] | length
    elif .result.type_guide then
        [.result.type_guide[] | select(.test_status == "passed")] | length
    else
        [.[] | select(.test_status == "passed")] | length
    end
')

MAX_BATCH=$(process_types '
    if .type_guide then
        [.type_guide[] | select(.batch_number != null) | .batch_number] | max // 0
    elif .result.type_guide then
        [.result.type_guide[] | select(.batch_number != null) | .batch_number] | max // 0
    else
        [.[] | select(.batch_number != null) | .batch_number] | max // 0
    end
')

echo "✓ Batch renumbering complete!"
echo ""
echo "Statistics:"
echo "  Total types: $TOTAL"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"  
echo "  Untested: $UNTESTED"
echo "  Batches to process: $MAX_BATCH"