#!/usr/bin/env python3
"""
Get subagent assignments for mutation testing.
Distributes batch types evenly across subagents with complete type data.
Usage: python3 mutation_test_get_subagent_assignments.py <batch_number> <max_subagents> <types_per_subagent>
"""
import json
import sys
import os

if len(sys.argv) != 4:
    print("Usage: python3 mutation_test_get_subagent_assignments.py <batch_number> <max_subagents> <types_per_subagent>", file=sys.stderr)
    sys.exit(1)

try:
    batch_num = int(sys.argv[1])
    max_subagents = int(sys.argv[2])
    types_per_subagent = int(sys.argv[3])
except ValueError as e:
    print(f"Error: All arguments must be integers: {e}", file=sys.stderr)
    sys.exit(1)

if max_subagents <= 0:
    print(f"Error: max_subagents must be positive, got: {max_subagents}", file=sys.stderr)
    sys.exit(1)

if types_per_subagent <= 0:
    print(f"Error: types_per_subagent must be positive, got: {types_per_subagent}", file=sys.stderr)
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

# Get types for the specified batch - type_guide is a dict
batch_types = []
for type_name, type_data in type_guide.items():
    if type_data.get('batch_number') == batch_num:
        # Add type_name to the dict for consistency
        type_item = dict(type_data)
        type_item['type_name'] = type_name
        batch_types.append(type_item)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)

# Calculate total types needed
total_types_needed = max_subagents * types_per_subagent

# Check if we have enough types in this batch
if len(batch_types) < total_types_needed:
    print(f"Error: Batch {batch_num} has {len(batch_types)} types, but need {total_types_needed} ({max_subagents} subagents × {types_per_subagent} types each)", file=sys.stderr)
    sys.exit(1)

# Take only the types we need for this configuration
batch_types = batch_types[:total_types_needed]

# Distribute types across subagents
assignments = []
for subagent_num in range(1, max_subagents + 1):
    start_index = (subagent_num - 1) * types_per_subagent
    end_index = start_index + types_per_subagent

    subagent_types = []
    for type_item in batch_types[start_index:end_index]:
        type_data = {
            'type_name': type_item['type_name'],
            'spawn_format': type_item.get('spawn_format'),
            'mutation_paths': type_item.get('mutation_paths'),
            'supported_operations': type_item.get('supported_operations'),
            'has_serialize': type_item.get('has_serialize'),
            'has_deserialize': type_item.get('has_deserialize'),
            'in_registry': type_item.get('in_registry'),
            'schema_info': type_item.get('schema_info')
        }
        subagent_types.append(type_data)

    assignment = {
        'subagent': subagent_num,
        'port': 30000 + subagent_num,
        'types': subagent_types
    }
    assignments.append(assignment)

# Output the assignments as JSON
output = {
    'batch_number': batch_num,
    'max_subagents': max_subagents,
    'types_per_subagent': types_per_subagent,
    'total_types': len(batch_types),
    'assignments': assignments
}

print(json.dumps(output, indent=2))