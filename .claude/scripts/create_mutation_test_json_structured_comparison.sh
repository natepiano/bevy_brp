#!/bin/bash

# Simplified structured comparison wrapper - delegates to Python script
# Usage: create_mutation_test_json_structured_comparison.sh <baseline_file> <current_file>

set -e

if [ $# -ne 2 ]; then
    echo "Usage: $0 <baseline_file> <current_file>"
    exit 1
fi

BASELINE_FILE="$1"
CURRENT_FILE="$2"
SCRIPT_DIR="$(dirname "$0")"
PYTHON_SCRIPT="$SCRIPT_DIR/create_mutation_test_json_deep_comparison.py"

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

# Run the unified Python comparison
python3 "$PYTHON_SCRIPT" "$BASELINE_FILE" "$CURRENT_FILE"