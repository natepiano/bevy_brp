#!/usr/bin/env python3
"""Rebalance integration test ordering based on prior run times.

Reads timing data and reorders tests in the config using alternating
slowest-to-fastest assignment to balance batch wall times.

Usage:
    # From a JSON timings file: {"test_name": seconds, ...}
    python3 rebalance_tests.py --timings timings.json

    # From key=value pairs on the command line
    python3 rebalance_tests.py watch_component=43.1 query_filtered=62.8 ...

    # Dry run (show new order without writing)
    python3 rebalance_tests.py --dry-run --timings timings.json

    # Custom config path
    python3 rebalance_tests.py --config path/to/config.json --timings timings.json
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import TypedDict


class TimingEntry(TypedDict):
    name: str
    duration: float


class _TestConfigRequired(TypedDict):
    test_name: str
    test_file: str


class TestConfig(_TestConfigRequired, total=False):
    app_name: str
    app_type: str | None
    apps: list[dict[str, object]]


class Config(TypedDict):
    batch_size: int
    tests: list[TestConfig]


DEFAULT_CONFIG = Path(__file__).resolve().parent.parent.parent / "config" / "integration_tests.json"


def load_timings(args: list[str]) -> dict[str, float]:
    """Parse timings from args — either --timings file.json or key=value pairs."""
    timings: dict[str, float] = {}
    timings_file: str | None = None
    i = 0
    while i < len(args):
        if args[i] == "--timings" and i + 1 < len(args):
            timings_file = args[i + 1]
            i += 2
            continue
        if "=" in args[i]:
            name, value = args[i].split("=", 1)
            timings[name.strip()] = float(value.strip())
        i += 1

    if timings_file:
        with open(timings_file) as f:
            file_data: dict[str, float] = json.load(f)  # pyright: ignore[reportAny]
        timings.update(file_data)

    return timings


def rebalance(tests: list[TestConfig], timings: dict[str, float], batch_size: int) -> list[TestConfig]:
    """Reorder tests using alternating assignment for balanced batches.

    Algorithm:
    1. Sort tests by duration descending
    2. Alternate assignment: positions 0,2,4,... -> batch 1; positions 1,3,5,... -> batch 2
    3. Return batch 1 tests followed by batch 2 tests
    """
    num_batches = (len(tests) + batch_size - 1) // batch_size

    entries: list[TimingEntry] = []
    missing: list[str] = []
    for test in tests:
        name = test["test_name"]
        if name in timings:
            entries.append({"name": name, "duration": timings[name]})
        else:
            missing.append(name)

    if missing:
        print(f"WARNING: No timing data for: {', '.join(missing)}", file=sys.stderr)
        print("These tests will be placed at the end with duration 0.", file=sys.stderr)
        for name in missing:
            entries.append({"name": name, "duration": 0.0})

    entries.sort(key=lambda e: e["duration"], reverse=True)

    test_by_name: dict[str, TestConfig] = {t["test_name"]: t for t in tests}
    batches: list[list[TimingEntry]] = [[] for _ in range(num_batches)]

    for i, entry in enumerate(entries):
        batch_idx = i % num_batches
        batches[batch_idx].append(entry)

    result: list[TestConfig] = []
    batch_totals: list[float] = []
    for batch_idx, batch in enumerate(batches):
        total = sum(e["duration"] for e in batch)
        batch_totals.append(total)
        for entry in batch:
            result.append(test_by_name[entry["name"]])

    return result


def main() -> None:
    args = sys.argv[1:]

    config_path = DEFAULT_CONFIG
    dry_run = False

    filtered_args: list[str] = []
    i = 0
    while i < len(args):
        if args[i] == "--config" and i + 1 < len(args):
            config_path = Path(args[i + 1])
            i += 2
            continue
        if args[i] == "--dry-run":
            dry_run = True
            i += 1
            continue
        filtered_args.append(args[i])
        i += 1

    timings = load_timings(filtered_args)
    if not timings:
        print("ERROR: No timing data provided.", file=sys.stderr)
        print(__doc__, file=sys.stderr)
        sys.exit(1)

    with open(config_path) as f:
        config: Config = json.load(f)  # pyright: ignore[reportAny]

    tests = config["tests"]
    batch_size = config["batch_size"]
    num_batches = (len(tests) + batch_size - 1) // batch_size

    rebalanced = rebalance(tests, timings, batch_size)

    print(f"Tests: {len(tests)}, Batch size: {batch_size}, Batches: {num_batches}")
    print()

    for batch_idx in range(num_batches):
        start = batch_idx * batch_size
        end = min(start + batch_size, len(rebalanced))
        batch_tests = rebalanced[start:end]
        batch_total = sum(timings.get(t["test_name"], 0.0) for t in batch_tests)
        max_time = max((timings.get(t["test_name"], 0.0) for t in batch_tests), default=0.0)
        print(f"Batch {batch_idx + 1} (wall ~{max_time:.0f}s, sum {batch_total:.0f}s):")
        for j, test in enumerate(batch_tests):
            name = test["test_name"]
            duration = timings.get(name, 0.0)
            print(f"  {start + j + 1:3d}. {name:<25s} {duration:6.1f}s")
        print()

    if dry_run:
        print("(dry run — config not modified)")
    else:
        config["tests"] = rebalanced
        with open(config_path, "w") as f:
            _ = json.dump(config, f, indent=2)
            _ = f.write("\n")
        print(f"Config updated: {config_path}")


if __name__ == "__main__":
    main()
