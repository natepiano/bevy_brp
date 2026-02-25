#!/bin/bash
# Extract test objectives from multiple test files
# Usage: extract_test_objectives.sh file1.md file2.md ...
# Output: One objective per line (or "N/A" if not found)

set -euo pipefail

for file in "$@"; do
    if [ -f "$file" ]; then
        awk '
            {
                line = $0
                sub(/\r$/, "", line)
                lower = tolower(line)

                # End objective parsing when the next h2 section starts.
                if (in_obj && line ~ /^##[[:space:]]+/) {
                    in_obj = 0
                    stop = 1
                }

                # Heading-only form: "## Objective", "## Objectives", optional trailing colon.
                if (!in_obj && !stop && lower ~ /^##[[:space:]]+objectives?([[:space:]]*:)?[[:space:]]*$/) {
                    in_obj = 1
                    next
                }

                # Inline form: "## Objective: text"
                if (!in_obj && !stop && lower ~ /^##[[:space:]]+objectives?[[:space:]]*:[[:space:]]+/) {
                    txt = line
                    sub(/^##[[:space:]]+[Oo]bjectives?[[:space:]]*:[[:space:]]*/, "", txt)
                    gsub(/^[[:space:]]+|[[:space:]]+$/, "", txt)
                    obj = txt
                    stop = 1
                    next
                }

                if (in_obj && !stop) {
                    txt = line
                    gsub(/^[[:space:]]+|[[:space:]]+$/, "", txt)

                    # Skip leading blank lines in the objective section.
                    if (txt == "" && !started) {
                        next
                    }

                    # Stop at first blank line after objective content begins.
                    if (txt == "" && started) {
                        stop = 1
                        in_obj = 0
                        next
                    }

                    started = 1
                    obj = (obj == "" ? txt : obj " " txt)
                }
            }
            END {
                gsub(/[[:space:]]+/, " ", obj)
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", obj)
                print (obj == "" ? "N/A" : obj)
            }
        ' "$file"
    else
        echo "N/A"
    fi
done
