#!/bin/bash
# Summarize test results from JSON files
# Usage: ./summarize_results.sh <json_file>

if [ $# -ne 1 ]; then
    echo "Usage: $0 <json_file>"
    echo "Example: $0 /var/folders/.../batch_results_1.json"
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

# Count by status
echo "Status breakdown:"
jq -r '[.[] | .status] | group_by(.) | map({status: .[0], count: length}) | .[] | "\(.status): \(.count)"' "$JSON_FILE"

echo
echo "Total results: $(jq 'length' "$JSON_FILE")"

# Show failures if any
FAIL_COUNT=$(jq '[.[] | select(.status == "FAIL")] | length' "$JSON_FILE")
if [ "$FAIL_COUNT" -gt 0 ]; then
    echo
    echo "❌ Failed types:"
    jq -r '.[] | select(.status == "FAIL") | "  - \(.type): \(.fail_reason)"' "$JSON_FILE"
fi

# Show component not found if any
NOT_FOUND_COUNT=$(jq '[.[] | select(.status == "COMPONENT_NOT_FOUND")] | length' "$JSON_FILE")
if [ "$NOT_FOUND_COUNT" -gt 0 ]; then
    echo
    echo "⚠️  Components not found:"
    jq -r '.[] | select(.status == "COMPONENT_NOT_FOUND") | "  - \(.type)"' "$JSON_FILE" | head -5
    if [ "$NOT_FOUND_COUNT" -gt 5 ]; then
        echo "  ... and $((NOT_FOUND_COUNT - 5)) more"
    fi
fi