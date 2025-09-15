#!/usr/bin/env python3
"""
Get simple batch assignments for mutation testing.
Returns only type names assigned to each subagent - no mutation paths.
Usage: python3 mutation_test_get_batch_assignments.py <batch_number>
"""
import json
import sys
import os

if len(sys.argv) != 2:
    print("Usage: python3 mutation_test_get_batch_assignments.py <batch_number>", file=sys.stderr)
    sys.exit(1)

try:
    batch_num = int(sys.argv[1])
except ValueError:
    print(f"Error: Batch number must be an integer, got: {sys.argv[1]}", file=sys.stderr)
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
# type_guide is a dict keyed by type names
for type_name, type_data in type_guide.items():
    if type_data.get('batch_number') == batch_num:
        batch_types.append(type_name)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(0)

# Create assignments - one assignment index per subagent
assignments = []
for i in range(len(batch_types)):
    assignments.append({
        'subagent': i + 1,
        'port': 30001 + i,
        'assignment_index': i  # Index used to retrieve types from batch
    })

# Output the assignments as JSON
output = {
    'batch_number': batch_num,
    'assignments': assignments
}

print(json.dumps(output, indent=2))