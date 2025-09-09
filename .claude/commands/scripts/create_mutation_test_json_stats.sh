#!/bin/bash

# Mutation Test Statistics Reporter for FULL SCHEMA format
# Usage: ./type_stats.sh [json_file]
#
# Reports statistics about the mutation test tracking file.
# If no file is specified, uses the default location in $TMPDIR.

set -e

# Use provided file or default
JSON_FILE="${1:-$TMPDIR/all_types.json}"

# Check if the JSON file exists
if [ ! -f "$JSON_FILE" ]; then
    echo "Error: $JSON_FILE not found!"
    exit 1
fi

# Helper function to extract type_info array from either format
extract_type_info() {
    # Handle both wrapped (with type_info at root) and direct array formats
    jq '
        if .type_info then
            .type_info
        elif .result.type_info then
            .result.type_info
        else
            .
        end
    ' "$JSON_FILE"
}

# Generate comprehensive statistics
echo "✅ Mutation Test Statistics (Full Schema Format)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Basic counts
TOTAL=$(extract_type_info | jq 'length')
SPAWN_SUPPORTED=$(extract_type_info | jq '[.[] | select(has("spawn_format"))] | length')
SPAWN_NOT_SUPPORTED=$(extract_type_info | jq '[.[] | select(has("spawn_format") | not)] | length')
WITH_MUTATIONS=$(extract_type_info | jq '[.[] | select(.mutation_paths != null and .mutation_paths != {} and .mutation_paths != [])] | length')
NO_MUTATIONS=$(extract_type_info | jq '[.[] | select(.mutation_paths == null or .mutation_paths == {} or .mutation_paths == [])] | length')
NO_CAPABILITIES=$(extract_type_info | jq '[.[] | select((has("spawn_format") | not) and (.mutation_paths == null or .mutation_paths == {} or .mutation_paths == []))] | length')

echo "Capability Summary:"
echo "  Total types: $TOTAL"
echo "  Spawn-supported types: $SPAWN_SUPPORTED"
echo "  Non-spawn types: $SPAWN_NOT_SUPPORTED"
echo "  Types with mutations: $WITH_MUTATIONS"
echo "  Types without mutations: $NO_MUTATIONS"
echo "  Types with no capabilities: $NO_CAPABILITIES"
echo ""

# Test status counts
UNTESTED=$(extract_type_info | jq '[.[] | select(.test_status == "untested")] | length')
PASSED=$(extract_type_info | jq '[.[] | select(.test_status == "passed")] | length')
FAILED=$(extract_type_info | jq '[.[] | select(.test_status == "failed")] | length')

echo "Test Status:"
echo "  Untested: $UNTESTED"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"
PROGRESS=$(awk "BEGIN {printf \"%.1f\", ($PASSED + $FAILED) * 100.0 / $TOTAL}")
echo "  Progress: $PROGRESS% tested"
echo ""

# Batch information
MAX_BATCH=$(extract_type_info | jq '[.[] | select(.batch_number != null) | .batch_number] | max // 0')
TYPES_IN_BATCHES=$(extract_type_info | jq '[.[] | select(.batch_number != null)] | length')

echo "Batch Information:"
echo "  Types assigned to batches: $TYPES_IN_BATCHES"
echo "  Total batches: $MAX_BATCH"
if [ "$MAX_BATCH" -gt 0 ]; then
    AVG_PER_BATCH=$(awk "BEGIN {printf \"%.1f\", $TYPES_IN_BATCHES / $MAX_BATCH}")
    echo "  Average types per batch: $AVG_PER_BATCH"
fi
echo ""

# Progress tracking
if [ "$UNTESTED" -gt 0 ]; then
    echo "Next Steps:"
    echo "  - $UNTESTED types remain untested"
    if [ "$MAX_BATCH" -gt 0 ]; then
        echo "  - Run mutation tests for batches 1-$MAX_BATCH"
    else
        echo "  - Assign batch numbers using mutation_test_renumber_batches.sh"
    fi
else
    echo "✨ All types have been tested!"
fi
echo ""

# Schema completeness check (NEW for full schema format)
echo "Schema Completeness:"
HAS_EXAMPLES=$(extract_type_info | jq '[.[] | select(.mutation_paths != null and .mutation_paths != {} and .mutation_paths != []) | 
    if .mutation_paths | type == "object" then
        .mutation_paths | to_entries | .[0].value | has("example")
    else
        false
    end] | any')

if [ "$HAS_EXAMPLES" = "true" ]; then
    echo "  ✅ Full schemas with examples preserved"
else
    echo "  ⚠️  Warning: Examples may be missing (check file format)"
fi

# File format detection
if jq -e '.type_info' "$JSON_FILE" > /dev/null 2>&1; then
    echo "  Format: Full BRP schema (type_info at root)"
elif jq -e '.result.type_info' "$JSON_FILE" > /dev/null 2>&1; then
    echo "  Format: Full BRP schema (result.type_info)"
elif jq -e '.[0].type' "$JSON_FILE" > /dev/null 2>&1; then
    echo "  Format: Legacy array format"
else
    echo "  Format: Unknown"
fi