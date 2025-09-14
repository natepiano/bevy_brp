#!/bin/bash
# Summarize test results from JSON files
# Usage: ./summarize_results.sh <json_file>

if [ $# -ne 1 ]; then
    echo "Usage: $0 <json_file>"
    echo "Example: $0 /var/folders/.../batch_results_1.json"
    echo "Example: $0 /var/folders/.../all_types.json"
    exit 1
fi

JSON_FILE="$1"

if [ ! -f "$JSON_FILE" ]; then
    echo "Error: File '$JSON_FILE' not found"
    exit 1
fi

echo "📊 Test Results Summary for $(basename "$JSON_FILE")"
echo "============================================"
echo

# Check for type_guide at root (only supported format)
if ! jq -e '.type_guide' "$JSON_FILE" > /dev/null 2>&1; then
    echo "❌ Invalid JSON format. Expected: {type_guide: [...]}"
    exit 1
fi

# Type Guide Summary
echo "📋 Type Guide Summary:"
echo "Total types: $(jq '.type_guide | length' "$JSON_FILE")"
echo "Spawn-supported: $(jq '[.type_guide[] | select(.supported_operations[]? == "Spawn" or .supported_operations[]? == "Insert")] | length' "$JSON_FILE")"
echo "With mutations: $(jq '[.type_guide[] | select(.mutation_paths | length > 0)] | length' "$JSON_FILE")"

# Count mutation statuses
echo
echo "Mutation status breakdown:"
jq -r '
  [.type_guide[] | .mutation_paths | to_entries[] |
   .value.path_info.mutation_status // "unknown"
  ] | group_by(.) | map({status: .[0], count: length}) |
  .[] | "\(.status): \(.count)"
' "$JSON_FILE"

# Show test statuses if present
if jq -e '.type_guide[0].test_status' "$JSON_FILE" > /dev/null 2>&1; then
    echo
    echo "Test status breakdown:"
    jq -r '[.type_guide[] | .test_status] | group_by(.) | map({status: .[0], count: length}) | .[] | "\(.status): \(.count)"' "$JSON_FILE"

    # Show failed tests
    FAIL_COUNT=$(jq '[.type_guide[] | select(.test_status == "failed")] | length' "$JSON_FILE")
    if [ "$FAIL_COUNT" -gt 0 ]; then
        echo
        echo "❌ Failed types:"
        jq -r '.type_guide[] | select(.test_status == "failed") | "  - \(.type_name): \(.fail_reason // "no reason given")"' "$JSON_FILE"
    fi
fi