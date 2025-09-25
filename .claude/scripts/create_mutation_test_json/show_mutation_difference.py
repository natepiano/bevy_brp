#!/usr/bin/env python3
"""
Show the actual difference for a mutation path, including nested fields.
This shows what the comparison tool found.
"""

import json
import sys
from pathlib import Path

def extract_nested_field(data, field_path):
    """Extract a nested field from data using dot notation."""
    parts = field_path.split('.')
    current = data
    for part in parts:
        if current is None:
            return None
        if isinstance(current, dict) and part in current:
            current = current[part]
        else:
            return None
    return current

def main():
    if len(sys.argv) != 3:
        print("Usage: show_mutation_difference.py \"TypeName\" \"mutation.path\"")
        sys.exit(1)

    type_name = sys.argv[1]
    mutation_path = sys.argv[2]

    # Load the comparison file that has the actual differences
    comparison_file = Path('/var/folders/rf/twhh0jfd243fpltn5k0w1t980000gn/T/mutation_comparison_full.json')
    if not comparison_file.exists():
        print(f"ERROR: Comparison file not found: {comparison_file}")
        sys.exit(1)

    with open(comparison_file, 'r') as f:
        comparison = json.load(f)

    # Find all changes for this type and mutation path
    changes = []
    for change in comparison.get('unexpected_changes', []):
        if change['type_name'] == type_name and change['mutation_path'] == mutation_path:
            changes.append(change)

    if not changes:
        print(f"No changes found for {type_name} at path {mutation_path}")
        return

    print(f"## Mutation Path Comparison\n")
    print(f"**Type**: `{type_name}`")
    print(f"**Path**: `{mutation_path}`")
    print(f"**Changes Found**: {len(changes)}\n")

    # Group changes by the nested field that changed
    for change in changes:
        full_path = change['path']
        # Extract the nested field path after the mutation path
        # Format is like: mutation_paths..z_config.far_z_mode.example
        nested_part = full_path.replace(f'mutation_paths.{mutation_path}.', '')

        print(f"\n### Change in nested field: `{nested_part}`")
        print(f"**Change Type**: {change['change_type']}")
        print(f"**Description**: {change['description']}")

        print("\n```json")
        print("// BASELINE")
        if change['baseline'] is not None:
            print(json.dumps(change['baseline'], indent=2))
        else:
            print("(not present)")
        print("```\n")

        print("```json")
        print("// CURRENT")
        if change['current'] is not None:
            print(json.dumps(change['current'], indent=2))
        else:
            print("(not present)")
        print("```")

if __name__ == '__main__':
    main()