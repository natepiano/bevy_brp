#!/bin/bash

# Validate arguments for compare_mutation_path command
# Usage: compare_mutation_path_validate_args.sh TYPE_NAME MUTATION_PATH

TYPE_NAME="$1"
MUTATION_PATH="$2"

# Check if both arguments are provided
if [[ -z "$TYPE_NAME" ]]; then
    echo "Error: TYPE_NAME is required"
    echo "Usage: TYPE_NAME MUTATION_PATH"
    echo "Example: bevy_ui::ui_node::Node .grid_template_columns"
    exit 1
fi

if [[ -z "$MUTATION_PATH" ]]; then
    echo "Error: MUTATION_PATH is required"
    echo "Usage: TYPE_NAME MUTATION_PATH"
    echo "Example: bevy_ui::ui_node::Node .grid_template_columns"
    exit 1
fi

# Validate TYPE_NAME contains :: namespace separators
if [[ ! "$TYPE_NAME" =~ :: ]]; then
    echo "Error: TYPE_NAME must be fully-qualified (contain ::)"
    echo "Example: bevy_ui::ui_node::Node"
    exit 1
fi

# Success - output parsed arguments for use
echo "TYPE_NAME=$TYPE_NAME"
echo "MUTATION_PATH=$MUTATION_PATH"