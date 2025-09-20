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

# Merge results into mutation test file (expect type_guide as dict at root)
jq --slurpfile results "$RESULTS_FILE" '
  . as $mutation_test |
  $results[0] as $batch_results |
  # Create a lookup map from batch results for efficient access
  ($batch_results | map({key: .type, value: .}) | from_entries) as $result_map |

  # Update type_guide dict
  .type_guide |= with_entries(
    .key as $type_key |
    if $result_map[$type_key] then
      # Update entry with test result
      .value |= (
        .test_status = (if $result_map[$type_key].status == "PASS" then "passed" else "failed" end) |
        .fail_reason = $result_map[$type_key].fail_reason
      )
    else
      # Keep entry unchanged if no result for this type
      .
    end
  )
' "$MUTATION_TEST_FILE" > "${MUTATION_TEST_FILE}.tmp"

# Atomic move
mv "${MUTATION_TEST_FILE}.tmp" "$MUTATION_TEST_FILE"

# Report statistics
TOTAL_RESULTS=$(jq 'length' "$RESULTS_FILE")
PASSED=$(jq '[.[] | select(.status == "PASS")] | length' "$RESULTS_FILE")
FAILED=$(jq '[.[] | select(.status == "FAIL")] | length' "$RESULTS_FILE")
MISSING=$(jq '[.[] | select(.status == "COMPONENT_NOT_FOUND")] | length' "$RESULTS_FILE")

echo "✓ Merged $TOTAL_RESULTS results into $MUTATION_TEST_FILE"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"
echo "  Missing Components: $MISSING"

# Check for failures
if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "⚠️  FAILURES DETECTED:"
    # Save detailed failure information to a separate file
    FAILURE_LOG="${MUTATION_TEST_FILE%.json}_failures_$(date +%Y%m%d_%H%M%S).json"
    jq '[.[] | select(.status == "FAIL" or .status == "COMPONENT_NOT_FOUND")]' "$RESULTS_FILE" > "$FAILURE_LOG"
    echo "  Detailed failure information saved to: $FAILURE_LOG"
    echo ""

    # Display summary of failures
    jq -r '.[] | select(.status == "FAIL") | "  - \(.type): \(.failure_details.error_message // .fail_reason)"' "$RESULTS_FILE"
    exit 2  # Special exit code for failures
fi

# Check for missing components
if [ "$MISSING" -gt 0 ]; then
    echo ""
    echo "⚠️  MISSING COMPONENTS DETECTED:"
    jq -r '.[] | select(.status == "COMPONENT_NOT_FOUND") | "  - \(.type): \(.failure_details.error_message // .fail_reason)"' "$RESULTS_FILE"
    exit 2  # Special exit code for missing components
fi