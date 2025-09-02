#!/bin/bash

# Merge Batch Test Results into Type Validation JSON
# Usage: ./merge_batch_results.sh <results_file> <validation_file>
#
# Expects results_file to contain JSON array:
# [
#   {"type": "...", "status": "PASS|FAIL", "fail_reason": "..."},
#   ...
# ]

set -e

# Check arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <results_file> <validation_file>"
    echo "  results_file:     Path to batch results JSON (array of test results)"
    echo "  validation_file:  Path to type_validation.json to update"
    exit 1
fi

RESULTS_FILE="$1"
VALIDATION_FILE="$2"

# Check if files exist
if [ ! -f "$RESULTS_FILE" ]; then
    echo "Error: Results file '$RESULTS_FILE' not found!"
    exit 1
fi

if [ ! -f "$VALIDATION_FILE" ]; then
    echo "Error: Validation file '$VALIDATION_FILE' not found!"
    exit 1
fi

# Create backup
cp "$VALIDATION_FILE" "${VALIDATION_FILE}.bak"

# Merge results into validation file
jq --slurpfile results "$RESULTS_FILE" '
  . as $validation |
  $results[0] as $batch_results |
  $validation | map(
    . as $entry |
    ($batch_results[] | select(.type == $entry.type)) as $result |
    if $result then
      $entry | 
      .test_status = (if $result.status == "PASS" then "passed" else "failed" end) |
      .fail_reason = $result.fail_reason
    else
      $entry
    end
  )
' "$VALIDATION_FILE" > "${VALIDATION_FILE}.tmp"

# Atomic move
mv "${VALIDATION_FILE}.tmp" "$VALIDATION_FILE"

# Report statistics
TOTAL_RESULTS=$(jq 'length' "$RESULTS_FILE")
PASSED=$(jq '[.[] | select(.status == "PASS")] | length' "$RESULTS_FILE")
FAILED=$(jq '[.[] | select(.status == "FAIL")] | length' "$RESULTS_FILE")

echo "✓ Merged $TOTAL_RESULTS results into $VALIDATION_FILE"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"

# Check for failures
if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "⚠️  FAILURES DETECTED:"
    jq -r '.[] | select(.status == "FAIL") | "  - \(.type): \(.fail_reason)"' "$RESULTS_FILE"
    exit 2  # Special exit code for failures
fi