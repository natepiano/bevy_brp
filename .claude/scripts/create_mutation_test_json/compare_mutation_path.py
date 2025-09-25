#!/usr/bin/env python3
"""
Compare a specific mutation path between baseline and current.
This replaces the broken shell script that can't find its dependencies.
"""

import json
import sys
from pathlib import Path

def get_mutation_path_data(file_path: Path, type_name: str, mutation_path: str):
    """Extract mutation path data from a file."""
    with open(file_path, 'r') as f:
        data = json.load(f)

    # Handle both wrapped and unwrapped formats
    if 'type_guide' in data:
        type_guide = data['type_guide']
    else:
        type_guide = data

    if type_name not in type_guide:
        return None

    type_data = type_guide[type_name]
    if 'mutation_paths' not in type_data:
        return None

    mutation_paths = type_data['mutation_paths']
    if mutation_path not in mutation_paths:
        return None

    return mutation_paths[mutation_path]

def main():
    if len(sys.argv) != 3:
        print("ERROR: Usage: compare_mutation_path.py \"TypeName\" \"mutation.path\"")
        sys.exit(1)

    type_name = sys.argv[1]
    mutation_path = sys.argv[2]

    baseline_file = Path('.claude/transient/all_types_baseline.json')
    current_file = Path('.claude/transient/all_types.json')

    if not baseline_file.exists():
        print(f"ERROR: Baseline file not found: {baseline_file}")
        sys.exit(1)

    if not current_file.exists():
        print(f"ERROR: Current file not found: {current_file}")
        sys.exit(1)

    baseline_data = get_mutation_path_data(baseline_file, type_name, mutation_path)
    current_data = get_mutation_path_data(current_file, type_name, mutation_path)

    if baseline_data is None and current_data is None:
        print(f"ERROR: Type '{type_name}' or path '{mutation_path}' not found in either file")
        sys.exit(1)

    if baseline_data == current_data:
        print("IDENTICAL")
    else:
        print("DIFFERENT")
        print("=== BASELINE ===")
        print(json.dumps(baseline_data, indent=2))
        print("=== CURRENT ===")
        print(json.dumps(current_data, indent=2))

if __name__ == '__main__':
    main()