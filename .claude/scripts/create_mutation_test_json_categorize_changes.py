#!/usr/bin/env python3
"""
Categorize comparison output changes against expected changes JSON.

This script parses the structured comparison output and matches patterns
against the expected changes definitions to separate expected from unexpected changes.
"""

import json
import sys
import re
import argparse
from typing import TypedDict


class FieldData(TypedDict):
    occurrences: int
    types_affected: int
    processed: bool | None


class Patterns(TypedDict):
    field_removed: dict[str, FieldData]
    field_added: dict[str, FieldData]
    value_changes: int
    new_types: list[str]
    removed_types: list[str]
    modified_types: list[str]


class ExpectedChange(TypedDict):
    id: str
    pattern_type: str
    field: str | None
    affected_types: list[str] | None
    reason: str


class ExpectedChanges(TypedDict):
    expected_changes: list[ExpectedChange]


class MatchedExpected(TypedDict):
    change_id: str
    reason: str
    occurrences: int
    types_affected: int
    status: str


class UnmatchedPattern(TypedDict):
    pattern: str
    occurrences: int
    types_affected: int
    reason: str


class MatchResults(TypedDict):
    expected_matches: list[MatchedExpected]
    unexpected_patterns: list[UnmatchedPattern]


def parse_comparison_output(output_text: str) -> Patterns:
    """Parse the structured comparison output to extract change patterns."""
    patterns: Patterns = {
        'field_removed': {},
        'field_added': {},
        'value_changes': 0,
        'new_types': [],
        'removed_types': [],
        'modified_types': []
    }

    # Parse FIELD REMOVED patterns
    field_removed_match = re.search(r'IDENTIFIED PATTERN: FIELD REMOVED.*?Fields removed breakdown:(.*?)(?=📌|🔍|$)', output_text, re.DOTALL)
    if field_removed_match:
        for match in re.finditer(r"'(\w+)' field: (\d+) removal\(s\) across (\d+) type\(s\)", field_removed_match.group(1)):
            field_name = match.group(1)
            patterns['field_removed'][field_name] = FieldData(
                occurrences=int(match.group(2)),
                types_affected=int(match.group(3)),
                processed=None
            )

    # Parse FIELD ADDED patterns
    field_added_match = re.search(r'IDENTIFIED PATTERN: FIELD ADDED.*?Fields added breakdown:(.*?)(?=📌|🔍|$)', output_text, re.DOTALL)
    if field_added_match:
        for match in re.finditer(r"'(\w+)' field: (\d+) addition\(s\) across (\d+) type\(s\)", field_added_match.group(1)):
            field_name = match.group(1)
            patterns['field_added'][field_name] = FieldData(
                occurrences=int(match.group(2)),
                types_affected=int(match.group(3)),
                processed=None
            )

    # Parse VALUE CHANGE patterns
    value_change_match = re.search(r'IDENTIFIED PATTERN: VALUE CHANGE.*?Total changes: (\d+)', output_text, re.DOTALL)
    if value_change_match:
        patterns['value_changes'] = int(value_change_match.group(1))

    # Parse new types
    new_types_match = re.search(r'New Types:.*?(?:├─|│\s+├─)\s+([\w:]+)', output_text)
    if new_types_match:
        # Find all new type entries
        for match in re.finditer(r'(?:├─|│\s+├─)\s+([\w:]+)', output_text):
            type_name = match.group(1)
            if '::' in type_name:  # Valid type name
                patterns['new_types'].append(type_name)

    return patterns


def match_expected_changes(patterns: Patterns, expected_changes: ExpectedChanges) -> MatchResults:
    """Match detected patterns against expected changes definitions."""
    matched_expected: list[MatchedExpected] = []
    unmatched_patterns: list[UnmatchedPattern] = []

    for expected in expected_changes['expected_changes']:
        pattern_type = expected['pattern_type']
        field = expected.get('field')
        # affected_types = expected.get('affected_types', [])  # Currently unused
        # matched = False  # Currently unused

        if pattern_type == 'FIELD_REMOVED' and field and field in patterns['field_removed']:
            data = patterns['field_removed'][field]
            matched_expected.append(MatchedExpected(
                change_id=expected['id'],
                reason=expected['reason'],
                occurrences=data['occurrences'],
                types_affected=data['types_affected'],
                status='matched'
            ))
            # matched = True  # Currently unused
            patterns['field_removed'][field]['processed'] = True

        elif pattern_type == 'FIELD_ADDED' and field and field in patterns['field_added']:
            data = patterns['field_added'][field]
            matched_expected.append(MatchedExpected(
                change_id=expected['id'],
                reason=expected['reason'],
                occurrences=data['occurrences'],
                types_affected=data['types_affected'],
                status='matched'
            ))
            # matched = True  # Currently unused
            patterns['field_added'][field]['processed'] = True

    # Collect any unprocessed patterns as unexpected
    for field, data in patterns['field_removed'].items():
        if not data.get('processed'):
            unmatched_patterns.append(UnmatchedPattern(
                pattern=f"FIELD_REMOVED '{field}'",
                occurrences=data['occurrences'],
                types_affected=data['types_affected'],
                reason='No matching expected change definition'
            ))

    for field, data in patterns['field_added'].items():
        if not data.get('processed'):
            unmatched_patterns.append(UnmatchedPattern(
                pattern=f"FIELD_ADDED '{field}'",
                occurrences=data['occurrences'],
                types_affected=data['types_affected'],
                reason='No matching expected change definition'
            ))

    return MatchResults(
        expected_matches=matched_expected,
        unexpected_patterns=unmatched_patterns
    )


def main() -> None:
    parser = argparse.ArgumentParser(description='Categorize comparison output against expected changes')
    _ = parser.add_argument('--comparison-output', required=True, help='Path to comparison output file or "-" for stdin')
    _ = parser.add_argument('--expected-changes', required=True, help='Path to expected changes JSON file')
    args = parser.parse_args()

    # Read comparison output
    comparison_output: str = args.comparison_output  # pyright: ignore[reportAny]
    expected_changes_file: str = args.expected_changes  # pyright: ignore[reportAny]
    if comparison_output == '-':
        comparison_text = sys.stdin.read()
    else:
        with open(comparison_output, 'r') as f:
            comparison_text = f.read()

    # Read expected changes
    with open(expected_changes_file, 'r') as f:
        expected_changes: ExpectedChanges = json.load(f)  # pyright: ignore[reportAny]

    # Parse patterns from comparison output
    patterns = parse_comparison_output(comparison_text)

    # Match against expected changes
    results = match_expected_changes(patterns, expected_changes)

    # Output results
    print(json.dumps(results, indent=2))

    # Exit with status based on unexpected patterns
    if results['unexpected_patterns']:
        sys.exit(1)  # Unexpected patterns found
    else:
        sys.exit(0)  # All patterns matched expected changes


if __name__ == '__main__':
    main()