#!/usr/bin/env python3
"""
Get FULL TYPE SCHEMAS for a specific batch number from the mutation test tracking file.
Returns complete type information including spawn_format and mutation_paths with examples.
Usage: python3 mutation_test_get_batch_types.py <batch_number>
"""
import json
import sys
import os

if len(sys.argv) != 2:
    print("Usage: python3 mutation_test_get_batch_types.py <batch_number>", file=sys.stderr)
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

# Handle different file structures (wrapped vs direct array)
type_guide = None
if isinstance(data, dict):
    if 'type_guide' in data:
        # Format with type_guide at root
        type_guide = data['type_guide']
    elif 'result' in data and 'type_guide' in data['result']:
        # Format with result.type_guide
        type_guide = data['result']['type_guide']
    else:
        # Unknown dict format, treat as empty
        type_guide = []
else:
    # Direct array format (legacy)
    type_guide = data

# Get complete type schemas for the specified batch
batch_types = []
for t in type_guide:
    if t.get('batch_number') == batch_num:
        # Extract the complete type information
        type_data = {
            'type_name': t.get('type_name') or t.get('type', 'unknown'),
            'spawn_format': t.get('spawn_format'),
            'mutation_paths': t.get('mutation_paths'),
            'supported_operations': t.get('supported_operations'),
            'has_serialize': t.get('has_serialize'),
            'has_deserialize': t.get('has_deserialize'),
            'in_registry': t.get('in_registry'),
            'schema_info': t.get('schema_info'),
            # Include test metadata
            'test_status': t.get('test_status'),
            'batch_number': t.get('batch_number'),
            'fail_reason': t.get('fail_reason', '')
        }
        batch_types.append(type_data)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(0)

# Output the complete batch information as JSON
# This provides full schema information to the mutation test runner
output = {
    'batch_number': batch_num,
    'type_count': len(batch_types),
    'types': batch_types
}

print(json.dumps(output, indent=2))