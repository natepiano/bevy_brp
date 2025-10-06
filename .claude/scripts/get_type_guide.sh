#!/bin/bash

# Script to get type guide for a specific type from a type guide JSON file
# Usage:
#   ./get_type_guide.sh --file <path>                                    - List all types
#   ./get_type_guide.sh Transform --file <path>                          - Get type guide for Transform (case-insensitive)
#   ./get_type_guide.sh 2 --file <path>                                  - Select option 2 from multiple matches
#   ./get_type_guide.sh bevy_transform::components::transform::Transform --file <path> - Full path

# Parse arguments
SEARCH_TERM=""
TYPE_GUIDE_FILE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --file)
            TYPE_GUIDE_FILE="$2"
            shift 2
            ;;
        *)
            SEARCH_TERM="$1"
            shift
            ;;
    esac
done

# Validate required --file parameter
if [ -z "$TYPE_GUIDE_FILE" ]; then
    echo "Error: --file parameter is required"
    echo "Usage: $0 [type_name] --file <path_to_type_guide.json>"
    exit 1
fi

# Check if file exists
if [ ! -f "$TYPE_GUIDE_FILE" ]; then
    echo "Error: Type guide file not found at $TYPE_GUIDE_FILE"
    exit 1
fi

# Use Python for complex JSON processing and interaction
python3 -c "
import json
import sys
import re

search_term = '$SEARCH_TERM'

with open('$TYPE_GUIDE_FILE', 'r') as f:
    data = json.load(f)

type_guides = data.get('type_guide', {})

# If no search term, list all types
if not search_term:
    # Create list of all types with short names
    all_types = []
    for type_name, guide in type_guides.items():
        short_name = type_name.split('::')[-1]
        all_types.append({
            'short_name': short_name,
            'full_path': type_name
        })

    # Sort by short name
    all_types.sort(key=lambda x: x['short_name'].lower())

    # Output as JSON for command to process
    result = {
        'status': 'list_all',
        'types': all_types,
        'count': len(all_types)
    }
    print(json.dumps(result))
    sys.exit(0)

# Check if search term is a number (selection from previous multiple matches)
if search_term.isdigit():
    # This would be handled in a stateful way in practice
    print('Error: Number selection requires a previous search with multiple results')
    sys.exit(1)

# Find matching types
matches = []
for type_name, guide in type_guides.items():
    # Check for exact full path match first
    if type_name.lower() == search_term.lower():
        matches = [(type_name, guide)]
        break

    # Check for short name match (last segment)
    short_name = type_name.split('::')[-1]
    if short_name.lower() == search_term.lower():
        matches.append((type_name, guide))

# Handle results
if not matches:
    print('No type was found')
    sys.exit(0)

if len(matches) > 1:
    print(f'Found {len(matches)} types matching \"{search_term}\":')
    print()
    for i, (type_name, guide) in enumerate(matches, 1):
        print(f'{i}. {type_name}')
    print()
    print('Please run the command again with either:')
    print('  - The number of your choice (1, 2, 3, etc.)')
    print('  - The full type path')
    sys.exit(0)

# Single match - output the type guide as JSON only
type_name, guide = matches[0]
# Output JSON result that the command can process
result = {
    'status': 'found',
    'type_name': type_name,
    'guide': guide
}
print(json.dumps(result))
"