#!/usr/bin/env python3
"""
Tool to read specific details from the comprehensive comparison JSON file.
Supports various query modes for reviewing changes interactively.
"""

import json
import sys
import os
from pathlib import Path
from typing import Any, Dict, List, Optional

def get_comparison_file() -> Path:
    """Get the path to the comparison file using TMPDIR."""
    tmpdir = os.environ.get('TMPDIR', '/tmp')
    return Path(tmpdir) / 'mutation_comparison_full.json'

def load_comparison_data() -> Dict[str, Any]:
    """Load the comparison data file."""
    filepath = get_comparison_file()

    if not filepath.exists():
        print(f"❌ Comparison file not found: {filepath}")
        print("   Run create_mutation_test_json_compare.py first")
        sys.exit(1)

    with open(filepath) as f:
        return json.load(f)

def show_summary(data: Dict[str, Any]) -> None:
    """Show the comparison summary."""
    summary = data.get("summary", {})

    print("📊 COMPARISON SUMMARY")
    print("=" * 60)
    print(f"Total changes: {summary.get('total_changes', 0)}")
    print(f"Types modified: {summary.get('types_modified', 0)}")
    print(f"Types added: {summary.get('types_added', 0)}")
    print(f"Types removed: {summary.get('types_removed', 0)}")
    print()

    expected = data.get("expected_patterns", {})
    unexpected = data.get("unexpected_patterns", {})

    if expected:
        print(f"✅ EXPECTED PATTERNS: {len(expected)}")
        for pattern_name, pattern_data in expected.items():
            print(f"   {pattern_data.get('expected_name', pattern_name)}: {pattern_data.get('count', 0)} changes")

    if unexpected:
        print(f"\n⚠️  UNEXPECTED PATTERNS: {len(unexpected)}")
        for i, (pattern_name, pattern_data) in enumerate(unexpected.items(), 1):
            print(f"   {i}. {pattern_name}: {pattern_data.get('count', 0)} changes")

def show_pattern_list(data: Dict[str, Any]) -> None:
    """Show list of all patterns with numbers for selection."""
    unexpected = data.get("unexpected_patterns", {})

    if not unexpected:
        print("✅ No unexpected patterns found!")
        return

    print("⚠️  UNEXPECTED PATTERNS:")
    print("=" * 60)
    for i, (pattern_name, pattern_data) in enumerate(unexpected.items(), 1):
        count = pattern_data.get('count', 0)
        print(f"{i}. {pattern_name}: {count} changes")

    print("\nUse: read_comparison_detail.py pattern <number> to see details")

def show_pattern_details(data: Dict[str, Any], pattern_num: int, limit: int = 5) -> None:
    """Show details for a specific pattern."""
    unexpected = data.get("unexpected_patterns", {})

    pattern_names = list(unexpected.keys())
    if pattern_num < 1 or pattern_num > len(pattern_names):
        print(f"❌ Invalid pattern number. Valid range: 1-{len(pattern_names)}")
        return

    pattern_name = pattern_names[pattern_num - 1]
    pattern_data = unexpected[pattern_name]
    changes = pattern_data.get('changes', [])

    print(f"📌 PATTERN: {pattern_name}")
    print("=" * 60)
    print(f"Total changes: {len(changes)}")
    print(f"\nShowing first {min(limit, len(changes))} changes:")
    print()

    for i, change in enumerate(changes[:limit], 1):
        print(f"Change {i}:")
        print(f"  Type: {change.get('type_name', 'unknown')}")
        print(f"  Path: {change.get('path', 'unknown')}")
        print(f"  Change: {change.get('change_type', 'unknown')}")
        print(f"  Description: {change.get('description', '')}")

        # Show baseline and current values concisely
        baseline = change.get('baseline')
        current = change.get('current')

        if baseline is not None:
            baseline_str = json.dumps(baseline) if not isinstance(baseline, str) else baseline
            if len(baseline_str) > 100:
                baseline_str = baseline_str[:97] + "..."
            print(f"  Baseline: {baseline_str}")

        if current is not None:
            current_str = json.dumps(current) if not isinstance(current, str) else current
            if len(current_str) > 100:
                current_str = current_str[:97] + "..."
            print(f"  Current: {current_str}")
        print()

def get_specific_change(data: Dict[str, Any], pattern_num: int, change_num: int) -> Optional[Dict[str, Any]]:
    """Get a specific change from a pattern."""
    unexpected = data.get("unexpected_patterns", {})

    pattern_names = list(unexpected.keys())
    if pattern_num < 1 or pattern_num > len(pattern_names):
        return None

    pattern_name = pattern_names[pattern_num - 1]
    pattern_data = unexpected[pattern_name]
    changes = pattern_data.get('changes', [])

    if change_num < 1 or change_num > len(changes):
        return None

    change = changes[change_num - 1]
    change['_pattern_name'] = pattern_name
    change['_pattern_num'] = pattern_num
    change['_change_num'] = change_num
    change['_total_in_pattern'] = len(changes)

    return change

def show_full_change(change: Dict[str, Any]) -> None:
    """Show a single change with full details."""
    print("=" * 60)
    print(f"Pattern: {change.get('_pattern_name', 'unknown')} (#{change.get('_pattern_num', 0)})")
    print(f"Change {change.get('_change_num', 0)} of {change.get('_total_in_pattern', 0)}")
    print("=" * 60)

    print(f"Type: {change.get('type_name', 'unknown')}")
    print(f"Path: {change.get('path', 'unknown')}")
    print(f"Change Type: {change.get('change_type', 'unknown')}")
    print(f"Description: {change.get('description', '')}")
    print()

    # Show full baseline and current values
    baseline = change.get('baseline')
    current = change.get('current')

    if baseline is not None:
        print("BASELINE:")
        print("-" * 40)
        if isinstance(baseline, (dict, list)):
            print(json.dumps(baseline, indent=2))
        else:
            print(baseline)
        print()

    if current is not None:
        print("CURRENT:")
        print("-" * 40)
        if isinstance(current, (dict, list)):
            print(json.dumps(current, indent=2))
        else:
            print(current)
        print()

def get_session_state() -> Dict[str, int]:
    """Get the current review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'mutation_review_session.json'

    if session_file.exists():
        with open(session_file) as f:
            return json.load(f)

    return {"pattern": 1, "change": 1}

def save_session_state(pattern_num: int, change_num: int) -> None:
    """Save the current review session state."""
    session_file = Path(os.environ.get('TMPDIR', '/tmp')) / 'mutation_review_session.json'

    with open(session_file, 'w') as f:
        json.dump({"pattern": pattern_num, "change": change_num}, f)

def show_next_change(data: Dict[str, Any]) -> None:
    """Show the next change in the review sequence."""
    state = get_session_state()
    pattern_num = state["pattern"]
    change_num = state["change"]

    # Try to get current change
    change = get_specific_change(data, pattern_num, change_num)

    if not change:
        # Move to next pattern if needed
        unexpected = data.get("unexpected_patterns", {})
        if pattern_num < len(unexpected):
            pattern_num += 1
            change_num = 1
            change = get_specific_change(data, pattern_num, change_num)

    if change:
        show_full_change(change)

        # Save state for next time
        if change_num < change['_total_in_pattern']:
            save_session_state(pattern_num, change_num + 1)
        else:
            # Move to next pattern
            unexpected = data.get("unexpected_patterns", {})
            if pattern_num < len(unexpected):
                save_session_state(pattern_num + 1, 1)
            else:
                print("\n✅ Reached end of all unexpected changes!")
                # Reset to beginning
                save_session_state(1, 1)
    else:
        print("✅ No more changes to review!")
        save_session_state(1, 1)

def main():
    if len(sys.argv) < 2:
        print("Usage:")
        print("  read_comparison_detail.py summary              - Show summary")
        print("  read_comparison_detail.py patterns             - List all patterns")
        print("  read_comparison_detail.py pattern <num> [limit] - Show pattern details")
        print("  read_comparison_detail.py change <pattern> <num> - Show specific change")
        print("  read_comparison_detail.py next                 - Show next change for review")
        sys.exit(1)

    command = sys.argv[1].lower()
    data = load_comparison_data()

    if command == "summary":
        show_summary(data)

    elif command == "patterns":
        show_pattern_list(data)

    elif command == "pattern":
        if len(sys.argv) < 3:
            print("❌ Pattern number required")
            sys.exit(1)
        pattern_num = int(sys.argv[2])
        limit = int(sys.argv[3]) if len(sys.argv) > 3 else 5
        show_pattern_details(data, pattern_num, limit)

    elif command == "change":
        if len(sys.argv) < 4:
            print("❌ Pattern and change numbers required")
            sys.exit(1)
        pattern_num = int(sys.argv[2])
        change_num = int(sys.argv[3])
        change = get_specific_change(data, pattern_num, change_num)
        if change:
            show_full_change(change)
        else:
            print("❌ Change not found")

    elif command == "next":
        show_next_change(data)

    else:
        print(f"❌ Unknown command: {command}")
        sys.exit(1)

if __name__ == "__main__":
    main()