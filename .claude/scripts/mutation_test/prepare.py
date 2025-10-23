#!/usr/bin/env python3
"""
Mutation test preparation: batch renumbering and assignment generation.

This script combines two operations:
1. Batch renumbering: Reset failed tests and assign batch numbers
2. Assignment generation: Create test plans and distribute types

Configuration is loaded from .claude/config/mutation_test_config.json.
The batch number is auto-discovered by finding the first untested batch.

Usage:
  python3 mutation_test_prepare.py

Output:
  Returns AllAssignmentsOutput with assignments and test plan files.
"""

import json
import os
import subprocess
import sys
from copy import deepcopy
from datetime import datetime
from typing import Any, TypedDict, cast

# Import shared config module using relative import
from .config import (
    AllTypesData,
    MutationTestConfig,
    TypeData,
    TypeDataComplete,
    find_current_batch,
    load_config,
)

# Type alias for backward compatibility
TypeGuideRoot = AllTypesData
MutationConfig = MutationTestConfig

# Constants
OPERATION_ID_START = 1  # Operation IDs start at 1 for better human readability


# Type definitions for JSON structures (extends config.py's TypeData)
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""
    mutability: str
    root_example: object


class MutationPathData(TypedDict, total=False):
    description: str
    example: object
    examples: list[object]
    path_info: PathInfo


class SubagentAssignment(TypedDict):
    subagent: int
    port: int
    window_description: str  # Pre-formatted window title
    task_description: str  # Pre-formatted task description
    test_plan_file: str  # Path to generated test plan file
    type_descriptions: list[str]  # List of type descriptions for debug log


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
    call_count: int
    operation_announced: bool
    # Spawn/query specific
    components: dict[str, object] | None
    filter: dict[str, list[str]] | None
    data: dict[str, object] | None
    # Mutation specific
    entity: str | int | None  # "USE_QUERY_RESULT" or actual entity ID
    component: str | None
    resource: str | None
    path: str | None
    value: object
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


# Load configuration from config file
try:
    config = load_config()
except FileNotFoundError as e:
    print(f"Error loading config: {e}", file=sys.stderr)
    sys.exit(1)

max_subagents: int = config["max_subagents"]
types_per_subagent: int = config["types_per_subagent"]
base_port: int = config["base_port"]

# Calculate batch size
batch_size: int = max_subagents * types_per_subagent

# Get the JSON file path
json_file = ".claude/transient/all_types.json"

if not os.path.exists(json_file):
    print(f"Error: {json_file} not found!", file=sys.stderr)
    sys.exit(1)

# Load all_types.json to discover current batch
try:
    with open(json_file, "r", encoding="utf-8") as f:
        all_types_raw = json.load(f)  # pyright: ignore[reportAny]
        all_types_data: dict[str, object] = all_types_raw  # pyright: ignore[reportAny]
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Batch number will be discovered after renumbering
batch_num: int = -1  # Placeholder, will be set after renumbering


def renumber_batches(data: AllTypesData, batch_size: int) -> AllTypesData:
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

    # Get config values for proper capacity checking
    # Note: We reconstruct these from batch_size since config isn't passed here
    # batch_size = max_subagents * types_per_subagent
    # We'll use common default: types_per_subagent = 2
    types_per_subagent_val = 2  # Common config value
    max_subagents_val = batch_size // types_per_subagent_val

    for type_name, type_data_raw in untested_types:
        # Extract mutation_type to match preparation phase
        schema_info = type_data_raw.get("schema_info")
        mutation_type = extract_mutation_type(schema_info)

        # Build complete type_data with mutation_type (same as preparation phase)
        type_data: TypeDataComplete = cast(
            TypeDataComplete,
            cast(
                object,
                {
                    "type_name": type_name,
                    "spawn_format": type_data_raw.get("spawn_format"),
                    "mutation_paths": type_data_raw.get("mutation_paths"),
                    "supported_operations": type_data_raw.get("supported_operations"),
                    "in_registry": type_data_raw.get("in_registry"),
                    "schema_info": schema_info,
                    "mutation_type": mutation_type,
                },
            ),
        )

        # Calculate slots needed for this type
        slots_needed = calculate_type_slots(type_data)

        # Calculate how many subagents this type needs
        subagents_needed = (
            slots_needed + types_per_subagent_val - 1
        ) // types_per_subagent_val

        # Calculate how many subagents are currently used in this batch (running total)
        current_subagents_used = (
            current_batch_slots + types_per_subagent_val - 1
        ) // types_per_subagent_val

        # Check if adding this type would exceed subagent capacity
        # This ensures we don't orphan parts due to subagent packing
        if current_subagents_used + subagents_needed > max_subagents_val:
            # Start new batch
            current_batch += 1
            current_batch_slots = 0

            # Verify the type can fit in an empty batch
            if subagents_needed > max_subagents_val:
                error_msg = (
                    f"Type '{type_name}' requires {slots_needed} slots ({subagents_needed} subagents) "
                    + f"but batch can only provide {max_subagents_val} subagents. "
                    + "Increase max_subagents in config."
                )
                raise ValueError(error_msg)

        # Assign type to current batch
        type_guide[type_name]["batch_number"] = current_batch
        current_batch_slots += slots_needed

    # Report statistics
    total = len(type_guide)
    untested = len(
        [t for t in type_guide.values() if t.get("test_status") == "untested"]
    )
    failed = len([t for t in type_guide.values() if t.get("test_status") == "failed"])
    passed = len([t for t in type_guide.values() if t.get("test_status") == "passed"])
    max_batch = max(
        (t.get("batch_number") or 0 for t in type_guide.values()), default=0
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


def generate_test_operations(type_data: TypeDataComplete, port: int) -> list[TestOperation]:
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
                        "call_count": 0,
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
                        "call_count": 0,
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
        # Note: path_info dict contains a "path_info" key that holds PathInfo
        path_metadata = cast(dict[str, object], path_info).get("path_info")
        if path_metadata:
            path_metadata_dict = cast(dict[str, object], path_metadata)
            if path_metadata_dict.get("mutability") == "not_mutable":
                continue
            root_example = path_metadata_dict.get("root_example")
        else:
            root_example = None
        # Only set root if: it exists, this is a nested path, and it differs from last root
        # Use JSON serialization for deep equality comparison
        root_example_json = (
            json.dumps(root_example, sort_keys=True)
            if root_example is not None
            else None
        )
        if (
            root_example is not None
            and path != ""
            and root_example_json != last_root_example_json
        ):
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
        path_info_dict = cast(dict[str, object], path_info)
        example = path_info_dict.get("example")
        examples = path_info_dict.get("examples")

        # Handle enum variants (multiple examples)
        test_values: list[object] = cast(
            list[object],
            examples if examples else ([example] if example is not None else [])
        )

        for test_value in test_values:
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


def calculate_type_slots(type_data: TypeDataComplete) -> int:
    """
    Calculate how many slots a type will consume (1, 2, 3, 4, ...).

    Based on original operation count, targeting ~10 operations per part.
    Prepended root mutations may add at most 1 extra operation per part,
    which is acceptable (parts can go slightly over 10).

    Args:
        type_data: The type to evaluate

    Returns:
        Number of slots this type will consume
    """
    # Generate operations to count them (using placeholder port)
    all_operations = generate_test_operations(type_data, port=30001)
    operation_count = len(all_operations)

    # Target ~10 operations per part
    # Ceiling division: (operation_count + 9) // 10
    slots_needed = (operation_count + 9) // 10

    return slots_needed


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
    all_operations: list[TestOperation], part_number: int, total_parts: int
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
        elif tool in [
            "mcp__brp__world_mutate_components",
            "mcp__brp__world_mutate_resources",
        ]:
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
            result.extend(mutations[: split_indices[0]])
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
        end_idx = (
            split_indices[part_number - 1]
            if part_number < total_parts
            else len(mutations)
        )

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
        data = cast(AllTypesData, json.load(f))
except json.JSONDecodeError as e:
    print(f"Error parsing JSON: {e}", file=sys.stderr)
    sys.exit(1)

# Expect type_guide at root
if "type_guide" not in data:
    print("Error: Expected dict with 'type_guide' at root", file=sys.stderr)
    sys.exit(1)

# STEP 1: Renumber batches before every batch (resets failed→untested, reassigns batch numbers)
data = renumber_batches(data, batch_size)

# Write updated data back to file
try:
    with open(json_file, "w") as f:
        json.dump(data, f, indent=2)
except IOError as e:
    print(f"Error writing updated JSON: {e}", file=sys.stderr)
    sys.exit(1)

# NOW discover current batch number using the renumbered data
batch_result: int | str = find_current_batch(data)
if batch_result == "COMPLETE":
    print("All tests complete! No untested batches remaining.", file=sys.stderr)
    sys.exit(0)

# At this point batch_result must be int (we exited if it was "COMPLETE")
assert isinstance(batch_result, int)
batch_num = batch_result

type_guide: dict[str, TypeDataComplete] = data["type_guide"]

# STEP 2: Get types for the specified batch
batch_types: list[TypeDataComplete] = []
for type_name, type_info in type_guide.items():
    if type_info.get("batch_number") == batch_num:
        # Add type_name to the dict for consistency
        type_item: TypeDataComplete = cast(
            TypeDataComplete, cast(object, {"type_name": type_name, **type_info})
        )
        batch_types.append(type_item)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)


# STEP 3: Identify large types for splitting using shared logic
# Pre-generate operations to count them
class TypePart(TypedDict):
    type_data: TypeDataComplete
    part_number: int  # 1-indexed
    total_parts: int
    all_operations: list[TestOperation]  # All operations for this type


type_parts: list[TypePart] = []

for type_item in batch_types:
    # Extract mutation_type from schema_info
    schema_info = type_item.get("schema_info")
    mutation_type = extract_mutation_type(schema_info)

    # Build complete type_data with mutation_type
    type_data: TypeDataComplete = cast(
        TypeDataComplete,
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
        type_parts.append(
            TypePart(
                type_data=type_data,
                part_number=part_num,
                total_parts=slots_needed,
                all_operations=all_operations,
            )
        )

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

        # Track operation IDs sequentially across all tests in this file
        operation_id_counter = OPERATION_ID_START

        for type_part in subagent_parts:
            type_data = type_part["type_data"]
            type_name = type_data["type_name"]
            mutation_type = type_data.get("mutation_type")
            part_number = type_part["part_number"]
            total_parts = type_part["total_parts"]
            all_operations = type_part["all_operations"]

            # Split operations if needed
            operations = split_operations_for_part(
                all_operations, part_number, total_parts
            )

            # Deep copy operations to avoid modifying shared references across subagents
            operations = deepcopy(operations)

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
                "operations": operations,
            }
            tests.append(test)

            # Extract short name (text after last ::)
            short_name = type_name.split("::")[-1]

            # Get category
            category = (
                "C"
                if mutation_type == "Component"
                else "R"
                if mutation_type == "Resource"
                else "?"
            )

            # Count operations
            op_count = len(operations)

            # Format description with part info if split
            if total_parts > 1:
                type_descriptions.append(
                    f"{short_name} ({category}: {op_count} ops, {part_number} of {total_parts})"
                )
            else:
                type_descriptions.append(f"{short_name} ({category}: {op_count} ops)")

        # Create test plan file using shared utility
        result = subprocess.run(
            [
                "python3",
                ".claude/scripts/mutation_test/get_plan_file_path.py",
                "--port",
                str(port),
            ],
            capture_output=True,
            text=True,
            check=True,
        )
        test_plan_file = result.stdout.strip()

        test_plan: TestPlan = {
            "batch_number": batch_num,
            "subagent_index": subagent_num - 1,  # 0-based index
            "port": port,
            "test_plan_file": test_plan_file,
            "tests": tests,
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

        assignment: SubagentAssignment = cast(
            SubagentAssignment,
            cast(
                object,
                {
                    "subagent": subagent_num,
                    "port": port,
                    "window_description": f"Subagent {subagent_num}: {types_str}",
                    "task_description": f"Test {types_str} ({subagent_num} of {actual_subagents_needed})",
                    "test_plan_file": test_plan_file,
                    "type_descriptions": type_descriptions,
                },
            ),
        )
        assignments.append(assignment)

# STEP 5: Backup and initialize debug log
DEBUG_LOG = "/tmp/mutation_hook_debug.log"

# Backup existing log if it exists
if os.path.exists(DEBUG_LOG):
    # Extract batch number and timestamp from existing log metadata
    batch_num_str = "unknown"
    log_timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    try:
        with open(DEBUG_LOG, encoding="utf-8") as f:
            for line in f:
                if line.startswith("# Batch Number:"):
                    parts = line.split()
                    if len(parts) >= 4:
                        batch_num_str = parts[3]
                elif line.startswith("# Started:"):
                    # Extract timestamp from "# Started: 2025-10-22 22:16:12"
                    parts = line.split()
                    if len(parts) >= 4:
                        date_part = parts[2].replace("-", "")
                        time_part = parts[3].replace(":", "")
                        log_timestamp = f"{date_part}_{time_part}"
                    break  # Found both metadata lines
    except Exception:
        pass  # Use defaults if parsing fails

    backup_file = f"/tmp/mutation_hook_debug_batch{batch_num_str}_{log_timestamp}.log"
    try:
        os.rename(DEBUG_LOG, backup_file)
        print(f"Backed up previous log to: {backup_file}", file=sys.stderr)
    except OSError:
        pass  # Ignore if backup fails

# Create new debug log with metadata for current batch
ports_str = ", ".join(str(a["port"]) for a in assignments)
timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

with open(DEBUG_LOG, "w", encoding="utf-8") as f:
    _ = f.write("# Mutation Test Debug Log\n")
    _ = f.write(f"# Batch Number: {batch_num}\n")
    _ = f.write(f"# Started: {timestamp}\n")
    _ = f.write(f"# Ports: {ports_str}\n")
    _ = f.write("# Subagent - Types\n")

    # Calculate total ops for each assignment and find max width for alignment
    assignment_ops: list[int] = []
    for assignment in assignments:
        total_ops = 0
        try:
            with open(assignment["test_plan_file"], "r") as plan_f:
                plan_data = json.load(plan_f)  # pyright: ignore[reportAny]
                plan = cast(TestPlan, plan_data)
                for test in plan.get("tests", []):
                    total_ops += len(test.get("operations", []))
        except (IOError, json.JSONDecodeError):
            pass
        assignment_ops.append(total_ops)

    # Find max width for right alignment
    max_ops_width = (
        max(len(str(ops)) for ops in assignment_ops) if assignment_ops else 1
    )
    max_subagent_width = len(str(len(assignments)))

    # Calculate indentation for multi-line type lists
    # Format: "#   {subagent_num} ({total_ops} ops): "
    # Total prefix length includes: "#   " (4) + num + " (" (2) + ops + " ops): " (7)
    prefix_length = 4 + max_subagent_width + 2 + max_ops_width + 7
    # For continuation lines, we need prefix_length - 1 spaces after the "#"
    continuation_indent = prefix_length - 1

    # Find longest type name across ALL assignments for global columnar alignment
    # Also find max operation count for right-aligning op counts
    max_type_name_length = 0
    max_op_count_per_type = 0
    for assignment in assignments:
        for type_desc in assignment["type_descriptions"]:
            # Split on first opening paren to get type name
            type_name_part = (
                type_desc.split(" (")[0] if " (" in type_desc else type_desc
            )
            max_type_name_length = max(max_type_name_length, len(type_name_part))

            # Extract operation count (e.g., "C: 7 ops" or "R: 11 ops")
            # Format is "TypeName (X: NN ops[, part info])"
            if " (" in type_desc and " ops" in type_desc:
                # Extract the number between ": " and " ops"
                parts = type_desc.split(": ")
                if len(parts) >= 2:
                    ops_part = parts[1].split(" ops")[0]
                    try:
                        op_count = int(ops_part)
                        max_op_count_per_type = max(max_op_count_per_type, op_count)
                    except ValueError:
                        pass

    # Calculate width needed for operation count alignment
    op_count_width = len(str(max_op_count_per_type)) if max_op_count_per_type > 0 else 1

    def format_ops_count(rest: str, width: int) -> str:
        """Format operation count with right alignment.

        Input: "C: 7 ops, 1 of 2)" or "R: 11 ops)"
        Output: "C:  7 ops, 1 of 2)" or "R: 11 ops)" (right-aligned count)
        """
        if ": " in rest and " ops" in rest:
            # Split on ": " to get prefix (C or R) and the rest
            prefix, after_colon = rest.split(": ", 1)
            # Split on " ops" to get the count and the suffix
            count_str, after_ops = after_colon.split(" ops", 1)
            # Right-align the count
            aligned_count = count_str.rjust(width)
            return f"{prefix}: {aligned_count} ops{after_ops}"
        return rest

    # Write formatted subagent lines
    for idx, assignment in enumerate(assignments):
        total_ops = assignment_ops[idx]
        subagent_num = assignment["subagent"]
        type_list = assignment["type_descriptions"]

        if type_list:
            # First type on same line as subagent info
            first_desc = type_list[0]
            if " (" in first_desc:
                type_name, rest = first_desc.split(" (", 1)
                # Right-align operation count in the rest part
                formatted_rest = format_ops_count(rest, op_count_width)
                padded_first = f"{type_name:<{max_type_name_length}} ({formatted_rest}"
            else:
                padded_first = first_desc
            _ = f.write(
                f"#   {subagent_num:>{max_subagent_width}} ({total_ops:>{max_ops_width}} ops): {padded_first}\n"
            )

            # Subsequent types indented on their own lines with columnar alignment
            for type_desc in type_list[1:]:
                if " (" in type_desc:
                    type_name, rest = type_desc.split(" (", 1)
                    # Right-align operation count in the rest part
                    formatted_rest = format_ops_count(rest, op_count_width)
                    padded_desc = (
                        f"{type_name:<{max_type_name_length}} ({formatted_rest}"
                    )
                else:
                    padded_desc = type_desc
                _ = f.write(f"#{' ' * continuation_indent}{padded_desc}\n")

    _ = f.write("# Test Plans\n")
    for assignment in assignments:
        _ = f.write(f"#   {assignment['test_plan_file']}\n")
    _ = f.write("# ----------------------------------------\n\n")

# Write initial announcements for first operation on each port using operation_update.py
for assignment in assignments:
    port = assignment["port"]
    try:
        _ = subprocess.run(
            [
                "python3",
                ".claude/scripts/mutation_test/operation_update.py",
                "--port",
                str(port),
                "--operation-id",
                str(OPERATION_ID_START),
                "--announced",
            ],
            check=True,
            capture_output=True,
        )
    except subprocess.CalledProcessError:
        # Ignore announcement failures - not critical
        pass

print(f"Created new debug log: {DEBUG_LOG}", file=sys.stderr)
print(f"  Batch: {batch_num}", file=sys.stderr)
print(f"  Ports: {ports_str}", file=sys.stderr)
print(f"  Types count: {len(assignments)}", file=sys.stderr)

# STEP 6: Open debug log in Zed
zed_cli = "/Applications/Zed.app/Contents/MacOS/cli"
if os.path.exists(zed_cli):
    try:
        _ = subprocess.Popen(
            [zed_cli, DEBUG_LOG], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
    except OSError:
        pass  # Ignore if Zed fails to open

# STEP 7: Return all assignments with test plan files generated
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
    (t.get("batch_number") or 0 for t in type_guide.values()), default=0
)
untested_count = len(
    [t for t in type_guide.values() if t.get("test_status") == "untested"]
)
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
