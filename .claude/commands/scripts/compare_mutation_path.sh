#!/bin/bash

# compare_mutation_path.sh
# Compares a specific mutation path between current and baseline type guides
#
# Usage 1 (with files): compare_mutation_path.sh CURRENT_FILE BASELINE_FILE TYPE_NAME MUTATION_PATH
# Usage 2 (with JSON): compare_mutation_path.sh TYPE_NAME MUTATION_PATH CURRENT_JSON BASELINE_JSON

set -e

# Detect usage mode based on number of arguments and whether first args are files
if [ $# -eq 4 ] && [ -f "$1" ] && [ -f "$2" ]; then
    # File mode (original behavior)
    CURRENT_FILE="$1"
    BASELINE_FILE="$2"
    TYPE_NAME="$3"
    MUTATION_PATH="$4"
elif [ $# -eq 4 ]; then
    # JSON mode (new behavior)
    TYPE_NAME="$1"
    MUTATION_PATH="$2"
    CURRENT_JSON="$3"
    BASELINE_JSON="$4"

    # Create temporary files for the JSON data
    CURRENT_FILE=$(mktemp /tmp/current_XXXXXX.json)
    BASELINE_FILE=$(mktemp /tmp/baseline_XXXXXX.json)

    # Write JSON to temp files
    echo "$CURRENT_JSON" > "$CURRENT_FILE"
    echo "$BASELINE_JSON" > "$BASELINE_FILE"

    # Clean up temp files on exit
    trap "rm -f $CURRENT_FILE $BASELINE_FILE" EXIT
else
    echo "Usage 1 (with files): $0 CURRENT_FILE BASELINE_FILE TYPE_NAME MUTATION_PATH"
    echo "Usage 2 (with JSON): $0 TYPE_NAME MUTATION_PATH CURRENT_JSON BASELINE_JSON"
    echo "Example: $0 bevy_ui::ui_node::Node .grid_template_columns '{...}' '{...}'"
    exit 1
fi

# Extract the mutation path data from both files
# Handle both the new format (direct type guides) and baseline format
echo "=== MUTATION PATH COMPARISON ==="
echo "Type: $TYPE_NAME"
echo "Path: $MUTATION_PATH"
echo ""

# Extract from current (new format - direct type guide)
echo "=== CURRENT VERSION ==="
CURRENT_DATA=$(jq -r --arg type "$TYPE_NAME" --arg path "$MUTATION_PATH" '
    if .[$type] then
        .[$type].mutation_paths[$path] // "Path not found"
    elif .type_guide then
        .type_guide[$type].mutation_paths[$path] // "Path not found"
    elif .type_guides then
        (.type_guides[] | select(.type_name == $type).mutation_paths[$path]) // "Path not found"
    else
        "Type not found"
    end
' "$CURRENT_FILE")

if [ "$CURRENT_DATA" = "Type not found" ] || [ "$CURRENT_DATA" = "Path not found" ]; then
    echo "Error: $CURRENT_DATA"
    echo ""
else
    echo "$CURRENT_DATA" | jq '.'
    echo ""
fi

# Extract from baseline (check both formats)
echo "=== BASELINE VERSION ==="
BASELINE_DATA=$(jq -r --arg type "$TYPE_NAME" --arg path "$MUTATION_PATH" '
    if .guide then
        # Format from get_type_guide.sh
        .guide.mutation_paths[$path] // "Path not found"
    elif .[$type] then
        # Direct format
        .[$type].mutation_paths[$path] // "Path not found"
    else
        "Type not found"
    end
' "$BASELINE_FILE")

if [ "$BASELINE_DATA" = "Type not found" ] || [ "$BASELINE_DATA" = "Path not found" ]; then
    echo "Error: $BASELINE_DATA"
    echo ""
else
    echo "$BASELINE_DATA" | jq '.'
    echo ""
fi

# Compare the examples if both exist
if [ "$CURRENT_DATA" != "Type not found" ] && [ "$CURRENT_DATA" != "Path not found" ] && \
   [ "$BASELINE_DATA" != "Type not found" ] && [ "$BASELINE_DATA" != "Path not found" ]; then

    echo "=== EXAMPLE COMPARISON ==="

    # Extract just the example field
    CURRENT_EXAMPLE=$(echo "$CURRENT_DATA" | jq '.example')
    BASELINE_EXAMPLE=$(echo "$BASELINE_DATA" | jq '.example')

    echo "Current example:"
    echo "$CURRENT_EXAMPLE" | jq '.'
    echo ""

    echo "Baseline example:"
    echo "$BASELINE_EXAMPLE" | jq '.'
    echo ""

    # Check if they're identical
    if [ "$CURRENT_EXAMPLE" = "$BASELINE_EXAMPLE" ]; then
        echo "✅ Examples are identical"
    else
        echo "⚠️  Examples differ"

        # Try to identify the type of change
        CURRENT_TYPE=$(echo "$CURRENT_EXAMPLE" | jq -r 'type')
        BASELINE_TYPE=$(echo "$BASELINE_EXAMPLE" | jq -r 'type')

        if [ "$CURRENT_TYPE" != "$BASELINE_TYPE" ]; then
            echo "   Type changed: $BASELINE_TYPE → $CURRENT_TYPE"
        elif [ "$CURRENT_TYPE" = "array" ]; then
            CURRENT_LEN=$(echo "$CURRENT_EXAMPLE" | jq 'length')
            BASELINE_LEN=$(echo "$BASELINE_EXAMPLE" | jq 'length')
            if [ "$CURRENT_LEN" != "$BASELINE_LEN" ]; then
                echo "   Array length changed: $BASELINE_LEN → $CURRENT_LEN"
            else
                echo "   Array contents changed (same length)"
            fi
        elif [ "$CURRENT_TYPE" = "object" ]; then
            CURRENT_KEYS=$(echo "$CURRENT_EXAMPLE" | jq -r 'keys | sort | join(",")')
            BASELINE_KEYS=$(echo "$BASELINE_EXAMPLE" | jq -r 'keys | sort | join(",")')
            if [ "$CURRENT_KEYS" != "$BASELINE_KEYS" ]; then
                echo "   Object keys changed"
                echo "   Baseline keys: $BASELINE_KEYS"
                echo "   Current keys: $CURRENT_KEYS"
            else
                echo "   Object values changed (same keys)"
            fi
        else
            echo "   Value changed: $BASELINE_EXAMPLE → $CURRENT_EXAMPLE"
        fi
    fi
fi