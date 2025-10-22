#!/bin/bash

# Merge Batch Test Results into Mutation Test JSON (FULL SCHEMA format)
# Usage: ./merge_batch_results.sh <batch_number>
#
# Expects batch_results_<batch_number>.json to contain JSON array:
# [
#   {"type": "...", "status": "PASS|FAIL", "fail_reason": "..."},
#   ...
# ]

set -e

# Check arguments
if [ $# -ne 1 ]; then
    echo "Usage: $0 <batch_number>"
    echo "  batch_number: The batch number to merge (e.g., 1, 2, 3)"
    echo ""
    echo "  Automatically uses:"
    echo "    - Input:  .claude/transient/batch_results_<batch_number>.json"
    echo "    - Output: .claude/transient/all_types.json"
    exit 1
fi

BATCH_NUMBER="$1"
RESULTS_FILE=".claude/transient/batch_results_${BATCH_NUMBER}.json"
MUTATION_TEST_FILE=".claude/transient/all_types.json"

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

# Check for failures and filter known issues
if [ "$FAILED" -gt 0 ] || [ "$MISSING" -gt 0 ]; then
    # Load known issues if the file exists
    KNOWN_ISSUES_FILE=".claude/config/mutation_test_known_issues.json"
    if [ -f "$KNOWN_ISSUES_FILE" ]; then
        KNOWN_ISSUES=$(cat "$KNOWN_ISSUES_FILE")
    else
        KNOWN_ISSUES="[]"
    fi

    # Filter out known issues from failures
    NEW_FAILURES=$(jq --argjson known "$KNOWN_ISSUES" '
      . as $results |
      # Get all failures (FAIL or COMPONENT_NOT_FOUND)
      [.[] | select(.status == "FAIL" or .status == "COMPONENT_NOT_FOUND")] as $failures |
      # Filter out known issues
      $failures | map(
        . as $failure |
        # Check if this failure is a known issue
        if ($known | map(select(.type == $failure.type)) | length) > 0
        then empty  # Filter out known issues
        else .      # Keep new failures
        end
      )
    ' "$RESULTS_FILE")

    # Count new vs known failures
    NEW_FAILURE_COUNT=$(echo "$NEW_FAILURES" | jq 'length')
    KNOWN_FAILURE_COUNT=$((FAILED + MISSING - NEW_FAILURE_COUNT))

    # Save all failure information (including known) to a log file
    FAILURE_LOG="${MUTATION_TEST_FILE%.json}_failures_$(date +%Y%m%d_%H%M%S).json"
    jq '[.[] | select(.status == "FAIL" or .status == "COMPONENT_NOT_FOUND")]' "$RESULTS_FILE" > "$FAILURE_LOG"

    if [ "$NEW_FAILURE_COUNT" -gt 0 ]; then
        # NEW failures detected
        echo ""
        echo "⚠️  NEW FAILURES DETECTED:"
        echo "  Total failures: $((FAILED + MISSING)) ($KNOWN_FAILURE_COUNT known, $NEW_FAILURE_COUNT new)"
        echo "  Detailed failure information saved to: $FAILURE_LOG"
        echo ""

        # Display summary of NEW failures only
        echo "$NEW_FAILURES" | jq -r '.[] | "  - \(.type): \(.failure_details.error_message // .fail_reason // "Component not found")"'

        # Exit code 2 = NEW failures exist
        exit 2
    else
        # All failures are known issues
        echo ""
        echo "✓ Batch completed with $KNOWN_FAILURE_COUNT known issue(s) (all expected)"
        echo "  Known issues encountered:"
        jq --argjson known "$KNOWN_ISSUES" -r '
          [.[] | select(.status == "FAIL" or .status == "COMPONENT_NOT_FOUND")] as $failures |
          $failures[] |
          . as $failure |
          if ($known | map(select(.type == $failure.type)) | length) > 0
          then "    - \(.type)"
          else empty
          end
        ' "$RESULTS_FILE"

        # Exit code 0 = success (only known issues)
        exit 0
    fi
fi