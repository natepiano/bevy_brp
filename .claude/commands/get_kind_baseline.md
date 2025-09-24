# Get Type Kind

Analyzes type_kind values in mutation paths from the baseline file.

## Command Execution

When you request type kind analysis, I will:

1. Verify that the baseline file exists at `.claude/transient/all_types_baseline.json`
2. Execute the appropriate script mode based on arguments provided
3. Process the JSON data using Python to extract type_kind information
4. Present results in a clear, readable format

<UserOutput>
## For summary mode:
## Type Kind Summary

```
Type kind summary (types containing at least one mutation path of each kind):

${TYPE_KIND_COUNTS}
```

## For query mode:
## Types with type_kind '${TYPE_KIND}'

```
Types containing mutation paths with type_kind '${TYPE_KIND}':

${TYPE_NAME_LIST}
```
</UserOutput>

## Usage

### Summary Mode (no arguments)
Shows a count of how many top-level types contain at least one mutation path of each type_kind:

```bash
/get_kind_baseline
```

### Query Mode (with type_kind argument)
Shows all top-level type names that contain at least one mutation path with the specified type_kind:

```bash
/get_kind_baseline List
/get_kind_baseline Struct
/get_kind_baseline Value
```

## Prerequisites

- Requires baseline file at `.claude/transient/all_types_baseline.json`
- Python 3 must be installed
- The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`

## Notes

- The script examines the `type_kind` field within `path_info` of each mutation path
- A type is counted/listed if it contains **at least one** mutation path with the specified type_kind
- The summary shows unique type counts (each type counted once per type_kind, regardless of how many paths match)
- Type names are sorted alphabetically in the output
