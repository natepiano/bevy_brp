#!/bin/bash

# Validate arguments for get_guide_current command
# Usage: get_guide_current_validate_args.sh TYPE_NAME [MUTATION_PATH]

TYPE_NAME="$1"
MUTATION_PATH="$2"

# Check if TYPE_NAME is provided
if [[ -z "$TYPE_NAME" ]]; then
    echo "Error: TYPE_NAME is required"
    echo "Usage: /get_guide_current <type_name> [mutation_path]"
    echo "Example: /get_guide_current Transform"
    echo "Example: /get_guide_current Transform .translation"
    exit 1
fi

# Success - output parsed arguments for use
echo "TYPE_NAME=$TYPE_NAME"
if [[ -n "$MUTATION_PATH" ]]; then
    echo "MUTATION_PATH=$MUTATION_PATH"
else
    echo "MUTATION_PATH="
fi