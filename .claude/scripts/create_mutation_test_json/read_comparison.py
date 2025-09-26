#!/usr/bin/env python3
"""
Tool to read specific details from the categorized comparison JSON file.
Supports reviewing unexpected changes interactively.
"""

import json
import sys
import os
from pathlib import Path
from typing import TypedDict, cast
from collections import defaultdict

# JSON value type - recursive definition for arbitrary JSON
JsonValue = str | int | float | bool | None | dict[str, "JsonValue"] | list["JsonValue"]

# Type definitions for new categorized JSON structure
class ChangeData(TypedDict):
    type_name: str
    path: str
    change_type: str
    description: str
    baseline: JsonValue
    current: JsonValue

class ExpectedMatchData(TypedDict):
    pattern_name: str
    description: str
    count: int
    expected_id: int
    changes: list[ChangeData]

class CategorizedComparisonData(TypedDict):
    summary: dict[str, JsonValue]
    expected_matches: dict[str, ExpectedMatchData]
    unexpected_changes: list[ChangeData]

def get_comparison_file() -> Path:
    """Get the path to the comparison file using TMPDIR."""
    tmpdir = os.environ.get('TMPDIR', '/tmp')
    return Path(tmpdir) / 'mutation_comparison_full.json'

def load_comparison_data() -> CategorizedComparisonData:
    """Load the categorized comparison data file."""
    filepath = get_comparison_file()

    if not filepath.exists():
        print(f"❌ Comparison file not found: {filepath}")
        print("   Run create_mutation_test_json_compare.py first")
        sys.exit(1)

    with open(filepath) as f:
        data = cast(CategorizedComparisonData, json.load(f))
        return data

def show_summary(data: CategorizedComparisonData) -> None:
    """Show the categorized comparison summary."""
    summary = data.get("summary", {})
    expected_matches = data.get("expected_matches", {})
    unexpected_changes = data.get("unexpected_changes", [])

    print("📊 CATEGORIZED COMPARISON SUMMARY")
    print("=" * 60)
    print(f"Total changes: {summary.get('total_changes', 0)}")
    print(f"Types modified: {summary.get('types_modified', 0)}")
    print(f"Types added: {summary.get('types_added', 0)}")
    print(f"Types removed: {summary.get('types_removed', 0)}")
    print()

    if expected_matches:
        print(f"✅ EXPECTED MATCHES: {len(expected_matches)}")
        for pattern_name, match_info in expected_matches.items():
            print(f"   {pattern_name}: {match_info.get('count', 0)} changes")
            print(f"      {match_info.get('description', '')}")
        print()

    unexpected_count = len(unexpected_changes)
    print(f"⚠️  UNEXPECTED CHANGES: {unexpected_count}")
    if unexpected_count > 0:
        print("   Use 'next' command to review unexpected changes one by one")
    else:
        print("   All changes match expected patterns!")

def show_expected_details(data: CategorizedComparisonData) -> None:
    """Show detailed breakdown of expected matches."""
    expected_matches = data.get("expected_matches", {})

    if not expected_matches:
        print("✅ No expected matches found")
        return

    print("✅ EXPECTED MATCHES DETAILS")
    print("=" * 60)

    for i, (pattern_name, match_info) in enumerate(expected_matches.items(), 1):
        print(f"{i}. {pattern_name}")
        print(f"   Description: {match_info.get('description', '')}")
        print(f"   Count: {match_info.get('count', 0)} changes")
        print(f"   Expected ID: {match_info.get('expected_id', 'unknown')}")

        # Show first few examples
        changes = match_info.get('changes', [])
        if changes:
            print("   Examples:")
            for change in changes[:3]:
                baseline_val = change['baseline']
                current_val = change['current']
                baseline = str(baseline_val) if len(str(baseline_val)) < 30 else str(baseline_val)[:27] + "..."
                current = str(current_val) if len(str(current_val)) < 30 else str(current_val)[:27] + "..."
                print(f"      • {change['type_name']}")
                print(f"        {baseline} → {current}")
            if len(changes) > 3:
                print(f"      ... and {len(changes) - 3} more")
        print()

def get_session_state() -> dict[str, int]:
    """Get the current review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'unexpected_review_session.json'

    if session_file.exists():
        try:
            with open(session_file) as f:
                state = cast(dict[str, int], json.load(f))
                return state
        except (json.JSONDecodeError, KeyError):
            pass

    return {"change_index": 0}

def save_session_state(change_index: int) -> None:
    """Save the current review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'unexpected_review_session.json'

    with open(session_file, 'w') as f:
        json.dump({"change_index": change_index}, f)

def show_next_unexpected(data: CategorizedComparisonData) -> None:
    """Show the next unexpected change for review."""
    unexpected_changes = data.get("unexpected_changes", [])

    if not unexpected_changes:
        print("✅ No unexpected changes to review!")
        return

    state = get_session_state()
    change_index = state["change_index"]

    if change_index >= len(unexpected_changes):
        print("✅ Finished reviewing all unexpected changes!")
        print("   Use 'reset' to start over")
        return

    change = unexpected_changes[change_index]

    # Format exactly as specified in FormatComparison
    print("## Mutation Path Comparison")
    print()
    print(f"**Type**: `{change.get('type_name', 'unknown')}`")

    mutation_path = change.get('mutation_path', '')
    print(f"**Path**: `{mutation_path}`")

    # Create change description based on what changed
    change_type = change.get('change_type', 'unknown')
    description = change.get('description', '')
    if 'examples' in description and 'example' in description:
        change_desc = "examples array → example field"
    elif change_type == 'added':
        change_desc = f"Field added: {description}"
    elif change_type == 'removed':
        change_desc = f"Field removed: {description}"
    else:
        change_desc = description
    print(f"**Change**: {change_desc}")
    print()

    # Show full baseline and current values in the exact format
    baseline = change.get('baseline')
    current = change.get('current')

    print("```json")
    print("// BASELINE")
    if baseline is not None:
        if isinstance(baseline, (dict, list)):
            print(json.dumps(baseline, indent=2))
        else:
            print(json.dumps(baseline))
    else:
        print("(not present)")
    print("```")
    print()

    print("```json")
    print("// CURRENT")
    if current is not None:
        if isinstance(current, (dict, list)):
            print(json.dumps(current, indent=2))
        else:
            print(json.dumps(current))
    else:
        print("(not present)")
    print("```")
    print()

    print(f"[Change {change_index + 1} of {len(unexpected_changes)}]")

    # Save state for next time
    save_session_state(change_index + 1)

def reset_session() -> None:
    """Reset the review session to start from the beginning."""
    save_session_state(0)
    print("🔄 Review session reset to beginning")

def show_unexpected_stats(data: CategorizedComparisonData) -> None:
    """Show statistics about unexpected changes."""
    unexpected_changes = data.get("unexpected_changes", [])

    if not unexpected_changes:
        print("✅ No unexpected changes!")
        return

    print(f"⚠️  UNEXPECTED CHANGES STATISTICS")
    print("=" * 60)
    print(f"Total unexpected changes: {len(unexpected_changes)}")

    # Group by change type
    by_change_type: dict[str, int] = {}
    by_type_name: dict[str, int] = {}

    for change in unexpected_changes:
        change_type = change.get('change_type', 'unknown')
        type_name = change.get('type_name', 'unknown')

        by_change_type[change_type] = by_change_type.get(change_type, 0) + 1
        by_type_name[type_name] = by_type_name.get(type_name, 0) + 1

    print("\nBy change type:")
    for change_type, count in sorted(by_change_type.items(), key=lambda x: x[1], reverse=True):
        print(f"   {change_type}: {count}")

    print(f"\nTop 10 affected types:")
    for type_name, count in sorted(by_type_name.items(), key=lambda x: x[1], reverse=True)[:10]:
        # Shorten long type names
        display_name = type_name if len(type_name) < 50 else type_name[:47] + "..."
        print(f"   {display_name}: {count}")

def get_structural_combinations(data: CategorizedComparisonData) -> dict[str, dict[str, list[ChangeData]]]:
    """Group changes by type and mutation path for structural review."""
    type_path_changes = defaultdict(lambda: defaultdict(list))

    # Process unexpected changes
    unexpected_changes = data.get("unexpected_changes", [])
    for change in unexpected_changes:
        type_name = change.get('type_name', 'unknown')
        mutation_path = change.get('mutation_path')

        # Only include mutation_paths changes
        if mutation_path is not None:
            display_path = 'Root Path ("")' if mutation_path == "" else f'Mutation Path "{mutation_path}"'
            type_path_changes[type_name][display_path].append(change)

    # Process expected changes too
    expected_matches = data.get("expected_matches", {})
    for match_info in expected_matches.values():
        changes = match_info.get('changes', [])
        for change in changes:
            type_name = change.get('type_name', 'unknown')
            mutation_path = change.get('mutation_path')

            if mutation_path is not None:
                display_path = 'Root Path ("")' if mutation_path == "" else f'Mutation Path "{mutation_path}"'
                type_path_changes[type_name][display_path].append(change)

    return dict(type_path_changes)

def show_structural_summary(data: CategorizedComparisonData) -> None:
    """Show structural differences grouped by type and mutation path."""
    type_path_changes = get_structural_combinations(data)

    print("📊 STRUCTURAL DIFFERENCES SUMMARY")
    print("=" * 60)

    if not type_path_changes:
        print("✅ No structural differences found")
        return

    total_combinations = 0
    for type_name in sorted(type_path_changes.keys()):
        paths = type_path_changes[type_name]
        print(f"\n{type_name}:")

        # Sort paths: Root path first, then mutation paths
        sorted_paths = sorted(paths.keys(), key=lambda x: (not x.startswith('Root'), x))

        for path_display in sorted_paths:
            changes = paths[path_display]
            change_count = len(changes)
            change_types = set(change.get('change_type', 'unknown') for change in changes)
            change_summary = ', '.join(sorted(change_types))

            print(f"  {path_display}: {change_count} modifications ({change_summary})")
            total_combinations += 1

    print(f"\n📈 TOTAL: {len(type_path_changes)} types, {total_combinations} type+path combinations")

def get_structural_session_state() -> dict[str, int]:
    """Get the current structural review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'structural_review_session.json'

    if session_file.exists():
        try:
            with open(session_file) as f:
                return json.load(f)
        except (json.JSONDecodeError, KeyError):
            pass

    return {"combination_index": 0}

def save_structural_session_state(combination_index: int) -> None:
    """Save the current structural review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'structural_review_session.json'

    with open(session_file, 'w') as f:
        json.dump({"combination_index": combination_index}, f)

def get_full_mutation_path_data(type_name: str, mutation_path: str, file_path: str) -> JsonValue:
    """Get the complete mutation path data from a file."""
    import json
    from pathlib import Path

    path = Path(file_path)
    if not path.exists():
        return None

    with open(path, 'r') as f:
        file_data = json.load(f)

    # Handle wrapped format
    if 'type_guide' in file_data:
        type_guide = file_data['type_guide']
    else:
        type_guide = file_data

    if type_name not in type_guide:
        return None

    type_data = type_guide[type_name]
    if 'mutation_paths' not in type_data:
        return None

    mutation_paths = type_data['mutation_paths']
    if mutation_path not in mutation_paths:
        return None

    return mutation_paths[mutation_path]

def show_next_structural(data: CategorizedComparisonData) -> None:
    """Show the next type+path combination for structural review."""
    type_path_changes = get_structural_combinations(data)

    if not type_path_changes:
        print("✅ No structural combinations to review!")
        return

    # Flatten to a list of (type_name, path, changes) tuples
    all_combinations = []
    for type_name in sorted(type_path_changes.keys()):
        paths = type_path_changes[type_name]
        sorted_paths = sorted(paths.keys(), key=lambda x: (not x.startswith('Root'), x))
        for path_display in sorted_paths:
            changes = paths[path_display]
            all_combinations.append((type_name, path_display, changes))

    state = get_structural_session_state()
    combination_index = state["combination_index"]

    if combination_index >= len(all_combinations):
        print("✅ Finished reviewing all structural combinations!")
        print(f"   Reviewed {len(all_combinations)} type+path combinations")
        print("   Use 'structural_reset' to start over")
        return

    type_name, path_display, changes = all_combinations[combination_index]

    # Get the actual mutation path from the display string
    mutation_path = path_display.replace('Mutation Path ', '').strip('"')
    if path_display == 'Root Path ("")':
        mutation_path = ""

    # Get the COMPLETE mutation path data for baseline and current
    baseline_path_data = get_full_mutation_path_data(type_name, mutation_path, '.claude/transient/all_types_baseline.json')
    current_path_data = get_full_mutation_path_data(type_name, mutation_path, '.claude/transient/all_types.json')

    # Format exactly as specified in FormatComparison
    print("## Mutation Path Comparison")
    print()
    print(f"**Type**: `{type_name}`")
    print(f"**Path**: `{mutation_path}`")

    # Create a summary of what changed based on the nested changes
    change_summary = []
    has_examples_to_example = False
    for change in changes:
        path = change.get('path', '')
        if 'examples' in path and change.get('change_type') == 'removed':
            has_examples_to_example = True
        elif 'example' in path and change.get('change_type') == 'added':
            has_examples_to_example = True

    if has_examples_to_example:
        change_summary = "examples array → example field pattern across nested fields"
    else:
        # Count change types
        change_types = {}
        for change in changes:
            ct = change.get('change_type', 'unknown')
            change_types[ct] = change_types.get(ct, 0) + 1
        change_summary = f"{len(changes)} nested changes ({', '.join(f'{ct}: {count}' for ct, count in change_types.items())})"

    print(f"**Change**: {change_summary}")
    print()

    # Show the COMPLETE mutation path data
    print("```json")
    print("// BASELINE")
    if baseline_path_data is not None:
        print(json.dumps(baseline_path_data, indent=2))
    else:
        print("(mutation path not present in baseline)")
    print("```")
    print()

    print("```json")
    print("// CURRENT")
    if current_path_data is not None:
        print(json.dumps(current_path_data, indent=2))
    else:
        print("(mutation path not present in current)")
    print("```")
    print()

    print(f"[Structural combination {combination_index + 1} of {len(all_combinations)}]")

    # Save state for next time
    save_structural_session_state(combination_index + 1)

def reset_structural_session() -> None:
    """Reset the structural review session to start from the beginning."""
    save_structural_session_state(0)
    print("🔄 Structural review session reset to beginning")

def show_filtered_changes(data: CategorizedComparisonData, filter_type: str, limit: int = 10) -> None:
    """Show unexpected changes filtered by change type."""
    unexpected_changes = data.get("unexpected_changes", [])

    if not unexpected_changes:
        print("✅ No unexpected changes!")
        return

    # Filter by change type
    filtered_changes = [
        change for change in unexpected_changes
        if change.get('change_type', '').lower() == filter_type.lower()
    ]

    if not filtered_changes:
        print(f"❌ No changes found with type '{filter_type}'")
        available_types = set(change.get('change_type', 'unknown') for change in unexpected_changes)
        print(f"Available types: {', '.join(sorted(available_types))}")
        return

    print(f"🔍 FILTERED CHANGES: {filter_type.upper()}")
    print("=" * 60)
    print(f"Showing first {min(limit, len(filtered_changes))} of {len(filtered_changes)} changes")
    print()

    for i, change in enumerate(filtered_changes[:limit]):
        print(f"{i+1}. Type: {change.get('type_name', 'unknown')}")
        print(f"   Path: {change.get('path', 'unknown')}")

        # Show mutation path if available
        mutation_path = change.get('mutation_path')
        if mutation_path is not None:
            display_path = f'"{mutation_path}"' if mutation_path == "" else mutation_path
            print(f"   Mutation Path: {display_path}")

        print(f"   Description: {change.get('description', '')}")

        # Show baseline and current values (abbreviated)
        baseline = change.get('baseline')
        current = change.get('current')

        if baseline is not None and current is not None:
            baseline_str = str(baseline) if len(str(baseline)) < 50 else str(baseline)[:47] + "..."
            current_str = str(current) if len(str(current)) < 50 else str(current)[:47] + "..."
            print(f"   Change: {baseline_str} → {current_str}")
        elif baseline is not None:
            baseline_str = str(baseline) if len(str(baseline)) < 50 else str(baseline)[:47] + "..."
            print(f"   Removed: {baseline_str}")
        elif current is not None:
            current_str = str(current) if len(str(current)) < 50 else str(current)[:47] + "..."
            print(f"   Added: {current_str}")
        print()

    if len(filtered_changes) > limit:
        print(f"... and {len(filtered_changes) - limit} more changes of this type")

def main() -> None:
    if len(sys.argv) < 2:
        print("📖 CATEGORIZED COMPARISON DETAIL READER")
        print("=" * 60)
        print(f"Reads from: $TMPDIR/mutation_comparison_full.json")
        print()
        print("Commands:")
        print("  summary           Show categorized comparison summary")
        print("  expected          Show expected matches details")
        print("  next              Show next unexpected change for review")
        print("  stats             Show unexpected changes statistics")
        print("  filter TYPE       Show first 10 changes of specific type")
        print("  reset             Reset review session to beginning")
        print()
        print("Structural Review Commands:")
        print("  structural        Show structural differences summary")
        print("  structural_next   Show next type+path combination")
        print("  structural_reset  Reset structural review session")
        print()
        print("Examples:")
        print("  read_comparison.py summary")
        print("  read_comparison.py expected")
        print("  read_comparison.py next")
        print("  read_comparison.py stats")
        print("  read_comparison.py filter removed")
        print("  read_comparison.py structural")
        print("  read_comparison.py structural_next")
        sys.exit(1)

    command = sys.argv[1].lower()
    data = load_comparison_data()

    if command == "summary":
        show_summary(data)
    elif command == "expected":
        show_expected_details(data)
    elif command == "next":
        show_next_unexpected(data)
    elif command == "stats":
        show_unexpected_stats(data)
    elif command == "filter":
        if len(sys.argv) < 3:
            print("❌ Filter command requires a change type")
            print("Usage: read_comparison.py filter TYPE")
            print("Available types: value_changed, removed, added, type_changed")
            sys.exit(1)
        filter_type = sys.argv[2]
        show_filtered_changes(data, filter_type)
    elif command == "reset":
        reset_session()
    elif command == "structural":
        show_structural_summary(data)
    elif command == "structural_next":
        show_next_structural(data)
    elif command == "structural_reset":
        reset_structural_session()
    else:
        print(f"❌ Unknown command: {command}")
        sys.exit(1)

if __name__ == "__main__":
    main()