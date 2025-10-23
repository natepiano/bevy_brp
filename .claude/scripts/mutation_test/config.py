#!/usr/bin/env python3
"""
Shared configuration and discovery module for mutation testing.

Provides centralized config loading and batch discovery for both
prepare.py and process_results.py scripts.
"""

import json
from pathlib import Path
from typing import TypedDict


class MutationTestConfig(TypedDict):
    """Configuration for mutation testing."""

    types_per_subagent: int
    max_subagents: int
    base_port: int
    stop_after_each_batch: bool


class TypeData(TypedDict):
    """Type data structure from all_types.json."""

    type_name: str  # Required field


class TypeDataOptional(TypedDict, total=False):
    """Optional fields for TypeData."""

    spawn_format: object | None
    mutation_paths: dict[str, object] | None
    supported_operations: list[str] | None
    in_registry: bool | None
    schema_info: dict[str, object] | None
    batch_number: int | None
    mutation_type: str | None
    test_status: str | None
    fail_reason: str | None


# Combine required and optional fields
class TypeDataComplete(TypeData, TypeDataOptional):
    """Complete TypeData with required and optional fields."""
    pass


class AllTypesData(TypedDict):
    """Structure of all_types.json."""

    type_guide: dict[str, TypeDataComplete]


def load_config() -> MutationTestConfig:
    """
    Load mutation test configuration from .claude/config/mutation_test_config.json.

    Returns:
        MutationTestConfig with all configuration values.

    Raises:
        FileNotFoundError: If config file doesn't exist.
        json.JSONDecodeError: If config file is invalid JSON.
    """
    # Find project root (contains .claude directory)
    current_dir = Path(__file__).resolve().parent
    project_root = current_dir.parent.parent.parent

    config_path = project_root / ".claude/config/mutation_test_config.json"

    if not config_path.exists():
        raise FileNotFoundError(
            (
                f"Config file not found: {config_path}\n"
                "Please create .claude/config/mutation_test_config.json"
            )
        )

    with open(config_path, "r", encoding="utf-8") as f:
        config_data: dict[str, int | bool] = json.load(f)  # pyright: ignore[reportAny]

    return MutationTestConfig(
        types_per_subagent=int(config_data["types_per_subagent"]),
        max_subagents=int(config_data["max_subagents"]),
        base_port=int(config_data["base_port"]),
        stop_after_each_batch=bool(config_data["stop_after_each_batch"]),
    )


def find_current_batch(all_types_data: AllTypesData) -> int | str:
    """
    Find the first batch with untested types.

    Args:
        all_types_data: Parsed all_types.json data.

    Returns:
        Batch number (int) to process next, or "COMPLETE" if all tests passed.
    """
    untested_batches: list[int] = [
        batch_num
        for t in all_types_data["type_guide"].values()
        if t.get("test_status") == "untested"
        and (batch_num := t.get("batch_number")) is not None
    ]

    if untested_batches:
        return min(untested_batches)
    return "COMPLETE"


def get_batch_size(config: MutationTestConfig) -> int:
    """Calculate batch size from config."""
    return config["types_per_subagent"] * config["max_subagents"]


def get_max_port(config: MutationTestConfig) -> int:
    """Calculate maximum port from config."""
    return config["base_port"] + config["max_subagents"] - 1


def get_port_range(config: MutationTestConfig) -> str:
    """Get port range string for display."""
    return f"{config['base_port']}-{get_max_port(config)}"
