#!/usr/bin/env python3
"""
Get types for a specific batch number from the mutation test tracking file.
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

# Get types for the specified batch
batch_types = [t['type'] for t in data if t.get('batch_number') == batch_num]

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(0)

# Print each type on a new line
for t in batch_types:
    print(t)