#!/bin/bash

# Script to get type guide for a specific type from baseline file
# Usage:
#   ./get_type_guide.sh                - List all types (no argument)
#   ./get_type_guide.sh Transform      - Get type guide for Transform (case-insensitive)
#   ./get_type_guide.sh 2              - Select option 2 from multiple matches
#   ./get_type_guide.sh bevy_transform::components::transform::Transform - Full path

BASELINE_FILE="$TMPDIR/all_types_baseline.json"

# Check if baseline file exists
if [ ! -f "$BASELINE_FILE" ]; then
    echo "Error: Baseline file not found at $BASELINE_FILE"
    exit 1
fi

# If no arguments, list all types
if [ $# -eq 0 ]; then
    SEARCH_TERM=""
else
    SEARCH_TERM="$1"
fi

# Use Python for complex JSON processing and interaction
python3 -c "
import json
import sys
import re

search_term = '$SEARCH_TERM'

with open('$BASELINE_FILE', 'r') as f:
    data = json.load(f)

type_guides = data.get('type_guide', [])

# If no search term, list all types
if not search_term:
    # Create list of all types with short names
    all_types = []
    for guide in type_guides:
        type_name = guide.get('type_name', '')
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
for guide in type_guides:
    type_name = guide.get('type_name', '')

    # Check for exact full path match first
    if type_name.lower() == search_term.lower():
        matches = [guide]
        break

    # Check for short name match (last segment)
    short_name = type_name.split('::')[-1]
    if short_name.lower() == search_term.lower():
        matches.append(guide)

# Handle results
if not matches:
    print('No type was found')
    sys.exit(0)

if len(matches) > 1:
    print(f'Found {len(matches)} types matching \"{search_term}\":')
    print()
    for i, guide in enumerate(matches, 1):
        print(f'{i}. {guide[\"type_name\"]}')
    print()
    print('Please run the command again with either:')
    print('  - The number of your choice (1, 2, 3, etc.)')
    print('  - The full type path')
    sys.exit(0)

# Single match - output the type guide as JSON only
guide = matches[0]
# Output JSON result that the command can process
result = {
    'status': 'found',
    'type_name': guide['type_name'],
    'guide': guide
}
print(json.dumps(result))
"