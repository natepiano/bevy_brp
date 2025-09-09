#!/bin/bash

# Merge Batch Test Results into Mutation Test JSON (FULL SCHEMA format)
# Usage: ./merge_batch_results.sh <results_file> <mutation_test_file>
#
# Expects results_file to contain JSON array:
# [
#   {"type": "...", "status": "PASS|FAIL", "fail_reason": "..."},
#   ...
# ]

set -e

# Check arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <results_file> <mutation_test_file>"
    echo "  results_file:      Path to batch results JSON (array of test results)"
    echo "  mutation_test_file: Path to mutation test JSON file to update"
    exit 1
fi

RESULTS_FILE="$1"
MUTATION_TEST_FILE="$2"

# Check if files exist
if [ ! -f "$RESULTS_FILE" ]; then
    echo "Error: Results file '$RESULTS_FILE' not found!"
    exit 1
fi

if [ ! -f "$MUTATION_TEST_FILE" ]; then
    echo "Error: Mutation test file '$MUTATION_TEST_FILE' not found!"
    exit 1
fi

# Merge results into mutation test file (handle both wrapped and direct formats)
jq --slurpfile results "$RESULTS_FILE" '
  . as $mutation_test |
  $results[0] as $batch_results |
  # Create a lookup map from batch results for efficient access
  ($batch_results | map({key: .type, value: .}) | from_entries) as $result_map |
  
  # Handle different file structures
  if .type_guide then
    # Update type_guide array
    .type_guide |= map(
      . as $entry |
      # Use type_name or type field
      ($entry.type_name // $entry.type // "unknown") as $type_key |
      if $result_map[$type_key] then
        # Update entry with test result
        $entry | 
        .test_status = (if $result_map[$type_key].status == "PASS" then "passed" else "failed" end) |
        .fail_reason = $result_map[$type_key].fail_reason
      else
        # Keep entry unchanged if no result for this type
        $entry
      end
    )
  elif .result.type_guide then
    # Update result.type_guide array
    .result.type_guide |= map(
      . as $entry |
      # Use type_name or type field
      ($entry.type_name // $entry.type // "unknown") as $type_key |
      if $result_map[$type_key] then
        # Update entry with test result
        $entry | 
        .test_status = (if $result_map[$type_key].status == "PASS" then "passed" else "failed" end) |
        .fail_reason = $result_map[$type_key].fail_reason
      else
        # Keep entry unchanged if no result for this type
        $entry
      end
    )
  else
    # Handle direct array format (legacy)
    map(
      . as $entry |
      if $result_map[.type] then
        # Update entry with test result
        $entry | 
        .test_status = (if $result_map[.type].status == "PASS" then "passed" else "failed" end) |
        .fail_reason = $result_map[.type].fail_reason
      else
        # Keep entry unchanged if no result for this type
        $entry
      end
    )
  end
' "$MUTATION_TEST_FILE" > "${MUTATION_TEST_FILE}.tmp"

# Atomic move
mv "${MUTATION_TEST_FILE}.tmp" "$MUTATION_TEST_FILE"

# Report statistics
TOTAL_RESULTS=$(jq 'length' "$RESULTS_FILE")
PASSED=$(jq '[.[] | select(.status == "PASS")] | length' "$RESULTS_FILE")
FAILED=$(jq '[.[] | select(.status == "FAIL")] | length' "$RESULTS_FILE")

echo "✓ Merged $TOTAL_RESULTS results into $MUTATION_TEST_FILE"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"

# Check for failures
if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "⚠️  FAILURES DETECTED:"
    jq -r '.[] | select(.status == "FAIL") | "  - \(.type): \(.fail_reason)"' "$RESULTS_FILE"
    exit 2  # Special exit code for failures
fi