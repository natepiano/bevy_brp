#!/usr/bin/env python3
"""
Get subagent assignments for mutation testing.
Distributes batch types evenly across subagents with complete type data.

Usage:
  # Get all assignments for a batch (main agent)
  python3 mutation_test_get_subagent_assignments.py --batch 1 --max-subagents 10 --types-per-subagent 2

  # Get single assignment (subagent)
  python3 mutation_test_get_subagent_assignments.py --batch 1 --max-subagents 10 --types-per-subagent 2 --subagent-index 4
"""
import json
import sys
import os
import argparse
from typing import Any, TypedDict, cast

# Type definitions for JSON structures
class MutationPathData(TypedDict, total=False):
    description: str
    example: Any  # pyright: ignore[reportExplicitAny] - arbitrary JSON value
    path_info: dict[str, str]

class TypeData(TypedDict):
    type_name: str  # Required field
    # Optional fields
    spawn_format: Any | None  # pyright: ignore[reportExplicitAny] - arbitrary JSON structure
    mutation_paths: dict[str, MutationPathData] | None
    supported_operations: list[str] | None
    has_serialize: bool | None
    has_deserialize: bool | None
    in_registry: bool | None
    schema_info: dict[str, Any] | None  # pyright: ignore[reportExplicitAny] - JSON schema
    batch_number: int | None

class TypeGuideRoot(TypedDict):
    type_guide: dict[str, TypeData]

class SubagentAssignment(TypedDict):
    subagent: int
    port: int
    types: list[TypeData]

class SingleSubagentOutput(TypedDict):
    batch_number: int
    subagent_index: int
    subagent_number: int
    port: int
    types: list[TypeData]

class AllAssignmentsOutput(TypedDict):
    batch_number: int
    max_subagents: int
    types_per_subagent: int
    total_types: int
    assignments: list[SubagentAssignment]

# Parse command line arguments
parser = argparse.ArgumentParser(description='Get subagent assignments for mutation testing')
_ = parser.add_argument('--batch', type=int, required=True,
                        help='Batch number to get assignments for')
_ = parser.add_argument('--max-subagents', type=int, required=True,
                        help='Maximum number of subagents')
_ = parser.add_argument('--types-per-subagent', type=int, required=True,
                        help='Number of types each subagent should test')
_ = parser.add_argument('--subagent-index', type=int, required=False,
                        help='Optional: Get assignment for specific subagent (0-based index)')

args = parser.parse_args()

batch_num: int = cast(int, args.batch)
max_subagents: int = cast(int, args.max_subagents)
types_per_subagent: int = cast(int, args.types_per_subagent)
subagent_index: int | None = cast(int | None, args.subagent_index)

if max_subagents <= 0:
    print(f"Error: max_subagents must be positive, got: {max_subagents}", file=sys.stderr)
    sys.exit(1)

if types_per_subagent <= 0:
    print(f"Error: types_per_subagent must be positive, got: {types_per_subagent}", file=sys.stderr)
    sys.exit(1)

if subagent_index is not None:
    if subagent_index < 0 or subagent_index >= max_subagents:
        print(f"Error: subagent_index must be in range [0, {max_subagents}), got: {subagent_index}", file=sys.stderr)
        sys.exit(1)

# Get the JSON file path from .claude/transient
json_file = '.claude/transient/all_types.json'

if not os.path.exists(json_file):
    print(f"Error: {json_file} not found!", file=sys.stderr)
    sys.exit(1)

try:
    with open(json_file, 'r') as f:
        data = cast(TypeGuideRoot, json.load(f))
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Expect type_guide at root
if 'type_guide' not in data:
    print(f"Error: Expected dict with 'type_guide' at root", file=sys.stderr)
    sys.exit(1)

type_guide: dict[str, TypeData] = data['type_guide']

# Get types for the specified batch - type_guide is a dict
batch_types: list[TypeData] = []
for type_name, type_info in type_guide.items():
    if type_info.get('batch_number') == batch_num:
        # Add type_name to the dict for consistency
        type_item: TypeData = cast(TypeData, cast(object, {'type_name': type_name, **type_info}))
        batch_types.append(type_item)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)

# Calculate total types needed
total_types_needed: int = max_subagents * types_per_subagent

# Check if we have enough types in this batch
if len(batch_types) < total_types_needed:
    print(f"Error: Batch {batch_num} has {len(batch_types)} types, but need {total_types_needed} ({max_subagents} subagents × {types_per_subagent} types each)", file=sys.stderr)
    sys.exit(1)

# Take only the types we need for this configuration
batch_types = batch_types[:total_types_needed]

# Distribute types across subagents
assignments: list[SubagentAssignment] = []
for subagent_num in range(1, max_subagents + 1):
    start_index = (subagent_num - 1) * types_per_subagent
    end_index = start_index + types_per_subagent

    subagent_types: list[TypeData] = []
    for type_item in batch_types[start_index:end_index]:
        type_data: TypeData = cast(TypeData, cast(object, {
            'type_name': type_item['type_name'],
            'spawn_format': type_item.get('spawn_format'),
            'mutation_paths': type_item.get('mutation_paths'),
            'supported_operations': type_item.get('supported_operations'),
            'has_serialize': type_item.get('has_serialize'),
            'has_deserialize': type_item.get('has_deserialize'),
            'in_registry': type_item.get('in_registry'),
            'schema_info': type_item.get('schema_info')
        }))
        subagent_types.append(type_data)

    assignment: SubagentAssignment = {
        'subagent': subagent_num,
        'port': 30000 + subagent_num,
        'types': subagent_types
    }
    assignments.append(assignment)

# Check if we're returning a single subagent assignment or all assignments
if subagent_index is not None:
    # Return single assignment for the specified subagent
    # subagent_index is 0-based, but subagent numbers are 1-based
    subagent_num: int = subagent_index + 1

    # Find the assignment for this subagent
    for assignment in assignments:
        if assignment['subagent'] == subagent_num:
            # Output format for single subagent
            single_output: SingleSubagentOutput = {
                'batch_number': batch_num,
                'subagent_index': subagent_index,
                'subagent_number': subagent_num,
                'port': assignment['port'],
                'types': assignment['types']
            }
            print(json.dumps(single_output, indent=2))
            sys.exit(0)

    # Should not reach here if validation was correct
    print(f"Error: Could not find assignment for subagent index {subagent_index}", file=sys.stderr)
    sys.exit(1)
else:
    # Return all assignments (original behavior for main agent)
    output: AllAssignmentsOutput = {
        'batch_number': batch_num,
        'max_subagents': max_subagents,
        'types_per_subagent': types_per_subagent,
        'total_types': len(batch_types),
        'assignments': assignments
    }
    print(json.dumps(output, indent=2))
