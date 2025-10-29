#!/bin/bash

# Get a list of mutation paths for a type from the baseline file
# Usage: ./get_mutation_path_list.sh TYPE_NAME
# Example: ./get_mutation_path_list.sh "BoxShadow"

BASELINE_FILE=".claude/transient/all_types_baseline.json"

if [ ! -f "$BASELINE_FILE" ]; then
    echo "❌ Baseline file not found: $BASELINE_FILE"
    exit 1
fi

if [ $# -eq 0 ]; then
    echo "Usage: $0 TYPE_NAME"
    echo ""
    echo "Examples:"
    echo "  $0 BoxShadow"
    echo "  $0 bevy_ui::ui_node::BoxShadow"
    exit 1
fi

TYPE_NAME="$1"

# Python script to extract the mutation path list
python3 <<EOF
import json
import sys

type_name = """$TYPE_NAME"""

try:
    with open("$BASELINE_FILE", 'r') as f:
        data = json.load(f)

    # Extract type guide
    if 'type_guide' in data:
        type_guide = data['type_guide']
    else:
        type_guide = data

    # Find the type - support both short names and full paths
    found_type = None
    if type_name in type_guide:
        # Exact match with full path
        found_type = type_name
    else:
        # Try to match by short name (last segment after ::)
        type_name_lower = type_name.lower()
        matches = []
        for full_type_name in type_guide.keys():
            short_name = full_type_name.split('::')[-1]
            if short_name.lower() == type_name_lower:
                matches.append(full_type_name)

        if len(matches) == 1:
            found_type = matches[0]
        elif len(matches) > 1:
            print(f"Found {len(matches)} types matching '{type_name}':")
            print()
            for i, match in enumerate(matches, 1):
                print(f"{i}. {match}")
            print()
            print("Please run the command again with the full type path")
            sys.exit(1)
        else:
            print(f"❌ Type not found: {type_name}")
            sys.exit(1)

    if not found_type:
        print(f"❌ Type not found: {type_name}")
        sys.exit(1)

    type_name = found_type  # Use the full type name from here on
    type_data = type_guide[type_name]

    if 'mutation_paths' not in type_data:
        print(f"❌ No mutation paths for type: {type_name}")
        sys.exit(1)

    mutation_paths = type_data['mutation_paths']

    # Output just the paths, one per line
    for path_obj in mutation_paths:
        path = path_obj.get('path', '')
        if path == "":
            print('""')  # Show empty string clearly
        else:
            print(path)

except FileNotFoundError:
    print(f"❌ Could not read baseline file: $BASELINE_FILE")
    sys.exit(1)
except json.JSONDecodeError as e:
    print(f"❌ Invalid JSON in baseline file: {e}")
    sys.exit(1)
except Exception as e:
    print(f"❌ Error: {e}")
    sys.exit(1)
EOF