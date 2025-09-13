#!/bin/bash

# Script to analyze type_kind values in mutation paths from baseline file
# Usage:
#   ./get_type_kind.sh          - Show summary count of all type_kinds
#   ./get_type_kind.sh List      - Show all types containing List type_kind

BASELINE_FILE="$TMPDIR/all_types_baseline.json"

# Check if baseline file exists
if [ ! -f "$BASELINE_FILE" ]; then
    echo "Error: Baseline file not found at $BASELINE_FILE"
    exit 1
fi

if [ $# -eq 0 ]; then
    # No arguments - show summary of all type_kinds
    echo "Type kind summary (types containing at least one mutation path of each kind):"
    echo

    # Use Python for complex JSON processing
    python3 -c "
import json
from collections import Counter

with open('$BASELINE_FILE', 'r') as f:
    data = json.load(f)

type_guides = data.get('type_guide', [])

# Count types by type_kind
type_kind_counts = Counter()
types_by_kind = {}

for guide in type_guides:
    if 'mutation_paths' in guide and guide['mutation_paths']:
        type_kinds_found = set()
        for path, path_data in guide['mutation_paths'].items():
            if 'path_info' in path_data and 'type_kind' in path_data['path_info']:
                kind = path_data['path_info']['type_kind']
                type_kinds_found.add(kind)
                if kind not in types_by_kind:
                    types_by_kind[kind] = set()
                types_by_kind[kind].add(guide['type_name'])

# Count unique types per kind
for kind, types in types_by_kind.items():
    type_kind_counts[kind] = len(types)

# Display sorted by kind name
for kind in sorted(type_kind_counts.keys()):
    print(f'{kind}: {type_kind_counts[kind]}')
"

else
    # Argument provided - show types containing that specific type_kind
    TYPE_KIND="$1"

    echo "Types containing mutation paths with type_kind '$TYPE_KIND':"
    echo

    python3 -c "
import json

with open('$BASELINE_FILE', 'r') as f:
    data = json.load(f)

type_guides = data.get('type_guide', [])
target_kind = '$TYPE_KIND'
matching_types = set()

for guide in type_guides:
    if 'mutation_paths' in guide and guide['mutation_paths']:
        for path, path_data in guide['mutation_paths'].items():
            if 'path_info' in path_data and path_data['path_info'].get('type_kind') == target_kind:
                matching_types.add(guide['type_name'])
                break  # Found at least one, no need to check other paths

if matching_types:
    for type_name in sorted(matching_types):
        print(type_name)
else:
    print(f'No types found with type_kind: {target_kind}')
"
fi