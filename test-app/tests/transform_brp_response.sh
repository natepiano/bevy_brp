#!/bin/bash

# Transform BRP Type Schema Response to Mutation Test JSON
# Usage: ./transform_brp_response.sh <input_file> <output_file>
#
# Converts the BRP type schema response into the mutation test format
# with spawn support detection, mutation paths, and test status fields.

set -e

# Check arguments
if [ $# -ne 2 ]; then
    echo "Usage: $0 <input_file> <output_file>"
    echo "  input_file:  Path to BRP type schema response JSON"
    echo "  output_file: Path to output mutation test JSON file"
    exit 1
fi

INPUT_FILE="$1"
OUTPUT_FILE="$2"

# Check if input file exists
if [ ! -f "$INPUT_FILE" ]; then
    echo "Error: Input file '$INPUT_FILE' not found!"
    exit 1
fi

# No exclusions - test all discovered types

# Transform the BRP response to mutation test format
jq '
.type_info | to_entries | [.[] | . as $item | .key as $idx | 
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