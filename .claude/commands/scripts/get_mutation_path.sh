#!/bin/bash

# Extract a specific mutation path from the baseline file
# Usage: ./get_mutation_path.sh TYPE_NAME MUTATION_PATH
# Example: ./get_mutation_path.sh "bevy_ui::ui_node::BoxShadow" ".0[0].color"

BASELINE_FILE="${TMPDIR}/all_types_baseline.json"

if [ ! -f "$BASELINE_FILE" ]; then
    echo "❌ Baseline file not found: $BASELINE_FILE"
    exit 1
fi

if [ $# -eq 0 ]; then
    echo "Usage: $0 TYPE_NAME [MUTATION_PATH]"
    echo ""
    echo "Examples:"
    echo "  $0 \"bevy_ui::ui_node::BoxShadow\"              # List all mutation paths"
    echo "  $0 \"bevy_ui::ui_node::BoxShadow\" \".0[0].color\" # Get specific path"
    exit 1
fi

TYPE_NAME="$1"
MUTATION_PATH="${2:-}"

# Python script to extract the mutation path
python3 <<EOF
import json
import sys

type_name = """$TYPE_NAME"""
mutation_path = """$MUTATION_PATH"""

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
            print("Please run the command again with either:")
            print("  - The number of your choice (1, 2, 3, etc.)")
            print("  - The full type path")
            sys.exit(1)
        else:
            # Check if it's a number selection from a previous disambiguation
            if type_name.isdigit():
                print("❌ Number selection is not supported in this context")
                print("Please use the full type path")
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

    if not mutation_path:
        # List all available mutation paths
        print(f"Available mutation paths for {type_name}:")
        print(f"Total paths: {len(mutation_paths)}")
        print()

        # Show first 20 paths as examples
        for i, path in enumerate(list(mutation_paths.keys())[:20]):
            if path == "":
                print('  ""  (root path)')
            else:
                print(f'  "{path}"')

        if len(mutation_paths) > 20:
            print(f"  ... and {len(mutation_paths) - 20} more paths")

        print()
        print("Run again with a specific path to see its details.")
    else:
        # Get specific mutation path
        if mutation_path not in mutation_paths:
            print(f"❌ Mutation path not found: {mutation_path}")
            print()
            print("Available paths that contain '{}':".format(mutation_path))
            matching = [p for p in mutation_paths.keys() if mutation_path in p]
            for p in matching[:10]:
                print(f'  "{p}"')
            if len(matching) > 10:
                print(f"  ... and {len(matching) - 10} more")
            sys.exit(1)

        path_data = mutation_paths[mutation_path]

        # Create a comprehensive JSON output
        output = {
            "type": type_name,
            "path": mutation_path,
            "data": path_data
        }

        # Output the raw formatted JSON
        print(json.dumps(output, indent=2))

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