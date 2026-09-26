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

import glob
import json
import os
import subprocess
import sys
from copy import deepcopy
from datetime import datetime
from pathlib import Path
from typing import TypedDict, cast

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
# Tools whose operations change a value - the operations split across parts
MUTATION_TOOLS = ("mcp__brp__world_mutate_components", "mcp__brp__world_mutate_resources")


# Type definitions for JSON structures (extends config.py's TypeData)
class PathInfo(TypedDict, total=False):
    """Path metadata including mutability and root examples."""

    mutability: str
    example: object
    unavailable_reason: str


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


# Load configuration from config file
try:
    mutation_config = load_config()
except FileNotFoundError as e:
    print(f"Error loading config: {e}", file=sys.stderr)
    sys.exit(1)

max_subagents: int = mutation_config["max_subagents"]
ops_per_subagent: int = mutation_config["ops_per_subagent"]
base_port: int = mutation_config["base_port"]

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

            all_operations = generate_test_operations(type_data)

            # Sanity check: the type must fit in an empty batch
            _, parts_in_empty_batch = plan_type_placement(
                all_operations, ops_per_subagent, ops_per_subagent
            )
            if len(parts_in_empty_batch) > max_subagents:
                print(
                    f"Warning: Type '{type_name}' requires {len(all_operations)} operations "
                    + f"in {len(parts_in_empty_batch)} parts but a batch has only "
                    + f"{max_subagents} subagents. This type will be skipped. "
                    + "Increase max_subagents or ops_per_subagent in config."
                )
                type_guide[type_name]["batch_number"] = -1  # Mark as skipped
                type_idx += 1
                continue

            # Plan the parts from the current position with the same planner the
            # assignment phase uses, so both phases pack the batch identically
            start_offset, parts = plan_type_placement(
                all_operations, ops_per_subagent - current_ops_in_subagent, ops_per_subagent
            )
            last_subagent_idx = current_subagent_idx + start_offset + len(parts) - 1

            if last_subagent_idx < max_subagents:
                type_guide[type_name]["batch_number"] = current_batch
                packed_any_in_batch = True

                # Advance to the subagent holding the last part
                if start_offset == 0 and len(parts) == 1:
                    current_ops_in_subagent += len(parts[0])
                else:
                    current_ops_in_subagent = len(parts[-1])
                current_subagent_idx = last_subagent_idx

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

    # Bevy 0.20 registers `ReflectComponent` alongside `ReflectResource`, so a resource lists
    # both traits; check Resource first
    if "Resource" in reflect_traits:
        return "Resource"
    if "Component" in reflect_traits:
        return "Component"

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
                "spawn": type_data_raw.get("spawn"),
                "resource": type_data_raw.get("resource"),
                "agent_guidance": type_data_raw.get("agent_guidance"),
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
    # For "bevy_time::time::Time<bevy_time::real::Real>", we want "Time<bevy_time::time::Real>"
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


ENTITY_ID_PLACEHOLDER = 8589934670  # Placeholder entity ID used in spawn/resource examples


def contains_entity_placeholder(value: object) -> bool:
    """Check if a value contains the entity ID placeholder anywhere in its structure."""
    if isinstance(value, int) and value == ENTITY_ID_PLACEHOLDER:
        return True
    elif isinstance(value, list):
        return any(contains_entity_placeholder(item) for item in cast(list[object], value))
    elif isinstance(value, dict):
        return any(
            contains_entity_placeholder(val)
            for val in cast(dict[object, object], value).values()
        )

    return False


def extract_example_value(type_data: TypeDataComplete, mutation_type: str | None) -> object | None:
    """
    Extract the example value from spawn or resource.

    Args:
        type_data: Complete type data
        mutation_type: "Component" or "Resource" or None

    Returns:
        The example value if present, None otherwise
    """
    if mutation_type == "Component":
        spawn = type_data.get("spawn")
        if spawn is not None:
            return spawn.get("example")
    elif mutation_type == "Resource":
        resource = type_data.get("resource")
        if resource is not None:
            return resource.get("example")

    return None


def generate_test_operations(type_data: TypeDataComplete) -> list[TestOperation]:
    """Generate test operations for a single type."""
    operations: list[TestOperation] = []
    type_name = type_data["type_name"]
    mutation_type = type_data.get("mutation_type")
    mutation_paths = type_data.get("mutation_paths") or []

    # Extract example value based on mutation_type
    example_value = extract_example_value(type_data, mutation_type)

    # Step 1: Spawn or Insert (if example exists)
    if example_value is not None:
        if mutation_type == "Component":
            # Spawn entity with component
            op = cast(
                TestOperation,
                cast(
                    object,
                    {
                        "operation_id": len(operations),
                        "tool": "mcp__brp__world_spawn_entity",
                        "components": {type_name: example_value},
                    },
                ),
            )

            # Check for entity ID placeholders
            if contains_entity_placeholder(example_value):
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
                        "value": example_value,
                    },
                ),
            )

            # Check for entity ID placeholders
            if contains_entity_placeholder(example_value):
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
        path_info_dict = cast(dict[str, object], path_info)
        path = cast(str, path_info_dict["path"])

        # Skip duplicate paths (marked during deduplication)
        if "duplicate_of" in path_info_dict:
            continue

        # Skip non-mutable paths
        # Note: path_info dict contains a "path_info" key that holds PathInfo
        path_metadata = path_info_dict.get("path_info")
        if path_metadata:
            path_metadata_dict = cast(dict[str, object], path_metadata)
            if path_metadata_dict.get("mutability") == "not_mutable":
                continue

            # Skip paths with unavailable root examples (unconstructible enum variants)
            if "unavailable_reason" in path_metadata_dict:
                continue

            # Check for root example requirement (variant-dependent paths)
            # Always emit root example if present - no optimization
            root_example = path_metadata_dict.get("example")
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


def plan_type_parts(
    all_operations: list[TestOperation],
    first_part_slots: int,
    slots_per_part: int,
) -> list[list[TestOperation]]:
    """
    Split a type's operations into parts that each fit in one subagent, filling greedily.

    Part 1 gets `first_part_slots` (the space left in the current subagent); every later
    part gets `slots_per_part`. Each part runs on a different app instance, so every part
    starts with the type's setup operations - exactly what part 1 has: the spawn (components)
    or insert (resources) when the type has an example, then the query that finds the entity
    id (components). A type without a spawn/insert example has only the query as setup and
    depends on the app creating the type at startup. The mutations follow, in order.

    A part never ends on a root example (path "" setting an enum variant) whose follow-on
    mutation falls in the next part; the root example moves to the next part with its
    follow-on, so no part exceeds its slots.

    Args:
        all_operations: All operations for the type, setup operations first
        first_part_slots: Slots available to part 1
        slots_per_part: Slots available to each later part

    Returns:
        The operations of each part in order: a single part when the type fits in
        `first_part_slots`, and an empty list when `first_part_slots` cannot hold the setup
        operations plus one mutation (the type then starts in the next subagent).

    Raises:
        ValueError: If `slots_per_part` cannot hold the setup operations plus one mutation.
    """
    if len(all_operations) <= first_part_slots:
        return [all_operations]

    setup_operations = [op for op in all_operations if op.get("tool") not in MUTATION_TOOLS]
    mutations = [op for op in all_operations if op.get("tool") in MUTATION_TOOLS]
    if not mutations:
        # Setup operations alone cannot be split - the type needs a fresh subagent
        return []

    parts: list[list[TestOperation]] = []
    mutations_consumed = 0
    slots = first_part_slots
    while mutations_consumed < len(mutations):
        mutation_end = min(
            mutations_consumed + slots - len(setup_operations), len(mutations)
        )
        # Keep a root example in the same part as the mutation that follows it
        if (
            mutations_consumed < mutation_end < len(mutations)
            and mutations[mutation_end - 1].get("is_root_example")
        ):
            mutation_end -= 1

        if mutation_end <= mutations_consumed:
            if not parts:
                return []
            raise ValueError(
                f"{slots_per_part} slots per subagent cannot hold "
                + f"{len(setup_operations)} setup operations plus one mutation"
            )

        parts.append(setup_operations + mutations[mutations_consumed:mutation_end])
        mutations_consumed = mutation_end
        slots = slots_per_part

    return parts


def plan_type_placement(
    all_operations: list[TestOperation],
    slots_left_in_subagent: int,
    ops_per_subagent: int,
) -> tuple[int, list[list[TestOperation]]]:
    """
    Plan a type's parts starting from the current subagent.

    Both batch packing (`renumber_batches`) and test plan assignment call this, so the
    parts they compute always agree.

    Args:
        all_operations: All operations for the type
        slots_left_in_subagent: Unused slots in the current subagent
        ops_per_subagent: Slots in an empty subagent

    Returns:
        `(start_offset, parts)`. `start_offset` is 0 when part 1 goes in the current
        subagent and 1 when the current subagent cannot hold part 1, so the type starts in
        the next one. Part N goes in subagent `current + start_offset + N - 1`.
    """
    parts = plan_type_parts(all_operations, slots_left_in_subagent, ops_per_subagent)
    if parts:
        return 0, parts

    parts = plan_type_parts(all_operations, ops_per_subagent, ops_per_subagent)
    if not parts:
        raise ValueError(
            f"{ops_per_subagent} slots per subagent cannot hold the setup operations "
            + "plus one mutation"
        )
    return 1, parts


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
    test_plan_file = mutation_config["test_plan_file_pattern"].format(port=port)

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

# Always call initialize (it exits early if already initialized)
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

# Print initialization output (will be "Already initialized" or initialization details)
if result.stderr:
    print(result.stderr, file=sys.stderr, end="")

# Reload the file (may have been modified by initialization)
try:
    with open(json_file, "r") as f:
        data = cast(AllTypesData, json.load(f))
except json.JSONDecodeError as e:
    print(f"Error parsing JSON after initialization: {e}", file=sys.stderr)
    sys.exit(1)

type_guide = data["type_guide"]

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

# Deduplication and validation now handled by initialize_test_metadata.py

# Renumber batches before every batch (resets failed→untested, reassigns batch numbers)
data = renumber_batches(data, max_subagents, ops_per_subagent, excluded_type_names)

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
class TypeWithOps(TypedDict):
    type_data: TypeDataComplete
    all_operations: list[TestOperation]


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

    types_with_ops.append(
        TypeWithOps(type_data=type_data, all_operations=all_operations)
    )

# BACKUP OLD TEST FILES BEFORE CREATING NEW ONES
DEBUG_LOG = get_mutation_test_log(mutation_config)
if os.path.exists(DEBUG_LOG):
    # Create timestamped backup folder
    log_timestamp = datetime.now().strftime("%Y%m%d_%H%M")
    log_dir = os.path.dirname(DEBUG_LOG)
    backup_folder = f"{log_dir}/mutation_test_{log_timestamp}"

    try:
        os.makedirs(backup_folder, exist_ok=True)

        # Move mutation_test.log
        backup_log_file = f"{backup_folder}/mutation_test.log"
        os.rename(DEBUG_LOG, backup_log_file)

        # Move all mutation_test_*.json files
        test_plan_pattern = f"{log_dir}/mutation_test_*.json"
        test_plan_files = glob.glob(test_plan_pattern)
        files_moved = 1  # Count the log file

        for test_plan_file in test_plan_files:
            filename = os.path.basename(test_plan_file)
            backup_test_file = f"{backup_folder}/{filename}"
            try:
                os.rename(test_plan_file, backup_test_file)
                files_moved += 1
            except OSError:
                pass  # Continue if individual file move fails

        print(
            f"Backed up {files_moved} test file(s) to: {backup_folder}",
            file=sys.stderr,
        )
    except OSError as e:
        print(f"Warning: Backup failed: {e}", file=sys.stderr)

# Distribute types across subagents in order, splitting a type into parts at subagent
# boundaries. Track which subagent we're on and how many operations are filled
assignments: list[SubagentAssignment] = []
current_subagent_num = 1
current_subagent_ops_used = 0
current_subagent_tests: list[TypeTest] = []
current_subagent_descriptions: list[str] = []
operation_id_counter = OPERATION_ID_START

for type_with_ops in types_with_ops:
    type_data = type_with_ops["type_data"]
    all_operations = type_with_ops["all_operations"]
    type_name = type_data["type_name"]
    mutation_type = type_data.get("mutation_type")

    # Check if we've exhausted available subagents
    if current_subagent_num > max_subagents:
        # No more subagents available - stop processing types
        break

    # Plan the parts the same way renumber_batches did when it packed this batch
    start_offset, parts = plan_type_placement(
        all_operations, ops_per_subagent - current_subagent_ops_used, ops_per_subagent
    )
    total_parts = len(parts)
    last_subagent_num = current_subagent_num + start_offset + total_parts - 1
    if last_subagent_num > max_subagents:
        # Packing guarantees a fit, so this only happens if the plans diverge; the type
        # stays untested and is packed again on the next run
        print(
            f"Note: Skipping type '{type_name}' - its {total_parts} part(s) need subagents "
            + f"through {last_subagent_num} but only {max_subagents} exist",
            file=sys.stderr,
        )
        continue

    for part_number, part_operations in enumerate(parts, start=1):
        # Each later part - and part 1 when the current subagent cannot hold it - goes in
        # the next subagent
        if part_number > 1 or start_offset > 0:
            finalize_subagent(
                current_subagent_num,
                current_subagent_tests,
                current_subagent_descriptions,
                batch_num,
                assignments,
            )
            current_subagent_tests = []
            current_subagent_descriptions = []
            current_subagent_num += 1
            current_subagent_ops_used = 0
            operation_id_counter = OPERATION_ID_START  # Reset operation IDs for new subagent

        operations = deepcopy(part_operations)

        # Renumber operation IDs
        port = calculate_port(current_subagent_num, mutation_config)
        for op in operations:
            op["operation_id"] = operation_id_counter
            op["port"] = port
            operation_id_counter += 1

        test: TypeTest = {
            "type_name": type_name,
            "mutation_type": mutation_type or "Unknown",
            "operations": operations,
        }
        if total_parts > 1:
            test["part_number"] = part_number
            test["total_parts"] = total_parts
        current_subagent_tests.append(test)

        description = (
            format_type_description(
                type_name, mutation_type, len(operations), part_number, total_parts
            )
            if total_parts > 1
            else format_type_description(type_name, mutation_type, len(operations))
        )
        current_subagent_descriptions.append(description)
        current_subagent_ops_used += len(operations)

    # If subagent is full, finalize it and start new one
    if current_subagent_ops_used >= ops_per_subagent:
        finalize_subagent(
            current_subagent_num,
            current_subagent_tests,
            current_subagent_descriptions,
            batch_num,
            assignments,
        )
        current_subagent_tests = []
        current_subagent_descriptions = []
        current_subagent_num += 1
        current_subagent_ops_used = 0
        operation_id_counter = OPERATION_ID_START  # Reset operation IDs for new subagent

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
