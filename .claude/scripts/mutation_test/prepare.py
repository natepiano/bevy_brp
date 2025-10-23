#!/usr/bin/env python3
"""
Mutation test preparation: batch renumbering and assignment generation.

This script combines two operations:
1. Batch renumbering (when batch=1): Reset failed tests and assign batch numbers
2. Assignment generation (all batches): Create test plans and distribute types

Usage:
  python3 mutation_test_prepare.py --batch 1 --max-subagents 10 --types-per-subagent 1

Output:
  Returns AllAssignmentsOutput with assignments and test plan files.
"""

import json
import sys
import os
import argparse
import tempfile
import glob
import subprocess
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
    in_registry: bool | None
    schema_info: dict[str, Any] | None  # pyright: ignore[reportExplicitAny] - JSON schema
    batch_number: int | None
    mutation_type: str | None  # "Component" or "Resource"
    test_status: str | None  # "untested", "passed", "failed"
    fail_reason: str | None


class TypeGuideRoot(TypedDict):
    type_guide: dict[str, TypeData]


class SubagentAssignment(TypedDict):
    subagent: int
    port: int
    window_description: str  # Pre-formatted window title
    task_description: str  # Pre-formatted task description
    test_plan_file: str  # Path to generated test plan file


class AllAssignmentsOutput(TypedDict):
    batch_number: int
    max_subagents: int
    types_per_subagent: int
    total_types: int
    progress_message: str  # Pre-formatted progress message for display
    assignments: list[SubagentAssignment]


# Test plan types
class TestOperation(TypedDict, total=False):
    operation_id: int  # Sequential ID for tracking operations
    tool: str  # MCP tool name
    # Common fields
    port: int
    status: str | None
    error: str | None
    retry_count: int
    operation_announced: bool
    # Spawn/query specific
    components: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    filter: dict[str, list[str]] | None
    data: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    # Mutation specific
    entity: str | int | None  # "USE_QUERY_RESULT" or actual entity ID
    component: str | None
    resource: str | None
    path: str | None
    value: Any  # pyright: ignore[reportExplicitAny]
    # Entity ID substitution
    entity_id_substitution: dict[str, str] | None


class TypeTest(TypedDict, total=False):
    type_name: str  # Required
    mutation_type: str  # Required: "Component" or "Resource"
    operations: list[TestOperation]  # Required
    part_number: int  # Optional: Which part of this type (1-indexed)
    total_parts: int  # Optional: Total parts for this type


class TestPlan(TypedDict):
    batch_number: int
    subagent_index: int
    port: int
    test_plan_file: str
    tests: list[TypeTest]


# Parse command line arguments
parser = argparse.ArgumentParser(
    description="Prepare mutation test: renumber batches and generate assignments"
)
_ = parser.add_argument(
    "--batch", type=int, required=True, help="Batch number to process"
)
_ = parser.add_argument(
    "--max-subagents", type=int, required=True, help="Maximum number of subagents"
)
_ = parser.add_argument(
    "--types-per-subagent",
    type=int,
    required=True,
    help="Number of types each subagent should test",
)

args = parser.parse_args()

batch_num: int = cast(int, args.batch)
max_subagents: int = cast(int, args.max_subagents)
types_per_subagent: int = cast(int, args.types_per_subagent)

if max_subagents <= 0:
    print(
        f"Error: max_subagents must be positive, got: {max_subagents}", file=sys.stderr
    )
    sys.exit(1)

if types_per_subagent <= 0:
    print(
        f"Error: types_per_subagent must be positive, got: {types_per_subagent}",
        file=sys.stderr,
    )
    sys.exit(1)

# Calculate batch size
batch_size = max_subagents * types_per_subagent

# Get the JSON file path
json_file = ".claude/transient/all_types.json"

if not os.path.exists(json_file):
    print(f"Error: {json_file} not found!", file=sys.stderr)
    sys.exit(1)


def renumber_batches(data: TypeGuideRoot, batch_size: int) -> TypeGuideRoot:
    """
    Renumber batches: reset failed tests to untested and assign batch numbers.
    This happens before every batch to ensure retry failures are picked up.

    Uses shared splitting logic to ensure batch numbers match what preparation will deliver.
    """
    type_guide = data["type_guide"]

    # Step 1: Reset failed tests to untested and clear their batch numbers
    for type_name, type_data in type_guide.items():
        if type_data.get("test_status") == "failed":
            type_data["test_status"] = "untested"
            type_data["fail_reason"] = ""
            type_data["batch_number"] = None

    # Step 2: Find highest batch number assigned to passed/auto-passed tests
    max_batch = 0
    for type_data in type_guide.values():
        if type_data.get("test_status") in ["passed", "auto_passed"]:
            batch_num = type_data.get("batch_number")
            if batch_num is not None and batch_num > max_batch:
                max_batch = batch_num

    # Step 3: Clear batch numbers only for untested types (retries + never tested)
    for type_data in type_guide.values():
        if type_data.get("test_status") == "untested":
            type_data["batch_number"] = None

    # Step 4: Assign batch numbers to untested types, starting after highest batch
    # Account for splitting: some types consume 2 slots instead of 1
    untested_types: list[tuple[str, TypeData]] = [
        (type_name, type_data)
        for type_name, type_data in type_guide.items()
        if type_data.get("test_status") == "untested"
    ]

    current_batch = max_batch + 1
    current_batch_slots = 0

    for type_name, type_data in untested_types:
        # Calculate slots needed for this type
        slots_needed = calculate_type_slots(type_data)

        # Check if this type fits in current batch
        if current_batch_slots + slots_needed > batch_size:
            # Start new batch
            current_batch += 1
            current_batch_slots = 0
            # Recalculate slots for new batch (may be different if split logic changes)
            slots_needed = calculate_type_slots(type_data)

        # Assign type to current batch
        type_guide[type_name]["batch_number"] = current_batch
        current_batch_slots += slots_needed

    # Report statistics
    total = len(type_guide)
    untested = len([t for t in type_guide.values() if t.get("test_status") == "untested"])
    failed = len([t for t in type_guide.values() if t.get("test_status") == "failed"])
    passed = len([t for t in type_guide.values() if t.get("test_status") == "passed"])
    max_batch = max(
        (t.get("batch_number") or 0 for t in type_guide.values()),
        default=0
    )

    print("✓ Batch renumbering complete!", file=sys.stderr)
    print("", file=sys.stderr)
    print("Statistics:", file=sys.stderr)
    print(f"  Total types: {total}", file=sys.stderr)
    print(f"  Passed: {passed}", file=sys.stderr)
    print(f"  Failed: {failed}", file=sys.stderr)
    print(f"  Untested: {untested}", file=sys.stderr)
    print(f"  Batches to process: {max_batch}", file=sys.stderr)
    print("", file=sys.stderr)

    return data


def extract_mutation_type(schema_info: dict[str, object] | None) -> str | None:
    """Extract mutation_type from schema_info.reflect_traits."""
    if not schema_info:
        return None

    reflect_traits = schema_info.get("reflect_traits")
    if not reflect_traits or not isinstance(reflect_traits, list):
        return None

    # Check for Component or Resource in reflect_traits
    if "Component" in reflect_traits:
        return "Component"
    if "Resource" in reflect_traits:
        return "Resource"

    return None


ENTITY_ID_PLACEHOLDER = 8589934670  # Placeholder entity ID used in spawn_format


def find_entity_id_placeholders(value: Any, path: str = "") -> dict[str, str]:  # pyright: ignore[reportExplicitAny,reportAny]
    """
    Recursively find entity ID placeholders in a value and return paths to them.
    Returns dict of path -> "QUERY_ENTITY" for substitution.

    Note: Uses Any type for recursive JSON traversal - unavoidable for arbitrary JSON structures.
    """
    substitutions: dict[str, str] = {}

    if isinstance(value, int) and value == ENTITY_ID_PLACEHOLDER:
        return {path: "QUERY_ENTITY"}
    elif isinstance(value, list):
        for i, item in enumerate(value):  # pyright: ignore[reportUnknownVariableType,reportUnknownArgumentType]
            item_path = f"{path}[{i}]" if path else f"[{i}]"
            substitutions.update(find_entity_id_placeholders(item, item_path))
    elif isinstance(value, dict):
        for key, val in value.items():  # pyright: ignore[reportUnknownVariableType]
            val_path: str = f"{path}.{key}" if path else str(key)  # pyright: ignore[reportUnknownArgumentType]
            substitutions.update(find_entity_id_placeholders(val, val_path))

    return substitutions


def generate_test_operations(type_data: TypeData, port: int) -> list[TestOperation]:
    """Generate test operations for a single type."""
    operations: list[TestOperation] = []
    type_name = type_data["type_name"]
    mutation_type = type_data.get("mutation_type")
    spawn_format = type_data.get("spawn_format")
    mutation_paths = type_data.get("mutation_paths") or {}

    # Step 1: Spawn or Insert (if spawn_format exists)
    if spawn_format is not None:
        if mutation_type == "Component":
            # Spawn entity with component
            op = cast(
                TestOperation,
                cast(
                    object,
                    {
                        "operation_id": len(operations),
                        "tool": "mcp__brp__world_spawn_entity",
                        "components": {type_name: spawn_format},
                        "port": port,
                        "status": None,
                        "error": None,
                        "retry_count": 0,
                        "operation_announced": False,
                    },
                ),
            )

            # Check for entity ID placeholders
            substitutions = find_entity_id_placeholders({type_name: spawn_format}, "")
            if substitutions:
                op["entity_id_substitution"] = substitutions

            operations.append(op)
        elif mutation_type == "Resource":
            # Insert resource
            op = cast(
                TestOperation,
                cast(
                    object,
                    {
                        "operation_id": len(operations),
                        "tool": "mcp__brp__world_insert_resources",
                        "resource": type_name,
                        "value": spawn_format,
                        "port": port,
                        "status": None,
                        "error": None,
                        "retry_count": 0,
                        "operation_announced": False,
                    },
                ),
            )

            # Check for entity ID placeholders
            substitutions = find_entity_id_placeholders(spawn_format, "")
            if substitutions:
                op["entity_id_substitution"] = substitutions

            operations.append(op)

    # Step 2: Query (components only)
    if mutation_type == "Component":
        operations.append(
            cast(
                TestOperation,
                cast(
                    object,
                    {
                        "operation_id": len(operations),
                        "tool": "mcp__brp__world_query",
                        "filter": {"with": [type_name]},
                        "data": {},
                        "port": port,
                        "status": None,
                        "error": None,
                        "operation_announced": False,
                    },
                ),
            )
        )

    # Step 3: Mutations
    # Track last root_example to avoid redundant root mutations
    last_root_example_json: str | None = None

    for path, path_info in mutation_paths.items():
        # Skip non-mutable paths
        if path_info.get("path_info", {}).get("mutability") == "not_mutable":
            continue

        # Check if root mutation is needed first
        root_example = path_info.get("path_info", {}).get("root_example")
        # Only set root if: it exists, this is a nested path, and it differs from last root
        # Use JSON serialization for deep equality comparison
        root_example_json = json.dumps(root_example, sort_keys=True) if root_example is not None else None
        if root_example is not None and path != "" and root_example_json != last_root_example_json:
            # Need to set root first
            if mutation_type == "Component":
                op = cast(
                    TestOperation,
                    cast(
                        object,
                        {
                            "operation_id": len(operations),
                            "tool": "mcp__brp__world_mutate_components",
                            "entity": "USE_QUERY_RESULT",
                            "component": type_name,
                            "path": "",
                            "value": root_example,
                            "port": port,
                            "status": None,
                            "error": None,
                            "retry_count": 0,
                            "operation_announced": False,
                        },
                    ),
                )
            else:  # Resource
                op = cast(
                    TestOperation,
                    cast(
                        object,
                        {
                            "operation_id": len(operations),
                            "tool": "mcp__brp__world_mutate_resources",
                            "resource": type_name,
                            "path": "",
                            "value": root_example,
                            "port": port,
                            "status": None,
                            "error": None,
                            "retry_count": 0,
                            "operation_announced": False,
                        },
                    ),
                )

            substitutions = find_entity_id_placeholders(root_example, "")
            if substitutions:
                op["entity_id_substitution"] = substitutions

            operations.append(op)
            last_root_example_json = root_example_json

        # Main mutation
        example = path_info.get("example")
        examples = path_info.get("examples")

        # Handle enum variants (multiple examples)
        test_values = examples if examples else ([example] if example is not None else [])

        for test_value in test_values:  # pyright: ignore[reportAny]
            # For enum variants: only process if it has an "example" key
            if isinstance(test_value, dict):
                if "example" not in test_value:
                    # No example means not testable (either metadata or not_mutable)
                    continue
                # Unwrap the example value
                test_value = test_value["example"]  # pyright: ignore[reportUnknownVariableType]

            if mutation_type == "Component":
                op = cast(
                    TestOperation,
                    cast(
                        object,
                        {
                            "operation_id": len(operations),
                            "tool": "mcp__brp__world_mutate_components",
                            "entity": "USE_QUERY_RESULT",
                            "component": type_name,
                            "path": path,
                            "value": test_value,
                            "port": port,
                            "status": None,
                            "error": None,
                            "retry_count": 0,
                            "operation_announced": False,
                        },
                    ),
                )
            else:  # Resource
                op = cast(
                    TestOperation,
                    cast(
                        object,
                        {
                            "operation_id": len(operations),
                            "tool": "mcp__brp__world_mutate_resources",
                            "resource": type_name,
                            "path": path,
                            "value": test_value,
                            "port": port,
                            "status": None,
                            "error": None,
                            "retry_count": 0,
                            "operation_announced": False,
                        },
                    ),
                )

            substitutions = find_entity_id_placeholders(test_value, "")
            if substitutions:
                op["entity_id_substitution"] = substitutions

            operations.append(op)

            # Track root mutations: if this is a root path mutation, update last_root_example_json
            # so subsequent paths don't redundantly set the same root
            if path == "":
                last_root_example_json = json.dumps(test_value, sort_keys=True)

    return operations


def calculate_type_slots(type_data: TypeData) -> int:
    """
    Calculate how many slots a type will consume (1, 2, 3, 4, ...).

    Shared splitting logic used by both renumbering and preparation.
    Types are split to target ~10 operations per part.

    Args:
        type_data: The type to evaluate

    Returns:
        Number of slots this type will consume
    """
    # Generate operations to count them (using placeholder port)
    all_operations = generate_test_operations(type_data, port=30001)
    operation_count = len(all_operations)

    # Target ~10 operations per part
    # If <= 10 operations, single part
    if operation_count <= 10:
        return 1

    # Count non-mutation operations (spawn + query)
    non_mutation_count = sum(
        1 for op in all_operations
        if op.get("tool") not in [
            "mcp__brp__world_mutate_components",
            "mcp__brp__world_mutate_resources"
        ]
    )

    mutation_count = operation_count - non_mutation_count

    # Part 1 gets: spawn + query + mutations (target 10 total)
    # Remaining parts get: query + mutations (target 10 total, so 9 mutations)
    part1_mutation_budget = max(1, 10 - non_mutation_count)
    remaining_mutations = mutation_count - part1_mutation_budget

    if remaining_mutations <= 0:
        return 1

    # Each additional part gets ~10 operations (1 query + 9 mutations)
    # Ceiling division: (remaining_mutations + 8) // 9
    additional_parts = (remaining_mutations + 8) // 9

    return 1 + additional_parts


def find_split_points(mutations: list[TestOperation], num_parts: int) -> list[int]:
    """
    Find split points that divide mutations into roughly equal parts.
    Simple division with no root mutation backtracking - prepending handles correctness.

    Args:
        mutations: List of mutation operations to split
        num_parts: Number of parts to split into

    Returns:
        List of split indices (length = num_parts - 1)
        For 4 parts, returns 3 indices indicating where to split
    """
    if num_parts == 1:
        return []

    mutation_count = len(mutations)
    mutations_per_part = mutation_count / num_parts

    split_indices: list[int] = []
    for part_num in range(1, num_parts):
        split_idx = int(part_num * mutations_per_part)
        split_indices.append(split_idx)

    return split_indices


def split_operations_for_part(
    all_operations: list[TestOperation],
    part_number: int,
    total_parts: int
) -> list[TestOperation]:
    """
    Split operations for multi-part type testing with variable part counts.

    Part 1: spawn/insert + query + first chunk of mutations
    Part 2+: query + root_mutation (if needed) + chunk of mutations (no spawn)

    Root mutations (path="") are prepended to parts that start with deep paths
    to establish correct variant structure.
    """
    if total_parts == 1:
        return all_operations

    # Find operation indices
    spawn_idx: int | None = None
    query_idx: int | None = None
    mutation_start_idx: int | None = None

    for idx, op in enumerate(all_operations):
        tool = op.get("tool", "")
        if tool in ["mcp__brp__world_spawn_entity", "mcp__brp__world_insert_resources"]:
            spawn_idx = idx
        elif tool == "mcp__brp__world_query":
            query_idx = idx
        elif tool in ["mcp__brp__world_mutate_components", "mcp__brp__world_mutate_resources"]:
            if mutation_start_idx is None:
                mutation_start_idx = idx

    # Get all mutation operations
    mutations: list[TestOperation] = []
    if mutation_start_idx is not None:
        mutations = all_operations[mutation_start_idx:]

    # Find all split points using simple fixed-size division
    split_indices = find_split_points(mutations, total_parts)

    if part_number == 1:
        # Part 1: spawn + query + first chunk of mutations
        result: list[TestOperation] = []

        # Add spawn if it exists
        if spawn_idx is not None:
            result.append(all_operations[spawn_idx])

        # Add query for components
        if query_idx is not None:
            result.append(all_operations[query_idx])

        # Add mutations up to first split
        if split_indices:
            result.extend(mutations[:split_indices[0]])
        else:
            result.extend(mutations)

        return result
    else:
        # Part 2+: query + possibly prepended root + chunk of mutations
        result = []

        # Add query for components
        if query_idx is not None:
            result.append(all_operations[query_idx])

        # Determine mutation range for this part
        start_idx = split_indices[part_number - 2]  # Previous split point
        end_idx = split_indices[part_number - 1] if part_number < total_parts else len(mutations)

        # Check if we need to prepend a root mutation
        if start_idx < len(mutations):
            first_op = mutations[start_idx]
            first_path = first_op.get("path", "")

            # If first operation is NOT a root mutation, find most recent root
            if first_path != "":
                for i in range(start_idx - 1, -1, -1):
                    if mutations[i].get("path") == "":
                        result.append(mutations[i])  # Duplicate the root mutation
                        break

        # Add this part's mutation chunk
        result.extend(mutations[start_idx:end_idx])

        return result


# Load and parse JSON file
try:
    with open(json_file, "r") as f:
        data = cast(TypeGuideRoot, json.load(f))
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Expect type_guide at root
if "type_guide" not in data:
    print(f"Error: Expected dict with 'type_guide' at root", file=sys.stderr)
    sys.exit(1)

# STEP 1: Cleanup previous runs if this is batch 1
if batch_num == 1:
    # Remove leftover files from previous runs
    tmpdir = tempfile.gettempdir()
    cleanup_patterns = [
        ".claude/transient/batch_results_*.json",
        ".claude/transient/all_types_failures_*.json",
        os.path.join(tmpdir, "mutation_test_subagent_*_plan.json")
    ]
    for pattern in cleanup_patterns:
        for filepath in glob.glob(pattern):
            try:
                os.remove(filepath)
            except OSError:
                pass  # Ignore errors if files don't exist or can't be removed

# STEP 2: Renumber batches before every batch (resets failed→untested, reassigns batch numbers)
data = renumber_batches(data, batch_size)

# Write updated data back to file
try:
    with open(json_file, "w") as f:
        json.dump(data, f, indent=2)
except IOError as e:
    print(f"Error writing updated JSON: {e}", file=sys.stderr)
    sys.exit(1)

type_guide: dict[str, TypeData] = data["type_guide"]

# STEP 2: Get types for the specified batch
batch_types: list[TypeData] = []
for type_name, type_info in type_guide.items():
    if type_info.get("batch_number") == batch_num:
        # Add type_name to the dict for consistency
        type_item: TypeData = cast(
            TypeData, cast(object, {"type_name": type_name, **type_info})
        )
        batch_types.append(type_item)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)

# STEP 3: Identify large types for splitting using shared logic
# Pre-generate operations to count them
class TypePart(TypedDict):
    type_data: TypeData
    part_number: int  # 1-indexed
    total_parts: int
    all_operations: list[TestOperation]  # All operations for this type

type_parts: list[TypePart] = []

for type_item in batch_types:
    # Extract mutation_type from schema_info
    schema_info = type_item.get("schema_info")
    mutation_type = extract_mutation_type(schema_info)

    # Build complete type_data with mutation_type
    type_data: TypeData = cast(
        TypeData,
        cast(
            object,
            {
                "type_name": type_item["type_name"],
                "spawn_format": type_item.get("spawn_format"),
                "mutation_paths": type_item.get("mutation_paths"),
                "supported_operations": type_item.get("supported_operations"),
                "in_registry": type_item.get("in_registry"),
                "schema_info": schema_info,
                "mutation_type": mutation_type,
            },
        ),
    )

    # Generate all operations for this type
    # Use a placeholder port - will be assigned later
    all_operations = generate_test_operations(type_data, port=30001)

    # Use shared splitting logic to determine how many parts
    slots_needed = calculate_type_slots(type_data)

    # Create parts for this type (1 to N parts based on operation count)
    for part_num in range(1, slots_needed + 1):
        type_parts.append(TypePart(
            type_data=type_data,
            part_number=part_num,
            total_parts=slots_needed,
            all_operations=all_operations
        ))

# STEP 4: Calculate flexible distribution based on type parts
total_available_parts: int = len(type_parts)

# Calculate optimal distribution: fill subagents with preferred count, handle remainder
base_types_per_subagent: int = min(types_per_subagent, total_available_parts)
full_subagents: int = total_available_parts // types_per_subagent
remainder_types: int = total_available_parts % types_per_subagent

# Determine actual number of subagents needed
if remainder_types > 0:
    actual_subagents_needed: int = min(full_subagents + 1, max_subagents)
else:
    actual_subagents_needed = min(full_subagents, max_subagents)

# STEP 5: Distribute type parts across subagents and generate test plans
assignments: list[SubagentAssignment] = []
part_index = 0

for subagent_num in range(1, actual_subagents_needed + 1):
    # Determine how many parts this subagent gets
    if subagent_num <= full_subagents:
        # This subagent gets the full amount
        parts_for_this_subagent = types_per_subagent
    else:
        # This is the last subagent, gets the remainder
        parts_for_this_subagent = remainder_types

    subagent_parts: list[TypePart] = []
    for _ in range(parts_for_this_subagent):
        if part_index < len(type_parts):
            subagent_parts.append(type_parts[part_index])
            part_index += 1

    if subagent_parts:  # Only create assignment if there are parts
        port = 30000 + subagent_num

        # Generate test plan and formatted descriptions
        tests: list[TypeTest] = []
        type_descriptions: list[str] = []

        # Track operation IDs sequentially across all types for this subagent
        operation_id_counter = 0

        for type_part in subagent_parts:
            type_data = type_part["type_data"]
            type_name = type_data["type_name"]
            mutation_type = type_data.get("mutation_type")
            part_number = type_part["part_number"]
            total_parts = type_part["total_parts"]
            all_operations = type_part["all_operations"]

            # Split operations if needed
            operations = split_operations_for_part(all_operations, part_number, total_parts)

            # Renumber operation IDs sequentially across all types and update port
            for op in operations:
                op["operation_id"] = operation_id_counter
                op["port"] = port
                operation_id_counter += 1

            # Add to test plan
            test: TypeTest = {
                "type_name": type_name,
                "mutation_type": mutation_type or "Unknown",
                "part_number": part_number,
                "total_parts": total_parts,
                "operations": operations
            }
            tests.append(test)

            # Extract short name (text after last ::)
            short_name = type_name.split("::")[-1]

            # Get category
            category = "C" if mutation_type == "Component" else "R" if mutation_type == "Resource" else "?"

            # Count operations
            op_count = len(operations)

            # Format description with part info if split
            if total_parts > 1:
                type_descriptions.append(f"{short_name} ({category}: {op_count} ops, part {part_number}/{total_parts})")
            else:
                type_descriptions.append(f"{short_name} ({category}: {op_count} ops)")

        # Create test plan file using shared utility
        result = subprocess.run(
            ["python3", ".claude/scripts/mutation_test/get_plan_file_path.py", "--port", str(port)],
            capture_output=True,
            text=True,
            check=True
        )
        test_plan_file = result.stdout.strip()

        test_plan: TestPlan = {
            "batch_number": batch_num,
            "subagent_index": subagent_num - 1,  # 0-based index
            "port": port,
            "test_plan_file": test_plan_file,
            "tests": tests
        }

        # Write test plan to temp file
        try:
            with open(test_plan_file, "w") as f:
                json.dump(test_plan, f, indent=2)
        except IOError as e:
            print(f"Error writing test plan file: {e}", file=sys.stderr)
            sys.exit(1)

        # Create assignment with pre-formatted descriptions
        # Join all type descriptions with commas
        types_str = ", ".join(type_descriptions)

        assignment: SubagentAssignment = {
            "subagent": subagent_num,
            "port": port,
            "window_description": f"Subagent {subagent_num}: {types_str}",
            "task_description": f"Test {types_str} ({subagent_num} of {actual_subagents_needed})",
            "test_plan_file": test_plan_file
        }
        assignments.append(assignment)

# STEP 5: Open test plan files in Zed
zed_cli = "/Applications/Zed.app/Contents/MacOS/cli"
if os.path.exists(zed_cli):
    for assignment in assignments:
        test_plan_file = assignment["test_plan_file"]
        try:
            _ = subprocess.Popen([zed_cli, test_plan_file], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except OSError:
            pass  # Ignore if Zed fails to open

# STEP 6: Return all assignments with test plan files generated
# Calculate unique types that actually made it into assignments
assigned_type_names: set[str] = set()
for assignment in assignments:
    # Open the test plan file to get the actual types assigned
    try:
        with open(assignment["test_plan_file"], "r") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            test_plan = cast(TestPlan, test_plan_raw)
            for test in test_plan.get("tests", []):
                type_name = test.get("type_name")
                if type_name:
                    assigned_type_names.add(type_name)
    except (IOError, json.JSONDecodeError):
        pass

unique_types_count = len(assigned_type_names)
subagent_count = len(assignments)

# Calculate statistics for progress message
total_batches = max(
    (t.get("batch_number") or 0 for t in type_guide.values()),
    default=0
)
untested_count = len([t for t in type_guide.values() if t.get("test_status") == "untested"])
remaining_types = untested_count - unique_types_count  # Remaining after this batch

# Generate progress message
if unique_types_count == subagent_count:
    distribution = f"{unique_types_count} types across {subagent_count} subagents"
else:
    distribution = f"{unique_types_count} types split across {subagent_count} subagents"

progress_message = f"Processing batch {batch_num} of {total_batches} - Testing {distribution} ({remaining_types} remaining)"

all_assignments_output: AllAssignmentsOutput = {
    "batch_number": batch_num,
    "max_subagents": subagent_count,  # Report actual subagents used
    "types_per_subagent": types_per_subagent,  # Keep original for reference
    "total_types": unique_types_count,
    "progress_message": progress_message,
    "assignments": assignments,
}

# Print summary to stderr for user visibility
print(f"✓ {distribution}", file=sys.stderr)

print(json.dumps(all_assignments_output, indent=2))
