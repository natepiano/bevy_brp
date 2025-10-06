# Get Type Kind

Analyzes type_kind values in mutation paths from the baseline file.

<ExecutionSteps>
**EXECUTE THESE STEPS IN ORDER:**

**STEP 1:** Execute <ArgumentProcessing/>
**STEP 2:** Execute <ScriptExecution/>
**STEP 3:** Display the script output to the user
</ExecutionSteps>

<ArgumentProcessing>
if $ARGUMENTS is empty: set MODE="summary"
if $ARGUMENTS contains one argument: set MODE="query" and TYPE_KIND="$1"
if $ARGUMENTS contains more than one argument: display error "Too many arguments" and usage, then exit
</ArgumentProcessing>

<ScriptExecution>
if MODE="summary": Use Bash tool to execute ".claude/scripts/get_type_kind.sh"
if MODE="query": Use Bash tool to execute ".claude/scripts/get_type_kind.sh "$TYPE_KIND""
Capture and display the script output
</ScriptExecution>

## Usage

### Summary Mode (no arguments)
Shows a count of how many top-level types contain at least one mutation path of each type_kind:

```bash
/get_kind_baseline
```

### Query Mode (with type_kind argument)
Shows all top-level type names that contain at least one mutation path with the specified type_kind.

Examples:
```bash
/get_kind_baseline List
/get_kind_baseline Struct
/get_kind_baseline Value
```

**Available type_kinds:** Array, Enum, List, Map, Set, Struct, Tuple, TupleStruct, Value

## Prerequisites

- Requires baseline file at `.claude/transient/all_types_baseline.json`
- Python 3 must be installed
- The baseline file must have the expected structure with `type_guide` array containing types with `mutation_paths`

## Notes

- The script examines the `type_kind` field within `path_info` of each mutation path
- A type is counted/listed if it contains **at least one** mutation path with the specified type_kind
- The summary shows unique type counts (each type counted once per type_kind, regardless of how many paths match)
- Type names are sorted alphabetically in the output
