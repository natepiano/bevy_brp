#!/bin/bash

# Mutation Test Statistics Reporter
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

# Generate comprehensive statistics
echo "✅ Mutation Test Statistics"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Basic counts
TOTAL=$(jq 'length' "$JSON_FILE")
SPAWN_SUPPORTED=$(jq '[.[] | select(.spawn_support == "supported")] | length' "$JSON_FILE")
SPAWN_NOT_SUPPORTED=$(jq '[.[] | select(.spawn_support == "not_supported")] | length' "$JSON_FILE")
WITH_MUTATIONS=$(jq '[.[] | select(.mutation_paths | length > 0)] | length' "$JSON_FILE")
NO_MUTATIONS=$(jq '[.[] | select(.mutation_paths | length == 0)] | length' "$JSON_FILE")
NO_CAPABILITIES=$(jq '[.[] | select(.spawn_support == "not_supported" and (.mutation_paths | length == 0))] | length' "$JSON_FILE")

echo "Capability Summary:"
echo "  Total types: $TOTAL"
echo "  Spawn-supported types: $SPAWN_SUPPORTED"
echo "  Non-spawn types: $SPAWN_NOT_SUPPORTED"
echo "  Types with mutations: $WITH_MUTATIONS"
echo "  Types without mutations: $NO_MUTATIONS"
echo "  Types with no capabilities: $NO_CAPABILITIES"
echo ""

# Test status counts
UNTESTED=$(jq '[.[] | select(.test_status == "untested")] | length' "$JSON_FILE")
PASSED=$(jq '[.[] | select(.test_status == "passed")] | length' "$JSON_FILE")
FAILED=$(jq '[.[] | select(.test_status == "failed")] | length' "$JSON_FILE")

echo "Test Status:"
echo "  Untested: $UNTESTED"
echo "  Passed: $PASSED"
echo "  Failed: $FAILED"
echo ""

# Batch information
MAX_BATCH=$(jq '[.[] | select(.batch_number != null) | .batch_number] | max // 0' "$JSON_FILE")
TYPES_WITH_BATCH=$(jq '[.[] | select(.batch_number != null)] | length' "$JSON_FILE")

if [ "$MAX_BATCH" -gt 0 ]; then
    echo "Batch Information:"
    echo "  Total batches: $MAX_BATCH"
    echo "  Types assigned to batches: $TYPES_WITH_BATCH"
    
    # Show batch distribution
    echo ""
    echo "  Batch distribution:"
    for i in $(seq 1 "$MAX_BATCH"); do
        COUNT=$(jq "[.[] | select(.batch_number == $i)] | length" "$JSON_FILE")
        printf "    Batch %2d: %d types\n" "$i" "$COUNT"
    done
    echo ""
fi

# Progress calculation
if [ "$TOTAL" -gt 0 ]; then
    PROGRESS=$(echo "scale=1; ($PASSED * 100) / $TOTAL" | bc)
    echo "Progress: ${PROGRESS}% complete"
    
    # Progress bar
    BAR_LENGTH=30
    FILLED=$(echo "scale=0; ($PASSED * $BAR_LENGTH) / $TOTAL" | bc)
    EMPTY=$((BAR_LENGTH - FILLED))
    
    echo -n "  ["
    [ "$FILLED" -gt 0 ] && printf '%.0s=' $(seq 1 "$FILLED")
    [ "$EMPTY" -gt 0 ] && printf '%.0s-' $(seq 1 "$EMPTY")
    echo "] $PASSED/$TOTAL"
fi

# Show failed types if any
if [ "$FAILED" -gt 0 ]; then
    echo ""
    echo "Failed Types:"
    jq -r '.[] | select(.test_status == "failed") | "  - \(.type): \(.fail_reason)"' "$JSON_FILE"
fi