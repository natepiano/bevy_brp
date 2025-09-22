#!/bin/bash

# Structured comparison wrapper with integrated categorization
# Usage: create_mutation_test_json_structured_comparison.sh [--detailed] <baseline_file> <current_file>
#
# Options:
#   --detailed  Generate detailed output to $TMPDIR/mutation_comparison_details.json
#
# This script runs the comparison and optionally categorizes the results against expected changes

set -e

# Check for --detailed flag
DETAILED_MODE=false
if [ "$1" = "--detailed" ]; then
    DETAILED_MODE=true
    shift
fi

if [ $# -ne 2 ]; then
    echo "Usage: $0 [--detailed] <baseline_file> <current_file>"
    exit 1
fi

BASELINE_FILE="$1"
CURRENT_FILE="$2"
SCRIPT_DIR="$(dirname "$0")"
PYTHON_SCRIPT="$SCRIPT_DIR/create_mutation_test_json_deep_comparison.py"
CATEGORIZE_SCRIPT="$SCRIPT_DIR/create_mutation_test_json_categorize_changes.py"
EXPECTED_CHANGES=".claude/types/create_mutation_test_json_expected_changes.json"

# Check if files exist
if [ ! -f "$BASELINE_FILE" ]; then
    echo "❌ Baseline file not found: $BASELINE_FILE"
    exit 1
fi

if [ ! -f "$CURRENT_FILE" ]; then
    echo "❌ Current file not found: $CURRENT_FILE"
    exit 1
fi

# Check if Python script exists
if [ ! -f "$PYTHON_SCRIPT" ]; then
    echo "❌ Python comparison script not found: $PYTHON_SCRIPT"
    exit 1
fi

# Create temp file for comparison output
TEMP_OUTPUT="${TMPDIR:-/tmp}/comparison_output_$$.txt"

# Run the unified Python comparison and save output
python3 "$PYTHON_SCRIPT" "$BASELINE_FILE" "$CURRENT_FILE" | tee "$TEMP_OUTPUT"

# If expected changes file exists and categorization script exists, run categorization
if [ -f "$EXPECTED_CHANGES" ] && [ -f "$CATEGORIZE_SCRIPT" ]; then
    echo ""
    echo "============================================================"
    echo "📊 CATEGORIZING CHANGES AGAINST EXPECTED PATTERNS"
    echo "============================================================"
    echo ""

    # Run categorization on the saved output
    python3 "$CATEGORIZE_SCRIPT" \
        --comparison-output "$TEMP_OUTPUT" \
        --expected-changes "$EXPECTED_CHANGES"
fi

# If detailed mode is requested, run the comparison again with --detailed flag to generate JSON output
if [ "$DETAILED_MODE" = true ]; then
    DETAILS_OUTPUT="$TMPDIR/mutation_comparison_details.json"
    echo ""
    echo "============================================================"
    echo "📊 GENERATING DETAILED COMPARISON DATA"
    echo "============================================================"
    echo ""

    # Run the comparison script with --detailed flag to generate JSON output
    python3 "$PYTHON_SCRIPT" "$BASELINE_FILE" "$CURRENT_FILE" --detailed

    if [ $? -eq 0 ]; then
        echo "✅ Detailed comparison data generated"
        echo "   File: $DETAILS_OUTPUT"
        echo "   Use 'Read $DETAILS_OUTPUT' to examine the detailed changes"
    else
        echo "❌ Failed to generate detailed comparison data"
    fi
fi

# Clean up temp file
rm -f "$TEMP_OUTPUT"