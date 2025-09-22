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
from pathlib import Path


def parse_comparison_output(output_text):
    """Parse the structured comparison output to extract change patterns."""
    patterns = {
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
            patterns['field_removed'][field_name] = {
                'occurrences': int(match.group(2)),
                'types_affected': int(match.group(3))
            }

    # Parse FIELD ADDED patterns
    field_added_match = re.search(r'IDENTIFIED PATTERN: FIELD ADDED.*?Fields added breakdown:(.*?)(?=📌|🔍|$)', output_text, re.DOTALL)
    if field_added_match:
        for match in re.finditer(r"'(\w+)' field: (\d+) addition\(s\) across (\d+) type\(s\)", field_added_match.group(1)):
            field_name = match.group(1)
            patterns['field_added'][field_name] = {
                'occurrences': int(match.group(2)),
                'types_affected': int(match.group(3))
            }

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


def match_expected_changes(patterns, expected_changes):
    """Match detected patterns against expected changes definitions."""
    matched_expected = []
    unmatched_patterns = []

    for expected in expected_changes['expected_changes']:
        pattern_type = expected['pattern_type']
        field = expected.get('field')
        affected_types = expected.get('affected_types', [])
        matched = False

        if pattern_type == 'FIELD_REMOVED' and field in patterns['field_removed']:
            data = patterns['field_removed'][field]
            matched_expected.append({
                'change_id': expected['id'],
                'reason': expected['reason'],
                'occurrences': data['occurrences'],
                'types_affected': data['types_affected'],
                'status': 'matched'
            })
            matched = True
            patterns['field_removed'][field]['processed'] = True

        elif pattern_type == 'FIELD_ADDED' and field in patterns['field_added']:
            data = patterns['field_added'][field]
            matched_expected.append({
                'change_id': expected['id'],
                'reason': expected['reason'],
                'occurrences': data['occurrences'],
                'types_affected': data['types_affected'],
                'status': 'matched'
            })
            matched = True
            patterns['field_added'][field]['processed'] = True

    # Collect any unprocessed patterns as unexpected
    for field, data in patterns['field_removed'].items():
        if not data.get('processed'):
            unmatched_patterns.append({
                'pattern': f"FIELD_REMOVED '{field}'",
                'occurrences': data['occurrences'],
                'types_affected': data['types_affected'],
                'reason': 'No matching expected change definition'
            })

    for field, data in patterns['field_added'].items():
        if not data.get('processed'):
            unmatched_patterns.append({
                'pattern': f"FIELD_ADDED '{field}'",
                'occurrences': data['occurrences'],
                'types_affected': data['types_affected'],
                'reason': 'No matching expected change definition'
            })

    return {
        'expected_matches': matched_expected,
        'unexpected_patterns': unmatched_patterns
    }


def main():
    parser = argparse.ArgumentParser(description='Categorize comparison output against expected changes')
    parser.add_argument('--comparison-output', required=True, help='Path to comparison output file or "-" for stdin')
    parser.add_argument('--expected-changes', required=True, help='Path to expected changes JSON file')
    args = parser.parse_args()

    # Read comparison output
    if args.comparison_output == '-':
        comparison_text = sys.stdin.read()
    else:
        with open(args.comparison_output, 'r') as f:
            comparison_text = f.read()

    # Read expected changes
    with open(args.expected_changes, 'r') as f:
        expected_changes = json.load(f)

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