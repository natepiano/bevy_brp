# Publishing BRP Agent Tools from Bevy Applications

## What it is

`bevy_brp_extras` lets a Bevy application publish curated descriptions and raw JSON Schemas for
selected existing BRP methods. `bevy_brp_mcp` exposes those records through the fixed, read-only
`brp_list_agent_tools` MCP tool; agents invoke a selected backing method through the existing
`brp_execute` tool.

The discovery and execution surfaces have distinct roles:

- `rpc_discover` exhaustively lists registered BRP method names.
- `brp_list_agent_tools` lists the developer-curated subset with descriptions and optional schemas.
- `brp_execute` invokes one exact BRP method with raw JSON parameters.

Catalog records are data, not dynamically registered MCP tools. Publishing a record does not
register its backing BRP handler, and most BRP methods need not be published for agents.

## How it works

### Registering and publishing a method

Applications perform two separate construction-time actions:

1. Register an instant handler in Bevy's `RemoteMethods`.
2. Publish agent-facing metadata for that method with `register_agent_tool`.

```rust
use bevy::prelude::*;
use bevy_brp_extras::{AgentTool, AppAgentToolExt, BrpExtrasPlugin};
use bevy_remote::{RemoteMethodSystemId, RemoteMethods};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, JsonSchema)]
struct MultiplyParams {
    value: i64,
    factor: i64,
}

#[derive(Serialize, JsonSchema)]
struct MultiplyResult {
    product: i64,
}

let mut app = App::new();
app.add_plugins(BrpExtrasPlugin);

let system_id = app.world_mut().register_system(multiply);
{
    let mut methods = app.world_mut().resource_mut::<RemoteMethods>();
    methods.insert(
        "example/multiply",
        RemoteMethodSystemId::Instant(system_id),
    );
}

app.register_agent_tool(
    AgentTool::new(
        "example_multiply",
        "example/multiply",
        "Multiply two signed integers",
    )
    .params_schema_for::<MultiplyParams>()
    .result_schema_for::<MultiplyResult>(),
);
```

The `RemoteMethods` borrow must end before calling `register_agent_tool`, which needs mutable access
to the `App`. Metadata registration may occur before or after adding `BrpExtrasPlugin`; both paths
initialize the same passive resource without replacing existing records.

The public extras API is deliberately limited to:

```rust
#[must_use]
pub struct AgentTool {
    // private fields
}

impl AgentTool {
    pub fn new(
        name: impl Into<String>,
        method: impl Into<String>,
        description: impl Into<String>,
    ) -> Self;

    pub fn params_schema(self, schema: schemars::Schema) -> Self;
    pub fn params_schema_for<T: schemars::JsonSchema>(self) -> Self;
    pub fn result_schema(self, schema: schemars::Schema) -> Self;
    pub fn result_schema_for<T: schemars::JsonSchema>(self) -> Self;
}

pub trait AppAgentToolExt {
    fn register_agent_tool(&mut self, agent_tool: AgentTool) -> &mut Self;
}
```

The generic setters run `schemars::schema_for!(T)` during application construction. `T` supplies
documentation only: it does not decode requests, encode responses, constrain the separately
registered handler, or create typed MCP support. The raw `Schema` setters support APIs that do not
warrant dedicated Rust request or response types.

`params_schema` describes the exact JSON-RPC `params` value forwarded to the method. It is not
wrapped as an MCP arguments object. Omitting it documents a parameterless method and tells callers
to omit `params`.

`result_schema` describes the exact BRP JSON-RPC `result` value. It is not wrapped in
`{ "result": ... }`. Omitting it leaves the result undocumented.

### The application-owned catalog

`BrpExtrasPlugin` registers `brp_extras/agent_tools` as an instant BRP method. It returns a versioned
document:

```json
{
  "version": 1,
  "tools": [
    {
      "name": "example_multiply",
      "method": "example/multiply",
      "description": "Multiply two signed integers",
      "params_schema": {
        "type": "object",
        "properties": {
          "value": { "type": "integer" },
          "factor": { "type": "integer" }
        },
        "required": ["value", "factor"]
      },
      "result_schema": {
        "type": "object",
        "properties": {
          "product": { "type": "integer" }
        },
        "required": ["product"]
      }
    }
  ]
}
```

Optional schemas are omitted rather than serialized as `null`. Raw object, array, primitive, and
boolean schemas are preserved without transformation.

For each request, the handler:

1. Reads the passive registered-metadata resource.
2. Sorts records by agent-facing `name`.
3. Verifies that every backing method exists in the live `RemoteMethods` resource and is instant.
4. Serializes the complete version-1 catalog.

Validation is all-or-error. One missing or watching method prevents a partial catalog from being
returned. The first rejected record in sorted name order supplies stable error data:

```json
{
  "name": "example_multiply",
  "method": "example/multiply",
  "reason": "backing_method_missing"
}
```

The other stable reason is `backing_method_watching`. Both failures use BRP internal-error code
`-32603`.

An installed plugin with no published records returns a valid empty version-1 catalog. An
application without `BrpExtrasPlugin` has no catalog endpoint and produces method-not-found code
`-32601`.

### MCP discovery and execution

`brp_list_agent_tools` is a fixed MCP tool whose only parameter is the optional BRP port:

```rust
pub struct ListAgentToolsParams {
    pub port: Port,
}
```

The port defaults to `15702`. Each call fetches `brp_extras/agent_tools` from the selected running
application and decodes version 1 defensively. The MCP crate does not depend on `bevy_brp_extras`; it
owns private wire-decoder types for the protocol.

Its public structured result is:

```json
{
  "result": {
    "usage": "Pass an entry's method and matching params to brp_execute.",
    "tools": [
      {
        "name": "example_multiply",
        "method": "example/multiply",
        "description": "Multiply two signed integers",
        "params_schema": { "type": "object" },
        "result_schema": { "type": "object" }
      }
    ]
  }
}
```

The normal agent workflow is:

```text
brp_list_agent_tools(port: 15702)
brp_execute(
    port: 15702,
    method: "example/multiply",
    params: { "value": 6, "factor": 7 }
)
```

`brp_execute` still performs fresh exact-name validation through `rpc_discover`, forwards the
supplied raw `params`, and preserves handler error codes and data. Direct HTTP JSON-RPC remains
useful for developer debugging outside MCP, but it is not a second MCP execution path.

Unsupported catalog versions, malformed documents, transport failures, and BRP errors identify the
catalog stage and port. A missing endpoint includes guidance to add `BrpExtrasPlugin`.

## Operational behavior

Registration creates one passive `RegisteredAgentTools(Vec<AgentTool>)` resource. The feature adds
no scheduled systems, polling, watchers, channels, caches, background synchronization,
notifications, or per-frame work.

The catalog handler is registered as a Bevy system only so BRP can invoke it on demand. Each request
performs sorting, live method lookups, and serialization. The MCP also fetches the application-owned
catalog afresh on every call and retains no catalog state.

There is no `RemoteCatalog`, replacement lifecycle, sync or clear operation, dynamic dispatcher,
`tools/list_changed` notification, or per-entry native MCP tool. Restarting an application with
different published metadata requires no MCP reconnect because the fixed tool reads the live
catalog. Installing a version of the MCP that first introduces `brp_list_agent_tools` still requires
the host to start that updated MCP binary so it can see the fixed tool schema.

## Implementation map

- `extras/src/agent_tools/registration.rs` — public builder and extension trait, passive resource,
  validation, rustdoc, and unit tests.
- `extras/src/agent_tools/catalog.rs` — version-1 serialization, deterministic ordering, live
  instant-method validation, and BRP errors.
- `extras/src/agent_tools/mod.rs` — public API and crate-private catalog boundary.
- `extras/src/plugin.rs` — resource initialization and instant catalog-method registration.
- `extras/src/constants.rs` — catalog version, method name, and stable error reasons.
- `extras/src/lib.rs` — public exports and crate-level workflow documentation.
- `extras/examples/agent_tool_registration.rs` — runnable typed `example/multiply` example.
- `extras/tests/agent_tool_registration.rs` — exported-API test that invokes the literal catalog
  endpoint.
- `mcp/src/brp_tools/tools/brp_list_agent_tools.rs` — fixed MCP handler, private decoder,
  structured result, and error mapping.
- `mcp/src/tool/name.rs` — static registration, port schema, read-only discovery annotation, and
  no-dynamic-tool assertions.
- `mcp/help_text/brp_list_agent_tools.txt` — curated discovery contract and execution handoff.
- `mcp/help_text/brp_execute.txt` — raw exact-method execution contract.
- `mcp/help_text/rpc_discover.txt` — exhaustive transport-discovery boundary.
- `test-app/examples/extras_plugin.rs` — typed `test/multiply` live fixture.
- `.claude/integration_tests/agent_tools.md` — two-application discovery, execution, error, and
  stateless-repeat coverage.

## Invariants

- `AgentTool` publishes metadata for an existing method; it never registers the handler.
- Published records may reference only instant BRP methods.
- The downstream feature API remains `AgentTool` plus `AppAgentToolExt`.
- Agent-facing names are unique and contain 1–128 ASCII letters, digits, periods, underscores, or
  hyphens.
- Method and description strings must be nonempty after trimming.
- Multiple agent-facing records may document the same backing method.
- Construction-time validation uses panics with the rejected name, field, and reason.
- Runtime metadata registration after `App::run` begins is unsupported.
- Schemas describe raw JSON-RPC values and are never compiled or validated by MCP.
- Catalog output is versioned, sorted by agent-facing name, and all-or-error.
- Every request validates against the current `RemoteMethods`.
- `rpc_discover` remains exhaustive; `brp_list_agent_tools` remains curated.
- Catalog records never become native MCP tools.
- `brp_execute` remains the supported MCP invocation path for published records.
- MCP keeps no mutable catalog state and fetches the application-owned catalog on every call.
- Shared result-placement machinery, static dispatch, and `brp_execute` behavior remain unchanged.

## Testing evidence

The extras unit tests cover owned values, raw and generated schemas, omitted schemas, name
boundaries and invalid characters, empty fields, duplicate names, allowed duplicate backing
methods, plugin ordering, deterministic catalog ordering, raw schema preservation, empty catalogs,
and exact missing/watching errors.

The public integration test constructs the feature only through exported APIs, invokes the literal
`brp_extras/agent_tools` method through `RemoteMethods`, and verifies version 1 plus generated
parameter and result schemas without exposing catalog internals.

The MCP unit tests cover the fixed registry entry, read-only discovery annotation, port-only
parameter schema, absence of per-record native tools, required wire fields, empty and populated
results, omitted schemas, raw schema preservation, version and decode failures, method-not-found
guidance, BRP error preservation, the exact structured result, and absence of MCP catalog state.

The repository integration case launches one app with extras and one without it. It verifies
exhaustive discovery, curated discovery, missing-plugin behavior, successful multiplication,
checked-overflow propagation, and identical repeated stateless catalog reads. Verification also
covered the workspace nextest suite, release example build, doctests, clippy, nightly formatting,
and configuration checks.

## Calibration and gotchas

- Adding metadata does not make a method callable. Insert the backing handler into `RemoteMethods`
  separately.
- End the mutable `RemoteMethods` guard before calling `register_agent_tool`.
- Registration order relative to `BrpExtrasPlugin` is flexible, but catalog reads require the
  plugin endpoint and a valid live instant backing method.
- One invalid record blocks the complete catalog. There is intentionally no partial-success
  response.
- An absent endpoint and an installed empty catalog are different states.
- Schema-derived types are documentation helpers only. Handler request and response types may
  differ, so developers must keep metadata accurate.
- Raw schemas allow APIs whose JSON shapes do not justify dedicated Rust types.
- Omitting `params_schema` means callers should omit `params`; it does not mean "any parameters."
- Catalog entry names are agent-facing identifiers. Execution always uses the record's exact
  `method`.
- The runnable headless example installs an example-local schedule runner so it remains available
  for remote calls; this is not library runtime machinery.
- A newly added fixed MCP tool requires installation and host reload once. Changes to application
  catalog contents do not.
- `brp_list_agent_tools` is read-only, but a selected method executed through `brp_execute` may have
  arbitrary effects.

## Why it is this way

The API separates callable behavior from documentation. Bevy's `RemoteMethods` remains the authority
for execution, while `AgentTool` adds only the information Bevy 0.19 discovery lacks. Generic schema
builders make common typed payloads concise, and raw-schema setters avoid requiring a Rust type for
every possible JSON API.

A passive application-owned catalog avoids synchronization and lifetime problems between Bevy and
MCP. Request-time validation permits flexible construction order while ensuring agents never
receive stale guidance for a missing or watching method. All-or-error behavior prevents a
superficially successful catalog from hiding application misconfiguration.

Keeping `brp_list_agent_tools` fixed and treating records as data avoids host-dependent dynamic-tool
refresh behavior and mutable MCP dispatch state. Reusing `brp_execute` preserves exact-name
discovery, transport handling, timeouts, and BRP error propagation in one execution path.
