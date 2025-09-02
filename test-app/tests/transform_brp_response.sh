#!/bin/bash

# Transform BRP Type Schema Response to Type Validation JSON
# Usage: ./transform_brp_response.sh <input_file> <output_file> [exclusions_file]
#
# Converts the BRP type schema response into the type validation format
# with spawn support detection, mutation paths, and test status fields.
# Optional exclusions_file filters out problematic types.

set -e

# Check arguments
if [ $# -lt 2 ] || [ $# -gt 3 ]; then
    echo "Usage: $0 <input_file> <output_file> [exclusions_file]"
    echo "  input_file:      Path to BRP type schema response JSON"
    echo "  output_file:     Path to output type_validation.json file"
    echo "  exclusions_file: Optional path to excluded types file"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_FILE="$2"
EXCLUSIONS_FILE="${3:-}"

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file '$INPUT_FILE' not found!"
    exit 1
fi

# Build exclusions filter for jq
if [ -n "$EXCLUSIONS_FILE" ] && [ -f "$EXCLUSIONS_FILE" ]; then
    # Extract non-comment, non-empty lines and build jq filter
    EXCLUSIONS=$(grep -v '^#' "$EXCLUSIONS_FILE" 2>/dev/null | grep -v '^$' | jq -R . | jq -s .)
    echo "✓ Loaded exclusions from $EXCLUSIONS_FILE"
else
    EXCLUSIONS="[]"
fi

# Transform the BRP response to type validation format with exclusion filtering
jq --argjson exclusions "$EXCLUSIONS" '
.type_info | to_entries | [.[] | . as $item | .key as $idx | 
  select(.value.type_name as $type | $exclusions | map(. == $type) | any | not) | 
  # Check if type supports mutation operations
  ((.value.supported_operations // []) | contains(["mutate"])) as $supports_mutate |
  # Get mutation paths only if mutation is supported, and filter out NotMutatable
  (if $supports_mutate then 
    ((.value.mutation_paths // {}) | to_entries | map(select(.value.path_kind != "NotMutatable")) | map(.key))
   else [] end) as $mutation_paths |
  # Check spawn support
  ((.value.supported_operations // []) | contains(["spawn", "insert"])) as $has_spawn_support |
  # Determine test status: auto-pass if no spawn support AND no mutation paths
  (if $has_spawn_support or ($mutation_paths | length > 0) then "untested" else "passed" end) as $test_status |
  {
  type: .value.type_name,
  spawn_support: (if $has_spawn_support then "supported" else "not_supported" end),
  mutation_paths: $mutation_paths,
  test_status: $test_status,
  batch_number: null,
  fail_reason: ""
}]' "$INPUT_FILE" > "$OUTPUT_FILE"

echo "✓ Successfully transformed BRP response"
echo "  Input:  $INPUT_FILE"
echo "  Output: $OUTPUT_FILE"

# Quick stats
TOTAL=$(jq 'length' "$OUTPUT_FILE")
SPAWN_SUPPORTED=$(jq '[.[] | select(.spawn_support == "supported")] | length' "$OUTPUT_FILE")
WITH_MUTATIONS=$(jq '[.[] | select(.mutation_paths | length > 0)] | length' "$OUTPUT_FILE")
REQUIRES_TESTING=$(jq '[.[] | select(.test_status == "untested")] | length' "$OUTPUT_FILE")
AUTO_PASSED=$(jq '[.[] | select(.test_status == "passed")] | length' "$OUTPUT_FILE")

echo ""
echo "Summary:"
echo "  Total types: $TOTAL"
echo "  Spawn-supported: $SPAWN_SUPPORTED"
echo "  With mutations: $WITH_MUTATIONS"
echo "  Requires testing: $REQUIRES_TESTING"
echo "  Auto-passed: $AUTO_PASSED"