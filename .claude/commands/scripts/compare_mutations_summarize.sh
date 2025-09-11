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

# Detect file format
if jq -e '.type_guide' "$JSON_FILE" > /dev/null 2>&1; then
    # New all_types.json format
    echo "📋 Type Guide Summary:"
    echo "Total types: $(jq '.type_guide | length' "$JSON_FILE")"
    echo "Spawn-supported: $(jq '[.type_guide[] | select(.supported_operations[]? == "Spawn" or .supported_operations[]? == "Insert")] | length' "$JSON_FILE")"
    echo "With mutations: $(jq '[.type_guide[] | select(.mutation_paths | length > 0)] | length' "$JSON_FILE")"
    
    # Count mutation statuses
    echo
    echo "Mutation status breakdown:"
    jq -r '
      [.type_guide[] | .mutation_paths | to_entries[] | 
       if .value.path_info.mutation_status then .value.path_info.mutation_status 
       else .value.mutation_status // "unknown" end
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

elif jq -e 'type == "array" and .[0].status' "$JSON_FILE" > /dev/null 2>&1; then
    # Old batch results format
    echo "📋 Batch Results Summary:"
    
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
else
    echo "❌ Unknown JSON format. Expected either:"
    echo "  - Type guide format: {type_guide: [...]}"
    echo "  - Batch results format: [{status: ..., type: ...}, ...]"
    exit 1
fi