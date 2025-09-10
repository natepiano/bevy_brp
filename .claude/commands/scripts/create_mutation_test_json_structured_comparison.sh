#!/bin/bash

# Enhanced structured comparison for mutation test files with FULL SCHEMA support
# Now compares not just paths but actual examples and formats
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

# Helper function to extract type_guide array from either format
extract_type_guide() {
    local file="$1"
    # Handle both wrapped (with type_guide at root) and direct array formats
    jq '
        if .type_guide then
            .type_guide
        elif .result.type_guide then
            .result.type_guide
        else
            .
        end
    ' "$file"
}

echo "🔍 STRUCTURED MUTATION TEST COMPARISON (Full Schema)"
echo "===================================================="
echo ""

# 1. Binary Identity Check
echo "📊 IDENTITY CHECK"
if cmp -s "$BASELINE_FILE" "$CURRENT_FILE"; then
    echo "✅ FILES ARE IDENTICAL"
    echo "   └─ Baseline and current files are byte-for-byte identical"
    echo ""
    
    # Even for identical files, show the current stats
    CURRENT_COUNT=$(extract_type_guide "$CURRENT_FILE" | jq 'length')
    CURRENT_SPAWN=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | select(has("spawn_format"))] | length')
    CURRENT_MUTATIONS=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | select(.mutation_paths != null and .mutation_paths != {} and .mutation_paths != [])] | length')
    CURRENT_TOTAL_PATHS=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | 
        if .mutation_paths != null and .mutation_paths != {} and .mutation_paths != [] then
            if .mutation_paths | type == "object" then
                .mutation_paths | keys | length
            else
                0
            end
        else
            0
        end] | add')
    
    echo "📈 CURRENT FILE STATISTICS"
    echo "   Total Types: $CURRENT_COUNT"
    echo "   Spawn-Supported: $CURRENT_SPAWN"
    echo "   Types with Mutations: $CURRENT_MUTATIONS"
    echo "   Total Mutation Paths: $CURRENT_TOTAL_PATHS"
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
BASELINE_COUNT=$(extract_type_guide "$BASELINE_FILE" | jq 'length')
CURRENT_COUNT=$(extract_type_guide "$CURRENT_FILE" | jq 'length')

# Get spawn-supported counts (check spawn_format existence)
BASELINE_SPAWN=$(extract_type_guide "$BASELINE_FILE" | jq '[.[] | select(has("spawn_format"))] | length')
CURRENT_SPAWN=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | select(has("spawn_format"))] | length')

# Get mutation counts (check mutation_paths not null/empty)
BASELINE_MUTATIONS=$(extract_type_guide "$BASELINE_FILE" | jq '[.[] | select(.mutation_paths != null and .mutation_paths != {} and .mutation_paths != [])] | length')
CURRENT_MUTATIONS=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | select(.mutation_paths != null and .mutation_paths != {} and .mutation_paths != [])] | length')

# Count total mutation paths across all types
BASELINE_TOTAL_PATHS=$(extract_type_guide "$BASELINE_FILE" | jq '[.[] | 
    if .mutation_paths != null and .mutation_paths != {} and .mutation_paths != [] then
        if .mutation_paths | type == "object" then
            .mutation_paths | keys | length
        else
            0
        end
    else
        0
    end] | add')
    
CURRENT_TOTAL_PATHS=$(extract_type_guide "$CURRENT_FILE" | jq '[.[] | 
    if .mutation_paths != null and .mutation_paths != {} and .mutation_paths != [] then
        if .mutation_paths | type == "object" then
            .mutation_paths | keys | length
        else
            0
        end
    else
        0
    end] | add')

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

if [ "$BASELINE_TOTAL_PATHS" -eq "$CURRENT_TOTAL_PATHS" ]; then
    echo "   Total Mutation Paths: $BASELINE_TOTAL_PATHS → $CURRENT_TOTAL_PATHS (no change)"
else
    echo "   Total Mutation Paths: $BASELINE_TOTAL_PATHS → $CURRENT_TOTAL_PATHS (${CURRENT_TOTAL_PATHS} - ${BASELINE_TOTAL_PATHS} = $((CURRENT_TOTAL_PATHS - BASELINE_TOTAL_PATHS)))"
fi

echo ""

# 4. Type-Level Changes Analysis
echo "🔍 TYPE-LEVEL CHANGES"

# Create temporary files for analysis
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Extract type names and key properties
extract_type_guide "$BASELINE_FILE" | jq -r '.[] | .type_name // .type // "unknown"' | sort > "$TEMP_DIR/baseline_types"
extract_type_guide "$CURRENT_FILE" | jq -r '.[] | .type_name // .type // "unknown"' | sort > "$TEMP_DIR/current_types"

# Find new types
NEW_TYPES=$(comm -13 "$TEMP_DIR/baseline_types" "$TEMP_DIR/current_types")
NEW_COUNT=$(echo "$NEW_TYPES" | grep -v "^$" | wc -l | tr -d ' ')

# Find removed types  
REMOVED_TYPES=$(comm -23 "$TEMP_DIR/baseline_types" "$TEMP_DIR/current_types")
REMOVED_COUNT=$(echo "$REMOVED_TYPES" | grep -v "^$" | wc -l | tr -d ' ')

# Find common types for detailed comparison
COMMON_TYPES=$(comm -12 "$TEMP_DIR/baseline_types" "$TEMP_DIR/current_types")

# Check for mutation path changes in common types
MODIFIED_COUNT=0
MODIFIED_TYPES=""
FORMAT_CHANGES=""

while read -r type_name; do
    if [ -n "$type_name" ]; then
        # Extract FULL type data and compare (not just path keys)
        BASELINE_TYPE=$(extract_type_guide "$BASELINE_FILE" | jq -c --arg t "$type_name" '
            .[] | select((.type_name // .type // "unknown") == $t)
        ')
        
        CURRENT_TYPE=$(extract_type_guide "$CURRENT_FILE" | jq -c --arg t "$type_name" '
            .[] | select((.type_name // .type // "unknown") == $t)
        ')
        
        # Check if the full type data differs (this catches value changes too)
        if [ "$BASELINE_TYPE" != "$CURRENT_TYPE" ]; then
            MODIFIED_COUNT=$((MODIFIED_COUNT + 1))
            if [ -z "$MODIFIED_TYPES" ]; then
                MODIFIED_TYPES="$type_name"
            else
                MODIFIED_TYPES="$MODIFIED_TYPES\n$type_name"
            fi
        fi
        
        # Check for format changes in examples (specifically looking for Vec3-like changes)
        # This is where we detect if examples changed from object to array format
        if echo "$type_name" | grep -q "TestComplexComponent\|Vec3\|Quat"; then
            # Extract and compare example formats for key paths
            BASELINE_EXAMPLE=$(extract_type_guide "$BASELINE_FILE" | jq --arg t "$type_name" '
                .[] | select((.type_name // .type // "unknown") == $t) | 
                if .mutation_paths | type == "object" then
                    .mutation_paths | to_entries | .[0].value.example
                else
                    null
                end
            ')
            
            CURRENT_EXAMPLE=$(extract_type_guide "$CURRENT_FILE" | jq --arg t "$type_name" '
                .[] | select((.type_name // .type // "unknown") == $t) | 
                if .mutation_paths | type == "object" then
                    .mutation_paths | to_entries | .[0].value.example
                else
                    null
                end
            ')
            
            if [ "$BASELINE_EXAMPLE" != "$CURRENT_EXAMPLE" ] && [ "$BASELINE_EXAMPLE" != "null" ]; then
                FORMAT_CHANGES="${FORMAT_CHANGES}   ├─ $type_name: Example format changed\n"
            fi
        fi
    fi
done <<< "$COMMON_TYPES"

# Display type-level changes
echo "   ├─ Modified Types: $MODIFIED_COUNT"
if [ "$MODIFIED_COUNT" -gt 0 ]; then
    echo -e "$MODIFIED_TYPES" | head -5 | while read -r type; do
        [ -n "$type" ] && echo "   │  ├─ $type: mutation paths changed"
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

# 5. Deep Structural Analysis (if differences found)
# Use Python script for detailed structural comparison
SCRIPT_DIR="$(dirname "$0")"
if [ "$MODIFIED_COUNT" -gt 0 ] || [ "$NEW_COUNT" -gt 0 ] || [ "$REMOVED_COUNT" -gt 0 ]; then
    echo "🔬 DEEP STRUCTURAL ANALYSIS"
    echo "─────────────────────────────────────────────"
    
    # Run the Python deep comparison script
    if [ -f "$SCRIPT_DIR/create_mutation_test_json_deep_comparison.py" ]; then
        python3 "$SCRIPT_DIR/create_mutation_test_json_deep_comparison.py" "$BASELINE_FILE" "$CURRENT_FILE"
    else
        # Fallback to basic format detection if Python script not available
        if [ -n "$FORMAT_CHANGES" ]; then
            echo "🔄 FORMAT CHANGES DETECTED"
            echo -e "$FORMAT_CHANGES"
        fi
        
        # Check TestComplexComponent specifically for Vec3 format
        check_vec3_format() {
            local file="$1"
            local label="$2"
            
            local example=$(extract_type_guide "$file" | jq -r '
                .[] | select((.type_name // .type // "unknown") == "extras_plugin::TestComplexComponent") |
                if .mutation_paths | type == "object" then
                    .mutation_paths.".points[0]".example // "not found"
                else
                    "format not recognized"
                end
            ')
            
            echo "   $label Vec3 in TestComplexComponent.points[0]: $example"
        }
        
        echo "🔎 CRITICAL TYPE VERIFICATION"
        check_vec3_format "$BASELINE_FILE" "Baseline"
        check_vec3_format "$CURRENT_FILE" "Current"
    fi
    echo ""
elif [ "$BASELINE_COUNT" != "$CURRENT_COUNT" ] || [ "$BASELINE_SPAWN" != "$CURRENT_SPAWN" ] || [ "$BASELINE_MUTATIONS" != "$CURRENT_MUTATIONS" ] || [ "$BASELINE_TOTAL_PATHS" != "$CURRENT_TOTAL_PATHS" ]; then
    # Metadata-only changes
    echo "📋 CHANGE ASSESSMENT"
    echo "   └─ Metadata changes only - no structural differences"
    echo ""
    echo "💡 RECOMMENDATION"
    echo "   Changes appear to be metadata only. Safe to promote if expected."
else
    # No changes at all (shouldn't happen as we check identity earlier)
    echo "📋 CHANGE ASSESSMENT"
    echo "   └─ No changes detected"
    echo ""
    echo "💡 RECOMMENDATION"
    echo "   Files are effectively identical. Safe to promote."
fi