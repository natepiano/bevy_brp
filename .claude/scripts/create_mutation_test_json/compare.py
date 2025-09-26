#!/usr/bin/env python3
"""
Simple comparison tool for mutation test JSON files.
Outputs raw differences without categorization.
"""

import json
import sys
import os
import re
from pathlib import Path
from datetime import datetime
from typing import TypedDict, cast

# Type definitions for structured data
JsonValue = str | int | float | bool | None | dict[str, "JsonValue"] | list["JsonValue"]

class DifferenceDict(TypedDict):
    path: str
    change_type: str
    baseline: JsonValue
    current: JsonValue
    description: str
    type_name: str
    mutation_path: str | None

class TypeStatsDict(TypedDict):
    baseline_only: list[str]
    current_only: list[str]
    modified: list[str]

class ComparisonResultDict(TypedDict):
    all_changes: list[DifferenceDict]
    type_stats: TypeStatsDict

class MetadataDict(TypedDict):
    generated_at: str
    output_version: str

class SummaryDict(TypedDict):
    total_changes: int
    types_modified: int
    types_added: int
    types_removed: int

class OutputDict(TypedDict):
    metadata: MetadataDict
    summary: SummaryDict
    all_changes: list[DifferenceDict]

class ExpectedChangeDict(TypedDict):
    id: int
    pattern_type: str
    description: str
    change_type: str
    path_regex: str
    value_condition: str

class ExpectedMatchDict(TypedDict):
    pattern_name: str
    description: str
    count: int
    expected_id: int
    changes: list[DifferenceDict]

class FileStatsDict(TypedDict):
    total_types: int
    spawn_supported: int
    types_with_mutations: int
    total_mutation_paths: int

class CategorizedOutputDict(TypedDict):
    metadata: MetadataDict
    current_file_stats: FileStatsDict
    comparison_summary: SummaryDict
    expected_matches: dict[str, ExpectedMatchDict]
    unexpected_changes: list[DifferenceDict]

def get_output_path() -> Path:
    """Get the output path using TMPDIR environment variable."""
    tmpdir = os.environ.get('TMPDIR', '/tmp')
    return Path(tmpdir) / 'mutation_comparison_full.json'

def show_expected_changes_help() -> None:
    """Show help for creating expected changes entries."""
    print("""
📖 EXPECTED CHANGES FORMAT

When creating .claude/config/create_mutation_test_json_expected_changes.json:

{
  "expected_changes": [
    {
      "id": 1,
      "pattern_type": "DESCRIPTIVE_CATEGORY_NAME",
      "description": "Human-readable explanation of what changed and why it's expected",
      "change_type": "value_changed|added|removed|type_changed",
      "path_regex": "regex pattern to match the path field",
      "value_condition": "python expression using 'baseline' and 'current' variables"
    }
  ]
}

EXAMPLE - Enum Variant Qualified Names:
{
  "id": 1,
  "pattern_type": "ENUM_VARIANT_QUALIFIED_NAMES",
  "description": "Enum variants changed from simple names to fully qualified names",
  "change_type": "value_changed",
  "path_regex": "mutation_paths\\\\.\\\\.*\\\\.examples\\\\[\\\\d+\\\\]\\\\.applicable_variants\\\\[\\\\d+\\\\]$",
  "value_condition": "current.endswith('::' + baseline) and '::' not in baseline and '::' in current"
}

MATCHING LOGIC:
- Changes are matched exclusively (first match wins)
- More specific patterns should have lower ID numbers
- value_condition is Python code with 'baseline' and 'current' variables
- path_regex uses Python regex syntax (remember to escape backslashes)

Run: python3 compare.py --help-expected-changes
""")
    sys.exit(0)

def load_files(baseline_path: str, current_path: str) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    """Load baseline and current JSON files."""
    with open(baseline_path) as f:
        baseline_data: JsonValue = cast(JsonValue, json.load(f))

    with open(current_path) as f:
        current_data: JsonValue = cast(JsonValue, json.load(f))

    # Extract type guides - safely handle the case where data might not be a dict
    if isinstance(baseline_data, dict):
        baseline = baseline_data.get('type_guide', baseline_data)
    else:
        baseline = baseline_data

    if isinstance(current_data, dict):
        current = current_data.get('type_guide', current_data)
    else:
        current = current_data

    # Ensure we return the correct type
    if not isinstance(baseline, dict):
        baseline = {}
    if not isinstance(current, dict):
        current = {}

    return baseline, current

def extract_mutation_path(path: str) -> str | None:
    """Extract the mutation path key from a JSON path if it's within mutation_paths."""
    if not path.startswith("mutation_paths."):
        return None

    # Handle double-dot case: could be root metadata or mutation path data
    if path.startswith("mutation_paths.."):
        # Get what comes after the double dots
        rest_after_dots = path[len("mutation_paths.."):]

        # We need to find where the mutation path ends and the nested field begins
        # Mutation paths can contain dots (like .z_config.far_z_mode)
        # The nested field is what comes after the mutation path

        # Known nested field patterns that indicate end of mutation path
        nested_field_indicators = [
            ".example", ".examples", ".path_info", ".description",
            ".mutation_status", ".type", ".type_kind", ".enum_variant_path",
            ".applicable_variants", ".signature"
        ]

        # Find the first occurrence of a nested field indicator
        mutation_path = ""
        for indicator in nested_field_indicators:
            if indicator in rest_after_dots:
                # Everything before this indicator is the mutation path
                mutation_path = rest_after_dots.split(indicator)[0]
                return f".{mutation_path}" if mutation_path else ""

        # If no indicator found, check if it's a metadata field at root
        if "." in rest_after_dots:
            first_part = rest_after_dots.split(".", 1)[0]
            if first_part in ["path_info", "description"]:
                return ""

        # If no nested field indicators found, the entire rest is the mutation path
        return f".{rest_after_dots}" if rest_after_dots else ""

    # Normal case: mutation_paths.some_key.rest -> ".some_key"
    parts = path.split(".", 3)  # Split into max 4 parts: ["mutation_paths", "key", "rest", ...]
    if len(parts) >= 2 and parts[1]:  # Check that the key part exists and is not empty
        mutation_key = parts[1]
        return f".{mutation_key}"

    return ""

def describe_value(val: JsonValue) -> str:
    """Create a concise description of a value."""
    match val:
        case None:
            return "null"
        case bool():
            return str(val).lower()
        case int() | float():
            return str(val)
        case str():
            if len(val) > 50:
                return f'"{val[:47]}..."'
            return f'"{val}"'
        case list():
            return f"array[{len(val)}]"
        case dict():
            keys = list(val.keys())[:3]
            if len(keys) < len(val):
                keys.append("...")
            return f"object{{{','.join(str(k) for k in keys)}}}"

def deep_compare_values(path: str, baseline_val: JsonValue, current_val: JsonValue) -> list[DifferenceDict]:
    """Recursively compare values and return all differences."""
    differences: list[DifferenceDict] = []

    # Check if both are None
    if baseline_val is None and current_val is None:
        return []

    # Check if one is None
    if baseline_val is None:
        differences.append(DifferenceDict(
            path=path,
            change_type="added",
            baseline=None,
            current=current_val,
            description=f"Added: {describe_value(current_val)}",
            type_name="",
            mutation_path=extract_mutation_path(path)
        ))
        return differences

    if current_val is None:
        differences.append(DifferenceDict(
            path=path,
            change_type="removed",
            baseline=baseline_val,
            current=None,
            description=f"Removed: {describe_value(baseline_val)}",
            type_name="",
            mutation_path=extract_mutation_path(path)
        ))
        return differences

    # Check if types differ
    if type(baseline_val) != type(current_val):
        differences.append(DifferenceDict(
            path=path,
            change_type="type_changed",
            baseline=baseline_val,
            current=current_val,
            description=f"Type changed: {type(baseline_val).__name__} → {type(current_val).__name__}",
            type_name="",
            mutation_path=extract_mutation_path(path)
        ))
        return differences

    # Compare based on type
    if isinstance(baseline_val, dict) and isinstance(current_val, dict):
        all_keys = set(baseline_val.keys()) | set(current_val.keys())
        for key in sorted(all_keys):
            new_path = f"{path}.{key}" if path else str(key)
            base_item = baseline_val.get(key)
            curr_item = current_val.get(key)
            differences.extend(deep_compare_values(new_path, base_item, curr_item))

    elif isinstance(baseline_val, list) and isinstance(current_val, list):
        # For lists, compare by index
        max_len = max(len(baseline_val), len(current_val))
        for i in range(max_len):
            new_path = f"{path}[{i}]"
            base_item = baseline_val[i] if i < len(baseline_val) else None
            curr_item = current_val[i] if i < len(current_val) else None

            if base_item is None and curr_item is not None:
                differences.append(DifferenceDict(
                    path=new_path,
                    change_type="added",
                    baseline=None,
                    current=curr_item,
                    description=f"Added element at index {i}",
                    type_name="",
                    mutation_path=extract_mutation_path(new_path)
                ))
            elif base_item is not None and curr_item is None:
                differences.append(DifferenceDict(
                    path=new_path,
                    change_type="removed",
                    baseline=base_item,
                    current=None,
                    description=f"Removed element at index {i}",
                    type_name="",
                    mutation_path=extract_mutation_path(new_path)
                ))
            else:
                differences.extend(deep_compare_values(new_path, base_item, curr_item))

    elif baseline_val != current_val:
        # Primitive values that differ
        differences.append(DifferenceDict(
            path=path,
            change_type="value_changed",
            baseline=baseline_val,
            current=current_val,
            description=f"Value changed: {describe_value(baseline_val)} → {describe_value(current_val)}",
            type_name="",
            mutation_path=extract_mutation_path(path)
        ))

    return differences

def compare_types(baseline: dict[str, JsonValue], current: dict[str, JsonValue]) -> ComparisonResultDict:
    """Compare all types and collect ALL differences."""
    all_changes: list[DifferenceDict] = []
    type_stats: TypeStatsDict = {
        "baseline_only": [],
        "current_only": [],
        "modified": []
    }

    all_types = set(baseline.keys()) | set(current.keys())

    for type_name in sorted(all_types):
        if type_name not in current:
            type_stats["baseline_only"].append(type_name)
            continue
        if type_name not in baseline:
            type_stats["current_only"].append(type_name)
            continue

        # Compare the type entries
        baseline_entry = baseline[type_name]
        current_entry = current[type_name]

        type_differences = deep_compare_values("", baseline_entry, current_entry)

        if type_differences:
            type_stats["modified"].append(type_name)
            for diff in type_differences:
                diff["type_name"] = type_name
                all_changes.append(diff)

    return ComparisonResultDict(
        all_changes=all_changes,
        type_stats=type_stats
    )

def load_expected_changes() -> list[ExpectedChangeDict]:
    """Load expected changes configuration file."""
    config_path = Path('.claude/config/create_mutation_test_json_expected_changes.json')

    if not config_path.exists():
        return []

    try:
        with open(config_path) as f:
            data = cast(dict[str, JsonValue], json.load(f))
            expected_changes = data.get('expected_changes', [])
            if isinstance(expected_changes, list):
                return cast(list[ExpectedChangeDict], expected_changes)
            return []
    except (json.JSONDecodeError, KeyError):
        return []

def matches_expected_pattern(change: DifferenceDict, pattern: ExpectedChangeDict) -> bool:
    """Check if a change matches an expected pattern."""
    # Check change type
    if change['change_type'] != pattern['change_type']:
        return False

    # Check path pattern
    try:
        if not re.match(pattern['path_regex'], change['path']):
            return False
    except re.error:
        return False

    # Check value condition
    try:
        baseline = change['baseline']
        current = change['current']

        # Create a safe environment for eval
        eval_globals = {
            'baseline': baseline,
            'current': current,
            'isinstance': isinstance,
            'str': str,
            'dict': dict,
            'list': list,
            'int': int,
            'float': float,
            'bool': bool,
            'type': type
        }

        eval_result = cast(object, eval(pattern['value_condition'], {"__builtins__": {}}, eval_globals))
        return bool(eval_result)
    except:
        return False

def categorize_changes(all_changes: list[DifferenceDict], expected_patterns: list[ExpectedChangeDict]) -> tuple[dict[str, ExpectedMatchDict], list[DifferenceDict]]:
    """Categorize changes into expected and unexpected groups with exclusive matching."""
    unmatched_changes = all_changes.copy()  # Start with all changes
    expected_matches: dict[str, ExpectedMatchDict] = {}

    # Process patterns in ID order (most specific first)
    sorted_patterns = sorted(expected_patterns, key=lambda p: p['id'])

    for pattern in sorted_patterns:
        matches: list[DifferenceDict] = []
        remaining: list[DifferenceDict] = []

        # Check each unmatched change
        for change in unmatched_changes:
            if matches_expected_pattern(change, pattern):
                matches.append(change)  # Consume this change
            else:
                remaining.append(change)  # Keep for next pattern

        # Update results if we found matches
        if matches:
            expected_matches[pattern['pattern_type']] = ExpectedMatchDict(
                pattern_name=pattern['pattern_type'],
                description=pattern['description'],
                count=len(matches),
                expected_id=pattern['id'],
                changes=matches
            )

        unmatched_changes = remaining  # Only unmatched remain

    return expected_matches, unmatched_changes

def calculate_file_statistics(data: dict[str, JsonValue]) -> FileStatsDict:
    """Calculate statistics for a mutation test file."""
    total_types = len(data)
    spawn_supported = sum(1 for t in data.values() if isinstance(t, dict) and t.get('spawn_format') is not None)
    types_with_mutations = sum(1 for t in data.values() if isinstance(t, dict) and t.get('mutation_paths'))
    total_paths = sum(
        len(cast(dict[str, JsonValue], t.get('mutation_paths', {}))) if isinstance(t.get('mutation_paths'), dict) else 0
        for t in data.values()
        if isinstance(t, dict)
    )

    return FileStatsDict(
        total_types=total_types,
        spawn_supported=spawn_supported,
        types_with_mutations=types_with_mutations,
        total_mutation_paths=total_paths
    )

def main() -> None:
    if '--help-expected-changes' in sys.argv:
        show_expected_changes_help()

    if len(sys.argv) != 3:
        print("Usage: create_mutation_test_json_compare.py <baseline.json> <current.json>")
        print("       create_mutation_test_json_compare.py --help-expected-changes")
        sys.exit(1)

    baseline_path = sys.argv[1]
    current_path = sys.argv[2]

    # Load files
    baseline, current = load_files(baseline_path, current_path)

    # Calculate statistics for current file
    current_stats = calculate_file_statistics(current)

    # Compare types and get ALL changes
    comparison_result = compare_types(baseline, current)
    all_changes = comparison_result["all_changes"]
    type_stats = comparison_result["type_stats"]

    # Extract stats safely
    modified_types = type_stats["modified"]
    added_types = type_stats["current_only"]
    removed_types = type_stats["baseline_only"]

    # Load expected changes and categorize
    expected_patterns = load_expected_changes()
    expected_matches, unexpected_changes = categorize_changes(all_changes, expected_patterns)

    print("🔍 MUTATION TEST COMPARISON COMPLETE")
    print("=" * 60)
    print()
    print("Current File Statistics:")
    print(f"  Types registered in Bevy: {current_stats['total_types']}")
    print(f"  Spawn-supported types: {current_stats['spawn_supported']}")
    print(f"  Types with mutations: {current_stats['types_with_mutations']}")
    print(f"  Total mutation paths: {current_stats['total_mutation_paths']}")
    print()
    print("Comparison Results:")
    print("=" * 60)
    print(f"Total changes: {len(all_changes)}")
    print(f"Types modified: {len(modified_types)}")
    print(f"Types added: {len(added_types)}")
    print(f"Types removed: {len(removed_types)}")
    print()

    if expected_matches:
        print("✅ EXPECTED CHANGES:")
        for pattern_name, match_info in expected_matches.items():
            print(f"   {pattern_name}: {match_info['count']} changes")
            print(f"      {match_info['description']}")
        print()

    unexpected_count = len(unexpected_changes)
    if unexpected_count > 0:
        print(f"⚠️  UNEXPECTED CHANGES: {unexpected_count}")
        print("   These changes need review - use comparison_review to examine them")
    else:
        print("✅ All changes match expected patterns!")

    print()
    print("Detailed results saved to comparison file")

    # Save categorized results
    output_path = get_output_path()
    categorized_result: CategorizedOutputDict = {
        "metadata": {
            "generated_at": datetime.now().isoformat(),
            "output_version": "2.0.0"
        },
        "current_file_stats": current_stats,
        "comparison_summary": {
            "total_changes": len(all_changes),
            "types_modified": len(modified_types),
            "types_added": len(added_types),
            "types_removed": len(removed_types)
        },
        "expected_matches": expected_matches,
        "unexpected_changes": unexpected_changes
    }

    with open(output_path, 'w') as f:
        json.dump(categorized_result, f, indent=2)

if __name__ == "__main__":
    main()