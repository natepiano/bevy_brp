#!/bin/bash

# Get list of excluded types from the exclusion file
# Returns a clean list without comments or empty lines

EXCLUSION_FILE="/Users/natemccoy/rust/bevy_brp/.claude/commands/scripts/mutation_test_excluded_types.txt"

# Extract non-comment, non-empty lines from the exclusion file
grep -v '^#' "$EXCLUSION_FILE" | grep -v '^$'