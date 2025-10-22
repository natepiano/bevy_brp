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
    assignments: list[SubagentAssignment]


# Test plan types
class TestOperation(TypedDict, total=False):
    tool: str  # MCP tool name
    # Common fields
    port: int
    status: str | None
    error: str | None
    retry_count: int
    # Spawn/query specific
    components: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    filter: dict[str, list[str]] | None
    data: dict[str, Any] | None  # pyright: ignore[reportExplicitAny]
    result_entity_id: int | None
    result_entities: list[int] | None
    # Mutation specific
    entity: str | int | None  # "USE_QUERY_RESULT" or actual entity ID
    component: str | None
    resource: str | None
    path: str | None
    value: Any  # pyright: ignore[reportExplicitAny]
    # Entity ID substitution
    entity_id_substitution: dict[str, str] | None


class TypeTest(TypedDict):
    type_name: str
    mutation_type: str  # "Component" or "Resource"
    operations: list[TestOperation]


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
    This happens only when batch == 1.
    """
    type_guide = data["type_guide"]

    # Step 1: Reset failed tests to untested
    for type_name, type_data in type_guide.items():
        if type_data.get("test_status") == "failed":
            type_data["test_status"] = "untested"
            type_data["fail_reason"] = ""

    # Step 2: Clear all batch numbers
    for type_data in type_guide.values():
        type_data["batch_number"] = None

    # Step 3: Assign batch numbers to untested types
    untested_types: list[str] = [
        type_name
        for type_name, type_data in type_guide.items()
        if type_data.get("test_status") == "untested"
    ]

    for index, type_name in enumerate(untested_types):
        batch_number = (index // batch_size) + 1
        type_guide[type_name]["batch_number"] = batch_number

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
                        "result_entity_id": None,
                        "error": None,
                        "retry_count": 0,
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
                        "result_entities": None,
                        "error": None,
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
    cleanup_patterns = [
        ".claude/transient/batch_results_*.json",
        ".claude/transient/all_types_failures_*.json"
    ]
    for pattern in cleanup_patterns:
        for filepath in glob.glob(pattern):
            try:
                os.remove(filepath)
            except OSError:
                pass  # Ignore errors if files don't exist or can't be removed

# STEP 2: Renumber batches if this is batch 1
if batch_num == 1:
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

# STEP 3: Calculate flexible distribution
total_available_types: int = len(batch_types)

# Calculate optimal distribution: fill subagents with preferred count, handle remainder
base_types_per_subagent: int = min(types_per_subagent, total_available_types)
full_subagents: int = total_available_types // types_per_subagent
remainder_types: int = total_available_types % types_per_subagent

# Determine actual number of subagents needed
if remainder_types > 0:
    actual_subagents_needed: int = min(full_subagents + 1, max_subagents)
else:
    actual_subagents_needed = min(full_subagents, max_subagents)

# STEP 4: Distribute types across subagents and generate test plans
assignments: list[SubagentAssignment] = []
type_index = 0

for subagent_num in range(1, actual_subagents_needed + 1):
    # Determine how many types this subagent gets
    if subagent_num <= full_subagents:
        # This subagent gets the full amount
        types_for_this_subagent = types_per_subagent
    else:
        # This is the last subagent, gets the remainder
        types_for_this_subagent = remainder_types

    subagent_types: list[TypeData] = []
    for _ in range(types_for_this_subagent):
        if type_index < len(batch_types):
            type_item = batch_types[type_index]
            schema_info = type_item.get("schema_info")
            mutation_type = extract_mutation_type(schema_info)
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
            subagent_types.append(type_data)
            type_index += 1

    if subagent_types:  # Only create assignment if there are types
        port = 30000 + subagent_num

        # Generate test plan and formatted descriptions
        tests: list[TypeTest] = []
        type_descriptions: list[str] = []

        for type_data in subagent_types:
            type_name = type_data["type_name"]
            mutation_type = type_data.get("mutation_type")

            # Generate operations for this type
            operations = generate_test_operations(type_data, port)

            # Add to test plan
            test: TypeTest = {
                "type_name": type_name,
                "mutation_type": mutation_type or "Unknown",
                "operations": operations
            }
            tests.append(test)

            # Extract short name (text after last ::)
            short_name = type_name.split("::")[-1]

            # Get category
            category = "C" if mutation_type == "Component" else "R" if mutation_type == "Resource" else "?"

            # Count operations
            op_count = len(operations)

            # Format as "ShortName (C: N ops)" or "ShortName (R: N ops)"
            type_descriptions.append(f"{short_name} ({category}: {op_count} ops)")

        # Create test plan file
        tmpdir = tempfile.gettempdir()
        test_plan_file = os.path.join(tmpdir, f"mutation_test_subagent_{port}_plan.json")

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
all_assignments_output: AllAssignmentsOutput = {
    "batch_number": batch_num,
    "max_subagents": len(assignments),  # Report actual subagents used
    "types_per_subagent": types_per_subagent,  # Keep original for reference
    "total_types": total_available_types,
    "assignments": assignments,
}
print(json.dumps(all_assignments_output, indent=2))
