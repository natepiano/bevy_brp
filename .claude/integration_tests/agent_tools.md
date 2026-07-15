# Agent Tool Catalog Tests

## Objective

Prove that the extras application exposes curated agent-tool metadata for an
existing BRP method, that the method succeeds and preserves handler errors through
`brp_execute`, and that an application without `BrpExtrasPlugin` has no catalog
endpoint.

## Runner-Managed App Context

The runner pre-launches both app instances and supplies one isolated port for each
label:

- **extras_app**: `extras_plugin`, with `bevy_brp_extras`
- **no_extras_app**: `no_extras_plugin`, with standard BRP only

Use `[extras_app port]` or `[no_extras_app port]` on every MCP call as directed.
Do not assume port 15702. Do not launch, stop, restart, reconnect, or otherwise
manage either app or the MCP host. This case requires no initial or final catalog
cleanup because the MCP owns no catalog state.

## Test Steps

### 1. Discover the complete BRP inventory

Call `mcp__brp__rpc_discover` on `[extras_app port]`.

- Require a successful response containing the complete BRP method inventory.
- Require `test/multiply` to be present among the many returned methods.
- Retain the complete discovered inventory for comparison with the curated catalog.

### 2. Read and validate the curated catalog

Call `mcp__brp__brp_list_agent_tools` on `[extras_app port]`. Retain the complete
top-level `result` payload as `first_catalog`.

- Require top-level `status: "success"`.
- Require exact
  `result.usage: "Pass an entry's method and matching params to brp_execute."`.
- Require `result.tools` to contain exactly one record with:
  - `name: "test_multiply"`
  - `method: "test/multiply"`
  - `description: "Multiply two signed integers with overflow checking"`
- Require the record's `params_schema` to have object type, exactly the required
  fields `value` and `factor`, and integer types for both properties.
- Require the record's `result_schema` to have object type, exactly the required
  fields `value`, `factor`, and `product`, and integer types for all three
  properties.
- Permit generated `$schema`, title, and other non-semantic schema metadata without
  requiring exact values.

### 3. Distinguish transport discovery from curated guidance

Compare the complete inventory from step 1 with `first_catalog`.

- Verify the catalog is not presented as the exhaustive `rpc.discover` document:
  the discovered inventory contains many BRP methods, while `result.tools` contains
  only the single developer-published `test_multiply` record.
- Verify the exact usage guidance directs the agent to invoke the record's method
  through `brp_execute`.
- Do not inspect the MCP tool list or search for a dynamic `test_multiply` MCP tool.
  The static registry test from Phase 3 owns proof that catalog records do not
  become native MCP tools.

### 4. Distinguish a missing endpoint from an empty catalog

Call `mcp__brp__brp_list_agent_tools` on `[no_extras_app port]`.

- Require top-level `status: "error"`.
- Require the error message to name `BrpExtrasPlugin`.
- Require error `metadata` to contain these exact values:
  - `stage: "catalog_request"`
  - `method: "brp_extras/agent_tools"`
  - `port: [no_extras_app port]`
  - `code: -32601`
- Require a successful top-level `result` to be absent. This is a missing catalog
  endpoint, not an installed empty catalog.

### 5. Execute the published method successfully

Call `mcp__brp__brp_execute` on `[extras_app port]` with:

```json
{
  "method": "test/multiply",
  "params": { "value": 6, "factor": 7 }
}
```

Require top-level `status: "success"` and exact
`result: { "value": 6, "factor": 7, "product": 42 }`.

### 6. Preserve the published method's overflow error

Call `mcp__brp__brp_execute` on `[extras_app port]` with JSON-safe overflow
operands:

```json
{
  "method": "test/multiply",
  "params": { "value": 4000000000, "factor": 4000000000 }
}
```

- Require top-level `status: "error"`.
- Require error `metadata` to contain these exact values:
  - `stage: "execution"`
  - `method: "test/multiply"`
  - `port: [extras_app port]`
  - `code: -32602`
  - `data: { "method": "test/multiply", "expected": { "value": "i64", "factor": "i64" } }`

### 7. Repeat the stateless catalog request

Call `mcp__brp__brp_list_agent_tools` again on `[extras_app port]` and retain its
complete top-level `result` payload as `second_catalog`.

- Require top-level `status: "success"`.
- Require `second_catalog` to be identical to `first_catalog`.
- Perform no sync, clear, reconnect, refresh, notification wait, or other mutable
  MCP-state operation between the two catalog calls.

## Expected Results

- `rpc.discover` reports the complete transport inventory, including
  `test/multiply`.
- `brp_list_agent_tools` returns one exact curated record with semantic parameter
  and result schemas plus `brp_execute` usage guidance.
- The application without `BrpExtrasPlugin` reports method-not-found metadata and
  no successful catalog result.
- `brp_execute` returns the exact typed success value and preserves the exact
  checked-overflow error metadata.
- Repeated catalog reads return identical results without MCP catalog state.

## Failure Criteria

Stop on the first mismatch in status, method inventory, catalog contents, semantic
schemas, missing-plugin metadata, execution result, overflow metadata, or repeated
catalog equality.
