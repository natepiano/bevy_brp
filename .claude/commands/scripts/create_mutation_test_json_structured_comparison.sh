#!/bin/bash

# Enhanced structured comparison for mutation test files
# Usage: create_mutation_test_json_structured_comparison.sh <baseline_file> <current_file>

set -e

if [ $# -ne 2 ]; then
    echo "Usage: $0 <baseline_file> <current_file>"
    exit 1
fi

BASELINE_FILE="$1"
CURRENT_FILE="$2"

# Check if files exist
if [ ! -f "$BASELINE_FILE" ]; then
    echo "❌ Baseline file not found: $BASELINE_FILE"
    exit 1
fi

if [ ! -f "$CURRENT_FILE" ]; then
    echo "❌ Current file not found: $CURRENT_FILE"
    exit 1
fi

echo "🔍 STRUCTURED MUTATION TEST COMPARISON"
echo "======================================"
echo ""

# 1. Binary Identity Check
echo "📊 IDENTITY CHECK"
if cmp -s "$BASELINE_FILE" "$CURRENT_FILE"; then
    echo "✅ FILES ARE IDENTICAL"
    echo "   └─ Baseline and current files are byte-for-byte identical"
    echo ""
    echo "📋 SUMMARY"
    echo "   └─ No changes detected - safe for promotion"
    exit 0
fi

# 2. Files differ - analyze changes
echo "⚠️  FILES DIFFER - ANALYZING CHANGES"
echo "   └─ Found differences requiring review"
echo ""

# 3. Metadata Comparison using jq
echo "📈 METADATA COMPARISON"

# Get type counts
BASELINE_COUNT=$(jq 'length' "$BASELINE_FILE")
CURRENT_COUNT=$(jq 'length' "$CURRENT_FILE")

# Get spawn-supported counts
BASELINE_SPAWN=$(jq '[.[] | select(.spawn_support == "supported")] | length' "$BASELINE_FILE")
CURRENT_SPAWN=$(jq '[.[] | select(.spawn_support == "supported")] | length' "$CURRENT_FILE")

# Get mutation counts
BASELINE_MUTATIONS=$(jq '[.[] | select(.mutation_paths | length > 0)] | length' "$BASELINE_FILE")
CURRENT_MUTATIONS=$(jq '[.[] | select(.mutation_paths | length > 0)] | length' "$CURRENT_FILE")

# Display metadata comparison
if [ "$BASELINE_COUNT" -eq "$CURRENT_COUNT" ]; then
    echo "   Total Types: $BASELINE_COUNT → $CURRENT_COUNT (no change)"
else
    echo "   Total Types: $BASELINE_COUNT → $CURRENT_COUNT (${CURRENT_COUNT} - ${BASELINE_COUNT} = $((CURRENT_COUNT - BASELINE_COUNT)))"
fi

if [ "$BASELINE_SPAWN" -eq "$CURRENT_SPAWN" ]; then
    echo "   Spawn-Supported: $BASELINE_SPAWN → $CURRENT_SPAWN (no change)"
else
    echo "   Spawn-Supported: $BASELINE_SPAWN → $CURRENT_SPAWN (${CURRENT_SPAWN} - ${BASELINE_SPAWN} = $((CURRENT_SPAWN - BASELINE_SPAWN)))"
fi

if [ "$BASELINE_MUTATIONS" -eq "$CURRENT_MUTATIONS" ]; then
    echo "   With Mutations: $BASELINE_MUTATIONS → $CURRENT_MUTATIONS (no change)"
else
    echo "   With Mutations: $BASELINE_MUTATIONS → $CURRENT_MUTATIONS (${CURRENT_MUTATIONS} - ${BASELINE_MUTATIONS} = $((CURRENT_MUTATIONS - BASELINE_MUTATIONS)))"
fi

echo ""

# 4. Type-Level Changes Analysis
echo "🔍 TYPE-LEVEL CHANGES"

# Create temporary files for analysis
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract type info for comparison
jq -r '.[] | "\(.type)|\(.spawn_support)|\(.mutation_paths | length)|\(.test_status)"' "$BASELINE_FILE" | sort > "$TEMP_DIR/baseline_types"
jq -r '.[] | "\(.type)|\(.spawn_support)|\(.mutation_paths | length)|\(.test_status)"' "$CURRENT_FILE" | sort > "$TEMP_DIR/current_types"

# Find new types
NEW_TYPES=$(comm -13 "$TEMP_DIR/baseline_types" "$TEMP_DIR/current_types" | cut -d'|' -f1)
NEW_COUNT=$(echo "$NEW_TYPES" | grep -v "^$" | wc -l | tr -d ' ')

# Find removed types  
REMOVED_TYPES=$(comm -23 "$TEMP_DIR/baseline_types" "$TEMP_DIR/current_types" | cut -d'|' -f1)
REMOVED_COUNT=$(echo "$REMOVED_TYPES" | grep -v "^$" | wc -l | tr -d ' ')

# Find modified types (different attributes for same type name)
MODIFIED_TYPES=""
MODIFIED_COUNT=0

while IFS='|' read -r type spawn_support mutation_count test_status; do
    if [ -n "$type" ]; then
        # Check if type exists in current but with different attributes
        CURRENT_ENTRY=$(grep "^$type|" "$TEMP_DIR/current_types" || true)
        if [ -n "$CURRENT_ENTRY" ]; then
            BASELINE_ENTRY="$type|$spawn_support|$mutation_count|$test_status"
            if [ "$BASELINE_ENTRY" != "$CURRENT_ENTRY" ]; then
                if [ -z "$MODIFIED_TYPES" ]; then
                    MODIFIED_TYPES="$type"
                else
                    MODIFIED_TYPES="$MODIFIED_TYPES\n$type"
                fi
                MODIFIED_COUNT=$((MODIFIED_COUNT + 1))
            fi
        fi
    fi
done < "$TEMP_DIR/baseline_types"

# Display type-level changes
echo "   ├─ Modified Types: $MODIFIED_COUNT"
if [ "$MODIFIED_COUNT" -gt 0 ]; then
    echo -e "$MODIFIED_TYPES" | head -5 | while read -r type; do
        [ -n "$type" ] && echo "   │  ├─ $type: attributes changed"
    done
    if [ "$MODIFIED_COUNT" -gt 5 ]; then
        echo "   │  └─ ... and $((MODIFIED_COUNT - 5)) more"
    fi
fi

echo "   ├─ New Types: $NEW_COUNT"
if [ "$NEW_COUNT" -gt 0 ] && [ "$NEW_COUNT" -le 5 ]; then
    echo "$NEW_TYPES" | while read -r type; do
        [ -n "$type" ] && echo "   │  ├─ $type"
    done
elif [ "$NEW_COUNT" -gt 5 ]; then
    echo "$NEW_TYPES" | head -5 | while read -r type; do
        [ -n "$type" ] && echo "   │  ├─ $type"
    done
    echo "   │  └─ ... and $((NEW_COUNT - 5)) more"
fi

echo "   └─ Removed Types: $REMOVED_COUNT"
if [ "$REMOVED_COUNT" -gt 0 ] && [ "$REMOVED_COUNT" -le 5 ]; then
    echo "$REMOVED_TYPES" | while read -r type; do
        [ -n "$type" ] && echo "       ├─ $type"
    done
elif [ "$REMOVED_COUNT" -gt 5 ]; then
    echo "$REMOVED_TYPES" | head -5 | while read -r type; do
        [ -n "$type" ] && echo "       ├─ $type"
    done
    echo "       └─ ... and $((REMOVED_COUNT - 5)) more"
fi

echo ""

# 5. Change Assessment and Summary
echo "📋 CHANGE ASSESSMENT"

TOTAL_CHANGES=$((MODIFIED_COUNT + NEW_COUNT + REMOVED_COUNT))

if [ "$TOTAL_CHANGES" -eq 0 ]; then
    echo "   └─ No structural changes detected - metadata only differences"
elif [ "$TOTAL_CHANGES" -le 3 ] && [ "$REMOVED_COUNT" -eq 0 ]; then
    echo "   └─ Minor changes detected - safe for promotion"
elif [ "$REMOVED_COUNT" -gt 0 ] || [ "$TOTAL_CHANGES" -gt 10 ]; then
    echo "   └─ Major changes detected - review recommended before promotion"
else
    echo "   └─ Moderate changes detected - review changes before promotion"
fi