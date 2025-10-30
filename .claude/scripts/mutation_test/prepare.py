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
from pathlib import Path
from typing import Any, TypedDict, cast

# Add the script directory to Python path for imports
script_dir = Path(__file__).parent
sys.path.insert(0, str(script_dir))

from config import (  # noqa: E402
    AllTypesData,
    MutationTestConfig,
    TypeData,
    TypeDataComplete,
    calculate_port,
    find_current_batch,
    get_mutation_test_log,
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
    root_example_unavailable_reason: str


class MutationPathData(TypedDict, total=False):
    path: str
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


class ExcludedTypeEntry(TypedDict):
    """Entry in the excluded types configuration file."""

    type_name: str
    reason: str


class ExcludedTypesConfig(TypedDict):
    """Configuration file for excluded types."""

    excluded_types: list[ExcludedTypeEntry]


class AllAssignmentsOutput(TypedDict):
    batch_number: int
    max_subagents: int
    ops_per_subagent: int
    total_types: int
    progress_message: str  # Pre-formatted progress message for display
    assignments: list[SubagentAssignment]


# Test plan types
class TestOperation(TypedDict, total=False):
    operation_id: int  # Sequential ID for tracking operations
    tool: str  # MCP tool name
    # Common fields
    port: int
    # Runtime tracking fields (added by operation_update.py)
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
    # Entity ID substitution - placeholder value to replace
    entity_id_substitution: int | None


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


class OperationIndices(TypedDict):
    """Indices of key operations within an operation list."""

    spawn_idx: int | None
    query_idx: int | None
    mutation_start_idx: int | None


# Load configuration from config file
try:
    mutation_config = load_config()
except FileNotFoundError as e:
    print(f"Error loading config: {e}", file=sys.stderr)
    sys.exit(1)

max_subagents: int = mutation_config["max_subagents"]
ops_per_subagent: int = mutation_config["ops_per_subagent"]
base_port: int = mutation_config["base_port"]

# Calculate batch capacity (total operations across all subagents)
batch_capacity: int = max_subagents * ops_per_subagent

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


def renumber_batches(
    data: AllTypesData,
    batch_capacity: int,
    max_subagents: int,
    ops_per_subagent: int,
    excluded_type_names: set[str],
) -> AllTypesData:
    """
    Pack ALL untested types into batches in a single pass.

    Algorithm:
    1. Reset failed tests to untested
    2. Find next batch number to use
    3. Pack everything that fits into current batch
    4. Increment batch number
    5. Loop through ALL remaining types (including skipped ones)
    6. Continue until all types are packed

    Args:
        data: AllTypesData containing type_guide
        batch_capacity: Total operation capacity for a batch (max_subagents * ops_per_subagent)
        max_subagents: Maximum number of subagents per batch
        ops_per_subagent: Operation capacity per subagent
        excluded_type_names: Set of type names to exclude from testing
    """
    type_guide = data["type_guide"]

    # Step 1: Reset failed tests to untested and clear their batch numbers
    for type_name, type_data in type_guide.items():
        if type_name in excluded_type_names:
            continue
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

    # Step 3: Clear batch numbers for ALL untested types
    for type_data in type_guide.values():
        if type_data.get("test_status") == "untested":
            type_data["batch_number"] = None

    # Step 4: Pack ALL untested types into batches (single pass)
    untested_types: list[tuple[str, TypeData]] = [
        (type_name, type_data)
        for type_name, type_data in type_guide.items()
        if type_data.get("test_status") == "untested"
        and type_name not in excluded_type_names
    ]

    current_batch = max_batch + 1
    current_subagent_idx = 0  # 0-indexed within batch
    current_ops_in_subagent = 0  # Operations used in current subagent

    # Pack all types into batches - loop until nothing left to pack
    while True:
        packed_any_in_batch = False
        type_idx = 0

        while type_idx < len(untested_types):
            type_name, type_data_raw = untested_types[type_idx]

            # Skip if already assigned to a batch
            if type_guide[type_name].get("batch_number") is not None:
                type_idx += 1
                continue

            # Extract mutation_type to match preparation phase
            schema_info = type_data_raw.get("schema_info")
            mutation_type = extract_mutation_type(schema_info)

            # Build complete type_data with mutation_type (same as preparation phase)
            type_data = build_type_data_complete(
                type_name, type_data_raw, mutation_type
            )

            # Calculate operations needed for this type
            ops_needed = calculate_type_operations(type_data)

            # Check if type can fit in empty batch (sanity check - warn if not in current batch)
            if ops_needed > batch_capacity:
                print(
                    f"Warning: Type '{type_name}' requires {ops_needed} operations "
                    + f"but batch capacity is only {batch_capacity} operations. "
                    + "This type will be skipped. Increase max_subagents or ops_per_subagent in config."
                )
                type_guide[type_name]["batch_number"] = -1  # Mark as skipped
                type_idx += 1
                continue

            # Can this type fit in remaining batch capacity?
            # Calculate actual operations including query overhead for splits
            can_fit = False

            # Calculate how many subagents remain (including current partial one)
            remaining_subagents = max_subagents - current_subagent_idx

            # Simulate packing to see if it fits
            test_ops_remaining = ops_needed
            test_subagent_idx = 0
            test_ops_in_subagent = (
                current_ops_in_subagent  # How many ops USED in current subagent
            )
            parts_needed = 0

            while test_ops_remaining > 0 and test_subagent_idx < remaining_subagents:
                # How much can we fit in this subagent?
                available_in_subagent = ops_per_subagent - test_ops_in_subagent
                if available_in_subagent <= 0:
                    # Move to next subagent
                    test_subagent_idx += 1
                    test_ops_in_subagent = 0
                    continue

                # For components, part 2+ needs a query operation (part 1 already has it)
                extra_ops = 0
                if parts_needed > 0 and mutation_type == "Component":
                    extra_ops = 1  # Query operation for this part

                ops_in_this_part = min(
                    test_ops_remaining, available_in_subagent - extra_ops
                )
                if ops_in_this_part <= 0:
                    # Can't fit even the query, need next subagent
                    test_subagent_idx += 1
                    test_ops_in_subagent = 0
                    continue

                test_ops_remaining -= ops_in_this_part
                test_ops_in_subagent += ops_in_this_part + extra_ops
                parts_needed += 1

                if test_ops_in_subagent >= ops_per_subagent:
                    test_subagent_idx += 1
                    test_ops_in_subagent = 0

            # Did we fit all operations?
            if test_ops_remaining == 0:
                can_fit = True

            if can_fit:
                # Yes, assign to current batch
                type_guide[type_name]["batch_number"] = current_batch
                packed_any_in_batch = True

                # Advance position accounting for query overhead in splits
                # This must match exactly how the simulation worked
                ops_to_place = ops_needed
                part_num = 0
                while ops_to_place > 0:
                    available = ops_per_subagent - current_ops_in_subagent

                    # For components, part 2+ needs a query operation
                    extra_ops = 0
                    if part_num > 0 and mutation_type == "Component":
                        extra_ops = 1

                    ops_in_this_part = min(ops_to_place, available - extra_ops)
                    current_ops_in_subagent += ops_in_this_part + extra_ops
                    ops_to_place -= ops_in_this_part
                    part_num += 1

                    # If current subagent is full, move to next
                    if current_ops_in_subagent >= ops_per_subagent:
                        current_subagent_idx += 1
                        current_ops_in_subagent = 0

            # Move to next type (whether it fit or not)
            type_idx += 1

        # Done iterating through all types for this batch
        # If we didn't pack anything, we're done entirely
        if not packed_any_in_batch:
            break

        # Start next batch
        current_batch += 1
        current_subagent_idx = 0
        current_ops_in_subagent = 0

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


def build_type_data_complete(
    type_name: str, type_data_raw: TypeData, mutation_type: str | None
) -> TypeDataComplete:
    """
    Build a complete TypeDataComplete dictionary from raw type data.

    Args:
        type_name: The fully-qualified type name
        type_data_raw: Raw type data from all_types.json
        mutation_type: The mutation type (Component/Resource) or None

    Returns:
        Complete TypeDataComplete dictionary
    """
    return cast(
        TypeDataComplete,
        cast(
            object,
            {
                "type_name": type_name,
                "spawn_format": type_data_raw.get("spawn_format"),
                "mutation_paths": type_data_raw.get("mutation_paths"),
                "supported_operations": type_data_raw.get("supported_operations"),
                "in_registry": type_data_raw.get("in_registry"),
                "schema_info": type_data_raw.get("schema_info"),
                "mutation_type": mutation_type,
            },
        ),
    )


def format_type_description(
    type_name: str,
    mutation_type: str | None,
    op_count: int,
    part_number: int | None = None,
    total_parts: int | None = None,
) -> str:
    """
    Format a type description for debug logging.

    Args:
        type_name: Fully-qualified type name
        mutation_type: "Component", "Resource", or None
        op_count: Number of operations for this type/part
        part_number: Optional part number for multi-part types (1-indexed)
        total_parts: Optional total parts for multi-part types

    Returns:
        Formatted description string like "TypeName (C: 10 ops)" or "TypeName (R: 5 ops, 2 of 3)"
    """
    # Find the last :: that appears before any < to handle generic types correctly
    # For "bevy_time::time::Time<bevy_time::time::Real>", we want "Time<bevy_time::time::Real>"
    if "<" in type_name:
        # Find position of first <
        generic_start = type_name.index("<")
        # Find last :: before the <
        prefix = type_name[:generic_start]
        last_separator = prefix.rfind("::")
        if last_separator != -1:
            short_name = type_name[last_separator + 2 :]
        else:
            short_name = type_name
    else:
        short_name = type_name.split("::")[-1]

    category = (
        "C"
        if mutation_type == "Component"
        else "R"
        if mutation_type == "Resource"
        else "?"
    )

    if part_number is not None and total_parts is not None:
        return (
            f"{short_name} ({category}: {op_count} ops, {part_number} of {total_parts})"
        )
    return f"{short_name} ({category}: {op_count} ops)"


ENTITY_ID_PLACEHOLDER = 8589934670  # Placeholder entity ID used in spawn_format


def contains_entity_placeholder(value: Any) -> bool:  # pyright: ignore[reportExplicitAny]
    """
    Check if a value contains the entity ID placeholder anywhere in its structure.

    Note: Uses Any type for recursive JSON traversal - unavoidable for arbitrary JSON structures.
    """
    if isinstance(value, int) and value == ENTITY_ID_PLACEHOLDER:
        return True
    elif isinstance(value, list):
        for item in value:  # pyright: ignore[reportUnknownVariableType]
            if contains_entity_placeholder(item):  # pyright: ignore[reportUnknownArgumentType]
                return True
    elif isinstance(value, dict):
        for val in value.values():  # pyright: ignore[reportUnknownVariableType]
            if contains_entity_placeholder(val):  # pyright: ignore[reportUnknownArgumentType]
                return True

    return False


def generate_test_operations(type_data: TypeDataComplete) -> list[TestOperation]:
    """Generate test operations for a single type."""
    operations: list[TestOperation] = []
    type_name = type_data["type_name"]
    mutation_type = type_data.get("mutation_type")
    spawn_format = type_data.get("spawn_format")
    mutation_paths = type_data.get("mutation_paths") or []

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
                    },
                ),
            )

            # Check for entity ID placeholders
            if contains_entity_placeholder(spawn_format):
                op["entity_id_substitution"] = ENTITY_ID_PLACEHOLDER

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
                    },
                ),
            )

            # Check for entity ID placeholders
            if contains_entity_placeholder(spawn_format):
                op["entity_id_substitution"] = ENTITY_ID_PLACEHOLDER

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
                        "entity": "USE_QUERY_RESULT",
                    },
                ),
            )
        )

    # Step 3: Mutations
    for path_info in mutation_paths:
        # Extract path from the path_info value (path_info is already object type)
        path = cast(str, cast(dict[str, object], path_info)["path"])

        # Skip non-mutable paths
        # Note: path_info dict contains a "path_info" key that holds PathInfo
        path_metadata = cast(dict[str, object], path_info).get("path_info")
        if path_metadata:
            path_metadata_dict = cast(dict[str, object], path_metadata)
            if path_metadata_dict.get("mutability") == "not_mutable":
                continue

            # Skip paths with unavailable root examples (unconstructible enum variants)
            if "root_example_unavailable_reason" in path_metadata_dict:
                continue

            # Check for root example requirement (variant-dependent paths)
            # Always emit root example if present - no optimization
            root_example = path_metadata_dict.get("root_example")
            if root_example is not None:
                # Emit root example operation to set enum variant
                if mutation_type == "Component":
                    root_op = cast(
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
                                "is_root_example": True,
                            },
                        ),
                    )
                else:  # Resource
                    root_op = cast(
                        TestOperation,
                        cast(
                            object,
                            {
                                "operation_id": len(operations),
                                "tool": "mcp__brp__world_mutate_resources",
                                "resource": type_name,
                                "path": "",
                                "value": root_example,
                                "is_root_example": True,
                            },
                        ),
                    )

                # Check for entity ID placeholders in root example
                if contains_entity_placeholder(root_example):
                    root_op["entity_id_substitution"] = ENTITY_ID_PLACEHOLDER

                operations.append(root_op)

        # Get test value for this mutation path (path_info is already object type)
        path_info_dict = cast(dict[str, object], path_info)
        example = path_info_dict.get("example")
        examples = path_info_dict.get("examples")

        # Get the first testable example (one operation per mutation path)
        test_value: object | None = None
        found_example = False
        if examples:
            # For enum variants: find first testable example
            examples_list = cast(list[object], examples)
            for candidate in examples_list:
                if isinstance(candidate, dict):
                    candidate_dict = cast(dict[str, object], candidate)
                    if "example" in candidate_dict:
                        # Found a testable variant (value may be None for Option::None)
                        test_value = candidate_dict["example"]
                        found_example = True
                        break
        elif "example" in path_info_dict:
            test_value = example
            found_example = True

        # Skip if no testable example found
        if not found_example:
            continue

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
                    },
                ),
            )

        if contains_entity_placeholder(test_value):
            op["entity_id_substitution"] = ENTITY_ID_PLACEHOLDER

        operations.append(op)

    return operations


def calculate_type_operations(type_data: TypeDataComplete) -> int:
    """
    Calculate how many operations a type will generate.

    Args:
        type_data: The type to evaluate

    Returns:
        Number of operations this type will generate
    """
    # Generate operations to count them (using placeholder port)
    all_operations = generate_test_operations(type_data)
    return len(all_operations)


def _find_operation_indices(all_operations: list[TestOperation]) -> OperationIndices:
    """
    Find indices of spawn, query, and mutation start operations.

    Args:
        all_operations: List of all operations for a type

    Returns:
        OperationIndices with spawn_idx, query_idx, and mutation_start_idx
    """
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

    return OperationIndices(
        spawn_idx=spawn_idx, query_idx=query_idx, mutation_start_idx=mutation_start_idx
    )


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


def finalize_subagent(
    current_subagent_num: int,
    current_subagent_tests: list[TypeTest],
    current_subagent_descriptions: list[str],
    batch_num: int,
    assignments: list[SubagentAssignment],
) -> None:
    """
    Finalize a subagent by writing its test plan to file and creating an assignment.

    Args:
        current_subagent_num: The subagent number (1-indexed)
        current_subagent_tests: List of tests for this subagent
        current_subagent_descriptions: List of type descriptions for this subagent
        batch_num: The current batch number
        assignments: List to append the new assignment to (modified in place)
    """
    if not current_subagent_tests:
        return  # Nothing to finalize

    port = calculate_port(current_subagent_num, mutation_config)
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
        "subagent_index": current_subagent_num - 1,
        "port": port,
        "test_plan_file": test_plan_file,
        "tests": current_subagent_tests,
    }

    try:
        with open(test_plan_file, "w", encoding="utf-8") as f:
            json.dump(test_plan, f, indent=2)
    except IOError as e:
        print(f"Error writing test plan file: {e}", file=sys.stderr)
        sys.exit(1)

    types_str = ", ".join(current_subagent_descriptions)
    assignment: SubagentAssignment = cast(
        SubagentAssignment,
        cast(
            object,
            {
                "subagent": current_subagent_num,
                "port": port,
                "window_description": f"Subagent {current_subagent_num}: {types_str}",
                "task_description": f"{port} {types_str}",
                "test_plan_file": test_plan_file,
                "type_descriptions": current_subagent_descriptions,
            },
        ),
    )
    assignments.append(assignment)


def split_operations_for_part_new(
    all_operations: list[TestOperation],
    part_number: int,
    total_parts: int,
    slots_per_subagent: int,
    accumulated_slots: int = 0,
) -> list[TestOperation]:
    """
    Split operations for subagent-boundary splitting with GREEDY filling.

    Each part uses as many operations as it can based on available slots.
    Part 1 includes spawn/insert, all parts include query for components.
    Part 2+ includes query + most recent root mutation to re-establish state.

    Greedy strategy: Each part takes `slots_per_subagent` worth of operations,
    filling subagents to capacity before moving to the next.

    Args:
        all_operations: All operations for this type
        part_number: Which part this is (1-indexed)
        total_parts: Total number of parts
        slots_per_subagent: How many slots THIS part gets (greedy allocation)
        accumulated_slots: How many slots have been used by previous parts
    """
    if total_parts == 1:
        return all_operations

    # Find operation indices
    indices = _find_operation_indices(all_operations)
    spawn_idx = indices["spawn_idx"]
    query_idx = indices["query_idx"]
    mutation_start_idx = indices["mutation_start_idx"]

    # Get all mutation operations
    mutations: list[TestOperation] = []
    if mutation_start_idx is not None:
        mutations = all_operations[mutation_start_idx:]

    # Calculate how many mutations have been consumed by previous parts
    # This is different from accumulated_slots because it doesn't count re-emitted operations
    if part_number == 1:
        accumulated_mutations = 0
    else:
        # For part 2+: Calculate actual mutation consumption
        # Part 1 overhead: spawn + query (only counted once)
        part1_overhead = (1 if spawn_idx is not None else 0) + (
            1 if query_idx is not None else 0
        )

        # Parts 2+ overhead per part: query only (root examples are now emitted inline)
        parts_2plus_overhead = 1  # query only

        # How many parts have already run (part_number - 1)
        # Part 1 consumed: (accumulated_slots for part 1) - part1_overhead
        # Parts 2+ each consumed: slots_per_subagent - parts_2plus_overhead
        if part_number == 2:
            # Only part 1 has run
            accumulated_mutations = accumulated_slots - part1_overhead
        else:
            # Part 1 + multiple parts 2+
            # First, get part 1's mutation consumption from the original accumulated_slots
            # We need to track this separately, but for now we can calculate it
            # from the pattern: accumulated_slots includes all operations including re-emits
            #
            # accumulated_slots = part1_total + sum(part2+_totals)
            # part1_total = part1_overhead + part1_mutations
            # part2+_total = parts_2plus_overhead + part2+_mutations
            #
            # For part N (N > 2):
            # accumulated_slots = (part1_overhead + part1_mutations) + (N-2) * (parts_2plus_overhead + partX_mutations)
            # But this is complex. Let's track mutations consumed directly:

            # Actually, we can calculate from accumulated_slots:
            # Remove overhead from all previous parts to get total mutations consumed
            total_overhead = part1_overhead  # Part 1 overhead

            # Each part 2+ adds query + root_mutation overhead
            num_parts_2plus_completed = part_number - 2
            total_overhead += num_parts_2plus_completed * parts_2plus_overhead

            accumulated_mutations = accumulated_slots - total_overhead

    # Calculate how many operations are overhead (spawn + query only)
    # No propagation logic needed - root examples are always emitted
    overhead_ops = 0
    if spawn_idx is not None and part_number == 1:
        overhead_ops += 1
    if query_idx is not None:
        overhead_ops += 1

    # Calculate mutation range based on accumulated mutations (GREEDY)
    mutation_start = accumulated_mutations
    mutations_in_this_part = slots_per_subagent - overhead_ops
    mutation_end = accumulated_mutations + mutations_in_this_part

    # Clamp to actual mutation count
    total_mutations = len(mutations)
    mutation_start = max(0, min(mutation_start, total_mutations))
    mutation_end = max(mutation_start, min(mutation_end, total_mutations))

    # Never split a root example from its follow-on operation
    # If last operation in this part is a root example, extend by 1 to include the pair
    if mutation_end > mutation_start and mutation_end < total_mutations:
        last_included_idx = mutation_end - 1
        if last_included_idx >= 0 and last_included_idx < len(mutations):
            last_op = mutations[last_included_idx]
            if last_op.get("is_root_example"):
                # Extend to include the follow-on operation (allows 1 op overage)
                mutation_end = min(mutation_end + 1, total_mutations)

    result: list[TestOperation] = []

    # Part 1: includes spawn
    if part_number == 1:
        if spawn_idx is not None:
            result.append(all_operations[spawn_idx])

    # All parts: include query for components
    if query_idx is not None:
        result.append(all_operations[query_idx])

    # Add this part's mutations (includes root examples which are now always emitted)
    result.extend(mutations[mutation_start:mutation_end])

    return result


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
    indices = _find_operation_indices(all_operations)
    spawn_idx = indices["spawn_idx"]
    query_idx = indices["query_idx"]
    mutation_start_idx = indices["mutation_start_idx"]

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

# Check if test metadata exists, initialize if missing
type_guide = data["type_guide"]
needs_initialization = False

# Check if any type is missing test metadata
for type_name, type_data in list(type_guide.items())[:5]:  # Check first 5 types
    if "test_status" not in type_data or "batch_number" not in type_data:
        needs_initialization = True
        break

if needs_initialization:
    print("Test metadata not found - initializing...", file=sys.stderr)
    import subprocess

    result = subprocess.run(
        [
            "python3",
            ".claude/scripts/mutation_test/initialize_test_metadata.py",
            "--file",
            json_file,
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"Error initializing test metadata: {result.stderr}", file=sys.stderr)
        sys.exit(1)
    print("Test metadata initialized successfully", file=sys.stderr)

    # Reload the file with metadata
    try:
        with open(json_file, "r") as f:
            data = cast(AllTypesData, json.load(f))
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON after initialization: {e}", file=sys.stderr)
        sys.exit(1)

# Load excluded types list (but don't delete them from data)
excluded_type_names: set[str] = set()
excluded_types_file = Path(".claude/config/mutation_test_excluded_types.json")
if excluded_types_file.exists():
    try:
        with open(excluded_types_file, "r") as f:
            excluded_config_raw = json.load(f)  # pyright: ignore[reportAny]
            excluded_config = cast(ExcludedTypesConfig, excluded_config_raw)
            excluded_type_names = {
                entry["type_name"] for entry in excluded_config["excluded_types"]
            }

            if excluded_type_names:
                print(
                    f"Loaded {len(excluded_type_names)} excluded types (will skip during testing)",
                    file=sys.stderr,
                )
    except (json.JSONDecodeError, IOError) as e:
        print(f"Warning: Failed to load excluded types: {e}", file=sys.stderr)

# Renumber batches before every batch (resets failed→untested, reassigns batch numbers)
data = renumber_batches(
    data, batch_capacity, max_subagents, ops_per_subagent, excluded_type_names
)

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

# Get types for the specified batch
batch_types: list[TypeDataComplete] = []
for type_name, type_info in type_guide.items():
    # Skip excluded types
    if type_name in excluded_type_names:
        continue

    if type_info.get("batch_number") == batch_num:
        # Add type_name to the dict for consistency
        type_item: TypeDataComplete = cast(
            TypeDataComplete, cast(object, {"type_name": type_name, **type_info})
        )
        batch_types.append(type_item)

if not batch_types:
    print(f"No types found for batch {batch_num}", file=sys.stderr)
    sys.exit(1)


# Build complete type data with operations for distribution
# New approach: Track subagent boundaries for splitting
class TypeWithOps(TypedDict):
    type_data: TypeDataComplete
    all_operations: list[TestOperation]
    ops_needed: int


types_with_ops: list[TypeWithOps] = []

for type_item in batch_types:
    # Extract mutation_type from schema_info
    schema_info = type_item.get("schema_info")
    mutation_type = extract_mutation_type(schema_info)

    # Build complete type_data with mutation_type
    type_data = build_type_data_complete(
        type_item["type_name"], type_item, mutation_type
    )

    # Generate all operations for this type
    # Use a placeholder port - will be assigned later
    all_operations = generate_test_operations(type_data)

    # Use actual operation count from generated operations
    ops_needed = len(all_operations)

    types_with_ops.append(
        TypeWithOps(
            type_data=type_data,
            all_operations=all_operations,
            ops_needed=ops_needed,
        )
    )

# Distribute types across subagents with boundary-only splitting
# Track which subagent we're on and how many operations are filled
assignments: list[SubagentAssignment] = []
current_subagent_num = 1
current_subagent_ops_used = 0
current_subagent_tests: list[TypeTest] = []
current_subagent_descriptions: list[str] = []
operation_id_counter = OPERATION_ID_START

for type_with_ops in types_with_ops:
    type_data = type_with_ops["type_data"]
    all_operations = type_with_ops["all_operations"]
    ops_needed = type_with_ops["ops_needed"]
    type_name = type_data["type_name"]
    mutation_type = type_data.get("mutation_type")

    # Check if we've exhausted available subagents
    if current_subagent_num > max_subagents:
        # No more subagents available - stop processing types
        break

    # Check if this type can fit in current subagent without splitting
    ops_remaining_in_subagent = ops_per_subagent - current_subagent_ops_used

    # If type doesn't fit in remaining space AND current subagent is empty, something is wrong
    if ops_needed > ops_per_subagent:
        # Type is too large for any single subagent - must be split
        needs_splitting = True
    elif ops_needed <= ops_remaining_in_subagent:
        # Type fits entirely in current subagent
        needs_splitting = False
    elif not current_subagent_tests:
        # Current subagent is empty, type doesn't fit - must be split or there's a bug
        needs_splitting = True
    else:
        # Type doesn't fit in remaining space, but we have tests already
        # We'll split it to use remaining space in this subagent
        needs_splitting = True

    # Handle non-split case (type fits entirely in current subagent)
    if not needs_splitting:
        # Fits entirely in current subagent (no split needed)
        operations = deepcopy(all_operations)

        # Renumber operation IDs
        port = calculate_port(current_subagent_num, mutation_config)
        for op in operations:
            op["operation_id"] = operation_id_counter
            op["port"] = port
            operation_id_counter += 1

        # Add to current subagent
        test: TypeTest = {
            "type_name": type_name,
            "mutation_type": mutation_type or "Unknown",
            "operations": operations,
        }
        current_subagent_tests.append(test)

        # Format description
        description = format_type_description(type_name, mutation_type, len(operations))
        current_subagent_descriptions.append(description)

        # Update operation count usage
        current_subagent_ops_used += ops_needed

        # If subagent is full, finalize it and start new one
        if current_subagent_ops_used >= ops_per_subagent:
            # Finalize current subagent
            finalize_subagent(
                current_subagent_num,
                current_subagent_tests,
                current_subagent_descriptions,
                batch_num,
                assignments,
            )

            # Clear the current subagent data immediately after finalizing
            current_subagent_tests = []
            current_subagent_descriptions = []

            # Check if we can start a new subagent
            if current_subagent_num >= max_subagents:
                # We've reached the limit - stop processing more types
                break

            # Start new subagent
            current_subagent_num += 1
            current_subagent_ops_used = 0
            operation_id_counter = (
                OPERATION_ID_START  # Reset operation IDs for new subagent
            )

    else:
        # Type needs to span multiple subagents - split at boundaries with GREEDY filling
        remaining_ops = ops_needed
        part_number = 1
        accumulated_ops_so_far = (
            0  # Track how many operations we've used for greedy splitting
        )

        # Calculate total_parts considering current subagent's available space
        # First part uses ops_remaining_in_subagent, remaining parts use full ops_per_subagent
        if ops_remaining_in_subagent > 0:
            after_first_part = ops_needed - ops_remaining_in_subagent
            if after_first_part > 0:
                total_parts = 1 + (
                    (after_first_part + ops_per_subagent - 1) // ops_per_subagent
                )
            else:
                total_parts = 1
        else:
            total_parts = (ops_needed + ops_per_subagent - 1) // ops_per_subagent

        # Check if we have enough subagents to complete all parts
        subagents_needed = current_subagent_num + total_parts - 1
        if subagents_needed > max_subagents:
            # Not enough subagents to complete this multi-part type
            # Try to find a smaller type that fits instead
            remaining_subagents = max_subagents - current_subagent_num + 1
            remaining_capacity = (
                ops_remaining_in_subagent + (remaining_subagents - 1) * ops_per_subagent
            )

            # Search for a smaller type in the remaining types
            found_smaller = False
            current_idx = types_with_ops.index(type_with_ops)
            for check_idx in range(current_idx + 1, len(types_with_ops)):
                check_type_with_ops = types_with_ops[check_idx]
                check_ops_needed = check_type_with_ops["ops_needed"]

                if check_ops_needed <= remaining_capacity:
                    # Calculate parts needed for this smaller type
                    if ops_remaining_in_subagent > 0:
                        check_after_first = check_ops_needed - ops_remaining_in_subagent
                        if check_after_first > 0:
                            check_parts = 1 + (
                                (check_after_first + ops_per_subagent - 1)
                                // ops_per_subagent
                            )
                        else:
                            check_parts = 1
                    else:
                        check_parts = (
                            check_ops_needed + ops_per_subagent - 1
                        ) // ops_per_subagent

                    # Verify this smaller type can complete within remaining subagents
                    check_subagents_needed = current_subagent_num + check_parts - 1
                    if check_subagents_needed <= max_subagents:
                        # Found a smaller type that fits - swap and process it
                        types_with_ops[current_idx], types_with_ops[check_idx] = (
                            types_with_ops[check_idx],
                            types_with_ops[current_idx],
                        )
                        found_smaller = True
                        break

            if not found_smaller:
                # No smaller types found that can complete - done with this batch
                print(
                    f"Note: Skipping type '{type_name}' - requires {total_parts} parts but only {remaining_subagents} subagent(s) remaining",
                    file=sys.stderr,
                )
                break

            # Skip to next iteration to process the swapped smaller type
            continue

        while remaining_ops > 0:
            # Calculate how many slots are available in this subagent
            # For part 1: use remaining space in current subagent
            # For parts 2+: use full subagent capacity
            if part_number == 1 and ops_remaining_in_subagent > 0:
                slots_for_this_part = ops_remaining_in_subagent
            else:
                slots_for_this_part = ops_per_subagent

            # Check if we have slots available for this part
            if slots_for_this_part > 0 and remaining_ops > 0:
                # Get operations for this part (pass slots available, not ops consumed)
                operations = split_operations_for_part_new(
                    all_operations,
                    part_number,
                    total_parts,
                    slots_for_this_part,  # How many slots available in this subagent
                    accumulated_ops_so_far,  # How many operations previous parts used
                )
                operations = deepcopy(operations)

                # Use ACTUAL operation count (accounts for pair-preservation overage)
                actual_ops_in_this_part = len(operations)

                # Renumber operation IDs
                port = calculate_port(current_subagent_num, mutation_config)
                for op in operations:
                    op["operation_id"] = operation_id_counter
                    op["port"] = port
                    operation_id_counter += 1

                # Add to current subagent
                test = cast(
                    TypeTest,
                    cast(
                        object,
                        {
                            "type_name": type_name,
                            "mutation_type": mutation_type or "Unknown",
                            "part_number": part_number,
                            "total_parts": total_parts,
                            "operations": operations,
                        },
                    ),
                )
                current_subagent_tests.append(test)

                # Format description
                description = format_type_description(
                    type_name, mutation_type, len(operations), part_number, total_parts
                )
                current_subagent_descriptions.append(description)

                # Update counters with actual operation count
                current_subagent_ops_used += actual_ops_in_this_part
                ops_remaining_in_subagent -= actual_ops_in_this_part
                remaining_ops -= actual_ops_in_this_part
                accumulated_ops_so_far += actual_ops_in_this_part
                part_number += 1

            # Check if we need to finalize current subagent and start a new one
            if current_subagent_ops_used >= ops_per_subagent or (
                remaining_ops > 0 and ops_remaining_in_subagent == 0
            ):
                # Finalize current subagent
                finalize_subagent(
                    current_subagent_num,
                    current_subagent_tests,
                    current_subagent_descriptions,
                    batch_num,
                    assignments,
                )

                # Clear the current subagent data immediately after finalizing
                current_subagent_tests = []
                current_subagent_descriptions = []

                # Always increment subagent counter after finalizing
                current_subagent_num += 1

                # Check if we can start a new subagent
                if current_subagent_num > max_subagents:
                    # We've reached the limit - stop splitting this type
                    break

                # Start new subagent
                current_subagent_ops_used = 0
                ops_remaining_in_subagent = ops_per_subagent
                operation_id_counter = (
                    OPERATION_ID_START  # Reset operation IDs for new subagent
                )

# Finalize last subagent if it has tests (and wasn't already finalized)
if current_subagent_tests:
    finalize_subagent(
        current_subagent_num,
        current_subagent_tests,
        current_subagent_descriptions,
        batch_num,
        assignments,
    )

# Calculate unique types that made it into assignments (for debug log and progress)
debug_assigned_type_names: set[str] = set()
for assignment in assignments:
    try:
        with open(assignment["test_plan_file"], "r") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            test_plan = cast(TestPlan, test_plan_raw)
            for test in test_plan.get("tests", []):
                type_name = test.get("type_name")
                if type_name:
                    debug_assigned_type_names.add(type_name)
    except (IOError, json.JSONDecodeError):
        pass

unique_types_count = len(debug_assigned_type_names)

# Calculate types remaining after this batch
untested_count = len(
    [t for t in type_guide.values() if t.get("test_status") == "untested"]
)
remaining_types = untested_count - unique_types_count

# Backup and initialize debug log
DEBUG_LOG = get_mutation_test_log(mutation_config)

# Backup existing log if it exists
if os.path.exists(DEBUG_LOG):
    # Extract batch number and timestamp from existing log metadata
    batch_num_str = "unknown"
    log_timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

    try:
        with open(DEBUG_LOG, encoding="utf-8") as f:
            found_batch = False
            found_started = False
            for line in f:
                if line.startswith("# Batch Number:"):
                    parts = line.split()
                    if len(parts) >= 4:
                        batch_num_str = parts[3]
                        found_batch = True
                elif line.startswith("# Started:"):
                    # Extract timestamp from "# Started: 2025-10-22 22:16:12"
                    parts = line.split()
                    if len(parts) >= 4:
                        date_part = parts[2].replace("-", "")
                        time_part = parts[3].replace(":", "")
                        log_timestamp = f"{date_part}_{time_part}"
                        found_started = True

                # Break after finding both
                if found_batch and found_started:
                    break
    except Exception:
        pass  # Use defaults if parsing fails

    log_dir = os.path.dirname(DEBUG_LOG)
    log_name = os.path.splitext(os.path.basename(DEBUG_LOG))[0]
    backup_file = f"{log_dir}/{log_name}_batch{batch_num_str}_{log_timestamp}.log"
    try:
        os.rename(DEBUG_LOG, backup_file)
        print(f"Backed up previous log to: {backup_file}", file=sys.stderr)
    except OSError:
        pass  # Ignore if backup fails

# Create new debug log with metadata for current batch
ports = [a["port"] for a in assignments]
if len(ports) > 0:
    min_port = min(ports)
    max_port = max(ports)
    ports_str = f"{min_port} - {max_port} ({len(ports)} ports)"
else:
    ports_str = "none"
timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

with open(DEBUG_LOG, "w", encoding="utf-8") as f:
    _ = f.write("# Mutation Test Debug Log\n")
    _ = f.write(f"# Started: {timestamp}\n")
    _ = f.write(f"# Batch Number:             {batch_num:>2}\n")
    ops_per_batch = max_subagents * ops_per_subagent
    _ = f.write(f"# Max subagents:            {max_subagents:>2}\n")
    _ = f.write(f"# Ops per Subagent:         {ops_per_subagent:>2}\n")
    _ = f.write(f"# Ops per Batch:           {ops_per_batch:>3}\n")
    _ = f.write(f"# Types remaining:         {remaining_types:>3}\n")
    _ = f.write(f"# Ports: {ports_str}\n")

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
    # Format: "# {subagent_num} ({total_ops} ops) "
    # Total prefix length includes: "# " (2) + num + " (" (2) + ops + " ops) " (6)
    prefix_length = 2 + max_subagent_width + 2 + max_ops_width + 6
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

    def format_type_description_line(
        type_desc: str, max_type_name_length: int, op_count_width: int
    ) -> str:
        """Format a type description line with proper alignment.

        Handles type names with special characters like < and > (e.g., Time<Fixed>).
        Uses .ljust() instead of f-string formatting to avoid issues with < characters.

        Args:
            type_desc: Type description string like "Time<Fixed> (R: 7 ops, 1 of 2)"
            max_type_name_length: Width to pad type name to
            op_count_width: Width for operation count alignment

        Returns:
            Formatted string with padded type name and aligned operation count
        """
        if " (" in type_desc:
            type_name, rest = type_desc.split(" (", 1)
            # Right-align operation count in the rest part
            formatted_rest = format_ops_count(rest, op_count_width)
            # Use .ljust() instead of f-string formatting to avoid issues with < in type names
            return f"{type_name.ljust(max_type_name_length)} ({formatted_rest}"
        return type_desc

    # Calculate max line width for test plan path alignment
    max_line_width = 0
    for idx, assignment in enumerate(assignments):
        total_ops = assignment_ops[idx]
        subagent_num = assignment["subagent"]
        type_list = assignment["type_descriptions"]

        if type_list:
            first_desc = type_list[0]
            if " (" in first_desc:
                type_name, rest = first_desc.split(" (", 1)
                formatted_rest = format_ops_count(rest, op_count_width)
                padded_first = f"{type_name:<{max_type_name_length}} ({formatted_rest}"
            else:
                padded_first = first_desc
            # Calculate the full line width (without test plan path and without colon)
            line = f"# {subagent_num:>{max_subagent_width}} ({total_ops:>{max_ops_width}} ops) {padded_first}"
            max_line_width = max(max_line_width, len(line))

    # Write header line
    type_header = "Type (C=Component, R=Resource: ops, Partition)"
    # Type column should be at least the header length, but can expand if content is longer
    min_type_width = len(type_header)
    # Use actual prefix length (dynamic based on subagent/ops widths), not fixed "# Subagent    "
    actual_content_width = max_line_width - prefix_length
    type_column_width = max(min_type_width, actual_content_width)
    # Subagent column width matches the "N (XX ops)" format: N + " (" (2) + XX + " ops)" (5)
    subagent_column_width = max_subagent_width + 2 + max_ops_width + 5
    header_line = f"# {'Subagent':<{subagent_column_width}} {type_header:<{type_column_width}} Test Plan"
    _ = f.write(f"{header_line}\n")

    # Write separator line to visually partition columns
    subagent_separator = "=" * subagent_column_width
    # Type separator fills the entire type column width
    type_separator = "=" * type_column_width
    # Test plan separator should match the width of the test plan file path
    test_plan_path_width = len(assignments[0]["test_plan_file"]) if assignments else 29
    test_plan_separator = "=" * test_plan_path_width
    # Single space between separators (matching header layout)
    separator_line = f"# {subagent_separator} {type_separator} {test_plan_separator}"
    _ = f.write(f"{separator_line}\n")

    # Write formatted subagent lines
    for idx, assignment in enumerate(assignments):
        total_ops = assignment_ops[idx]
        subagent_num = assignment["subagent"]
        type_list = assignment["type_descriptions"]
        test_plan_file = assignment["test_plan_file"]

        if type_list:
            # First type on same line as subagent info
            first_desc = type_list[0]
            padded_first = format_type_description_line(
                first_desc, max_type_name_length, op_count_width
            )
            # Build line and pad to type column width before adding test plan path
            line = f"# {subagent_num:>{max_subagent_width}} ({total_ops:>{max_ops_width}} ops) {padded_first}"
            # Pad to match header: "# " (2) + subagent_column + " " (1) + type_column
            total_width = 2 + subagent_column_width + 1 + type_column_width
            padded_line = line.ljust(total_width)
            _ = f.write(f"{padded_line} {test_plan_file}\n")

            # Subsequent types indented on their own lines with columnar alignment
            for type_desc in type_list[1:]:
                padded_desc = format_type_description_line(
                    type_desc, max_type_name_length, op_count_width
                )
                _ = f.write(f"#{' ' * continuation_indent}{padded_desc}\n")

    # Write separator line between table and logs
    _ = f.write(f"{separator_line}\n")

print(f"Created new debug log: {DEBUG_LOG}", file=sys.stderr)
print(f"  Batch: {batch_num}", file=sys.stderr)
print(f"  Ports: {ports_str}", file=sys.stderr)
print(f"  Types count: {len(assignments)}", file=sys.stderr)

# Return all assignments with test plan files generated
# Calculate unique types that actually made it into assignments
output_assigned_type_names: set[str] = set()
for assignment in assignments:
    # Open the test plan file to get the actual types assigned
    try:
        with open(assignment["test_plan_file"], "r") as f:
            test_plan_raw = json.load(f)  # pyright: ignore[reportAny]
            test_plan = cast(TestPlan, test_plan_raw)
            for test in test_plan.get("tests", []):
                type_name = test.get("type_name")
                if type_name:
                    output_assigned_type_names.add(type_name)
    except (IOError, json.JSONDecodeError):
        pass

unique_types_count = len(output_assigned_type_names)
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
    "ops_per_subagent": ops_per_subagent,  # Keep original for reference
    "total_types": unique_types_count,
    "progress_message": progress_message,
    "assignments": assignments,
}

# Print summary to stderr for user visibility
print(f"✓ {distribution}", file=sys.stderr)

print(json.dumps(all_assignments_output, indent=2))
