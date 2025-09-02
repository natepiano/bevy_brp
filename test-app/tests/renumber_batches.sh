#!/bin/bash

# Type Validation Test - Batch Renumbering Script
# Clears and reassigns batch numbers for untested/failed types

set -e

JSON_FILE="test-app/tests/type_validation.json"
BATCH_SIZE=30

# Check if the JSON file exists
if [ ! -f "$JSON_FILE" ]; then
    echo "Error: $JSON_FILE not found!"
    exit 1
fi

echo "Clearing existing batch numbers..."
# Clear all batch numbers
jq 'map(.batch_number = null)' "$JSON_FILE" > /tmp/type_validation_temp.json && \
    mv /tmp/type_validation_temp.json "$JSON_FILE"

echo "Assigning batch numbers to untested/failed types..."
# Assign batch numbers to untested types only (divide by BATCH_SIZE)
jq --argjson batch_size "$BATCH_SIZE" '
  [.[] | select(.test_status == "untested" or .test_status == "failed")] as $untested |
  ($untested | to_entries | map({key: .value.type, value: ((.key / $batch_size) | floor + 1)}) | from_entries) as $batch_map |
  map(
    if (.test_status == "untested" or .test_status == "failed") then
      .batch_number = $batch_map[.type]
    else
      .batch_number = null
    end
  )
' "$JSON_FILE" > /tmp/type_validation_temp.json && \
    mv /tmp/type_validation_temp.json "$JSON_FILE"

# Count statistics
TOTAL=$(jq 'length' "$JSON_FILE")
UNTESTED=$(jq '[.[] | select(.test_status == "untested")] | length' "$JSON_FILE")
FAILED=$(jq '[.[] | select(.test_status == "failed")] | length' "$JSON_FILE")
PASSED=$(jq '[.[] | select(.test_status == "passed")] | length' "$JSON_FILE")
MAX_BATCH=$(jq '[.[] | select(.batch_number != null) | .batch_number] | max // 0' "$JSON_FILE")

echo "✓ Batch renumbering complete!"
echo ""
echo "Statistics:"
echo "  Total types: $TOTAL"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"  
echo "  Untested: $UNTESTED"
echo "  Batches to process: $MAX_BATCH"