#!/usr/bin/env python3
"""
Get complete type guides with mutation paths for specified types.
Reads type names as JSON array from stdin, returns complete mutation paths.
Usage: echo '["type1", "type2"]' | python3 mutation_test_get_type_guides.py
"""
import json
import sys
import os

# Read type names from stdin
try:
    type_names = json.load(sys.stdin)
    if not isinstance(type_names, list):
        print("Error: Expected JSON array of type names on stdin", file=sys.stderr)
        sys.exit(1)
except json.JSONDecodeError as e:
    print(f"Error parsing input JSON: {e}", file=sys.stderr)
    sys.exit(1)

if not type_names:
    print("Error: No type names provided", file=sys.stderr)
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

# Find and return the complete type guides for requested types
result = []
for requested_type in type_names:
    # type_guide is a dict keyed by type names
    if requested_type in type_guide:
        t = type_guide[requested_type]
        # Return the complete type information
        type_data = {
            'type_name': requested_type,
            'spawn_format': t.get('spawn_format'),
            'mutation_paths': t.get('mutation_paths'),
            'supported_operations': t.get('supported_operations'),
            'has_serialize': t.get('has_serialize'),
            'has_deserialize': t.get('has_deserialize'),
            'in_registry': t.get('in_registry'),
            'schema_info': t.get('schema_info')
        }
        result.append(type_data)
    else:
        print(f"Warning: Type '{requested_type}' not found in all_types.json", file=sys.stderr)
        # Still include it with null data so subagent can report it properly
        result.append({
            'type_name': requested_type,
            'spawn_format': None,
            'mutation_paths': None,
            'supported_operations': [],
            'has_serialize': False,
            'has_deserialize': False,
            'in_registry': False,
            'schema_info': None
        })

# Output the complete type guides as JSON
print(json.dumps(result, indent=2))