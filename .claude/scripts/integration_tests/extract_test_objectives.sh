#!/bin/bash
# Extract test objectives from multiple test files
# Usage: extract_test_objectives.sh file1.md file2.md ...
# Output: One objective per line (or "N/A" if not found)

for file in "$@"; do
    if [ -f "$file" ]; then
        grep -A 1 "^## Objective" "$file" 2>/dev/null | tail -1 || echo "N/A"
    else
        echo "N/A"
    fi
done
