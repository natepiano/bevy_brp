#!/usr/bin/env python3
"""
Get types for mutation testing by batch assignment index.
Returns type names from the specified batch assignment WITHOUT allowing agent substitution.
Usage: python3 mutation_test_get_types_by_index.py <batch_number> <assignment_index>
"""
import json
import sys
import os

if len(sys.argv) != 3:
    print("Usage: python3 mutation_test_get_types_by_index.py <batch_number> <assignment_index>", file=sys.stderr)
    sys.exit(1)

try:
    batch_num = int(sys.argv[1])
    assignment_index = int(sys.argv[2])
except ValueError as e:
    print(f"Error: Both arguments must be integers: {e}", file=sys.stderr)
    sys.exit(1)

# Get the JSON file path from TMPDIR
tmpdir = os.environ.get('TMPDIR', '/tmp')
json_file = os.path.join(tmpdir, 'all_types.json')

if not os.path.exists(json_file):
    print(f"Error: {json_file} not found!", file=sys.stderr)
    sys.exit(1)

try:
    with open(json_file, 'r') as f:
        data = json.load(f)
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Expect type_guide at root
if not isinstance(data, dict) or 'type_guide' not in data:
    print(f"Error: Expected dict with 'type_guide' at root", file=sys.stderr)
    sys.exit(1)

type_guide = data['type_guide']

# Get types for the specified batch
batch_types = []
if isinstance(type_guide, list):
    # New format: type_guide is a list
    for item in type_guide:
        if item.get('batch_number') == batch_num:
            batch_types.append(item['type_name'])
else:
    # Old format: type_guide is a dict
    for type_name, type_data in type_guide.items():
        if type_data.get('batch_number') == batch_num:
            batch_types.append(type_name)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)

# Calculate which types this assignment index should get
# Each assignment gets exactly 1 type (TYPES_PER_SUBAGENT = 1)
types_per_assignment = 1
start_index = assignment_index * types_per_assignment
end_index = start_index + types_per_assignment

if start_index >= len(batch_types):
    print(f"Assignment index {assignment_index} out of range for batch {batch_num} (has {len(batch_types)} types)", file=sys.stderr)
    sys.exit(1)

# Get the types for this assignment
assigned_types = batch_types[start_index:end_index]

# Output the types as a JSON array
print(json.dumps(assigned_types))