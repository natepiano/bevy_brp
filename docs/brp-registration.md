# Publishing BRP agent tools from Bevy applications

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Build a passive developer-published catalog that agents inspect with `brp_list_agent_tools` and invoke through the existing `brp_execute` tool.

## Delegation Context

- **Project:** `bevy_brp` workspace: `bevy_brp_extras` 0.23.0-dev lets Bevy developers publish curated agent-facing metadata for existing BRP methods; `bevy_brp_mcp` 0.23.0-dev exposes the fixed `brp_list_agent_tools` discovery tool and retains `brp_execute` as the invocation path; unpublished `bevy_brp_test_apps` supplies the end-to-end fixture.
- **Stack:** Rust 2024; Bevy and `bevy_remote` 0.19.0; `rmcp` 2.2.0; `schemars` 1.2.1; `serde` and `serde_json`; existing `reqwest` BRP client.
- **Layout:** `extras/{src,examples,tests,README.md,CHANGELOG.md,Cargo.toml}` — passive registration API, catalog BRP endpoint, example, tests, and developer guidance; `mcp/{src,help_text,README.md,CHANGELOG.md}` — one fixed catalog-listing MCP tool and cross-linked discovery/invocation guidance; `test-app/{Cargo.toml,examples/extras_plugin.rs}` — `test/multiply` fixture; `.claude/{integration_tests,config,agents,commands}` — real agent integration coverage; `docs/brp-registration.md` — authoritative phased plan.
- **Key files:** `docs/brp-registration.md` — approved architecture and delegate-ready plan; `Cargo.toml` — workspace dependency/version source; `CLAUDE.md` — installed-MCP reload requirement after tool changes; `extras/Cargo.toml` — add `schemars` as a normal dependency; `extras/src/lib.rs` — module declaration, public `AgentTool`/`AppAgentToolExt` exports, and crate rustdoc; `extras/src/constants.rs` — `brp_extras/agent_tools` method and catalog-version constants; `extras/src/plugin.rs` — order-independent resource initialization and instant catalog-method registration; `extras/src/agent_tools/mod.rs` — feature module boundary and crate-private wire exports; `extras/src/agent_tools/registration.rs` — owned builder, extension trait, passive resource, construction-time validation, rustdoc, and unit tests; `extras/src/agent_tools/catalog.rs` — deterministic versioned catalog encoding, live instant-method checks, handler, and unit tests; `extras/examples/agent_tool_registration.rs` — runnable typed-schema registration example; `extras/tests/agent_tool_registration.rs` — focused public-API/example support tests; `extras/README.md` — concise developer workflow; `extras/CHANGELOG.md` — public API/catalog entry; `mcp/src/brp_tools/tools/brp_list_agent_tools.rs` — fixed port-parameterized catalog reader and typed result; `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — closest bespoke local-tool registration/result pattern; `mcp/src/brp_tools/tools/mod.rs` — tool exports; `mcp/src/brp_tools/mod.rs` — facade exports; `mcp/src/tool/name.rs` — static `ToolName` registration, parameters, handler, and discovery annotation; `mcp/src/tool/registry.rs` — read-only static registry assembly reference; `mcp/src/mcp_service.rs` — read-only immutable built-in listing/dispatch reference; `mcp/src/brp_tools/brp_client/client.rs` — existing BRP request path; `mcp/src/brp_tools/tools/brp_execute.rs` — existing exact-name discovery and invocation behavior that must remain unchanged; `mcp/src/brp_tools/tools/rpc_discover.rs` — existing transport-level method discovery boundary; `mcp/help_text/brp_list_agent_tools.txt` — catalog purpose and `brp_execute` handoff; `mcp/help_text/brp_execute.txt` — catalog cross-reference; `mcp/help_text/rpc_discover.txt` — distinction between all BRP methods and curated agent tools; `mcp/README.md` — list-then-execute workflow; `mcp/CHANGELOG.md` — fixed discovery-tool entry; `test-app/Cargo.toml` — schema derive dependency for the fixture; `test-app/examples/extras_plugin.rs` — publish metadata for existing `test/multiply`; `test-app/examples/no_extras_plugin.rs` — live missing-catalog integration fixture; `.claude/integration_tests/agent_tools.md` — catalog distinction, schema, successful execution, and BRP error coverage; `.claude/config/integration_tests.json` — register the `agent_tools` case; `.claude/agents/integration-tester.md` — authorize `brp_list_agent_tools` and retain `brp_execute`; `.claude/commands/integration_tests.md` — read-only integration-runner contract; `.cargo/config.toml` — workspace `mcp-debug` cfg; `.github/workflows/ci.yml` — canonical build/test/format commands.
- **Build:** `cargo build --release --all-features --workspace --examples`
- **Test:** `cargo nextest run --all-features --workspace --tests`; final repository integration phase: `/integration_tests agent_tools`.
- **Lint:** Full `clippy` skill, dispatched with `auto-proceed`; direct formatting, if needed, is `cargo +nightly fmt --all`, never plain `cargo fmt`.
- **Style:** `zsh ~/.claude/scripts/rust_style/load-rust-style.sh --project-root /Users/natemccoy/rust/bevy_brp`
- **Invariants:** Public feature surface is exactly owned `AgentTool` plus `AppAgentToolExt`; publishing metadata is separate from registering the backing BRP handler, and schema-derived Rust types generate documentation only—they never decode requests, encode responses, or require typed support in MCP. The passive version-1 `brp_extras/agent_tools` catalog is a curated subset of `RemoteMethods`, sorted deterministically, initialized regardless of plugin/registration order, and read only when requested; every advertised backing method must exist and be instant. Add no systems, polling, watchers, channels, background synchronization, or per-frame work. Catalog schemas describe the raw JSON-RPC `params` and raw BRP `result` values consumed and returned by the backing method; preserve raw JSON Schema support and perform no MCP-side compilation or runtime validation. `brp_list_agent_tools` is an ordinary immutable built-in MCP tool that fetches the live catalog on every call and returns `name`, `description`, `method`, `params_schema`, and `result_schema`; it creates no native per-entry tools, mutable catalog state, overlay, sync/clear lifecycle, `tools/list_changed`, dispatcher, or alternate execution path. `rpc.discover` continues to list every registered BRP method; `brp_list_agent_tools` lists only developer-published agent guidance; `brp_execute` remains the sole supported MCP invocation path and retains its live exact-method validation, raw parameter forwarding, and BRP error preservation. Documentation and tool results must explicitly teach `brp_list_agent_tools` → `brp_execute`, while noting that direct JSON-RPC HTTP remains possible outside MCP. Completion requires full public rustdoc, the runnable example and support test, concise README/help text, both changelogs, and an integration case proving the catalog/raw schemas/backing method plus successful and failing `brp_execute` calls.

## Phases

### Phase 1 — Add the typed agent-tool publication API  · status: done (`42539f3e`)

#### Work Order

**Goal:** Bevy applications can publish documentation for selected BRP methods through two public, construction-time types without adding runtime work.

**Spec:**

Add `schemars.workspace = true` as a normal `bevy_brp_extras` dependency. Create an owned `AgentTool` builder with private fields and this public surface:

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

    #[must_use]
    pub fn params_schema(self, schema: schemars::Schema) -> Self;

    #[must_use]
    pub fn params_schema_for<T: schemars::JsonSchema>(self) -> Self;

    #[must_use]
    pub fn result_schema(self, schema: schemars::Schema) -> Self;

    #[must_use]
    pub fn result_schema_for<T: schemars::JsonSchema>(self) -> Self;
}

pub trait AppAgentToolExt {
    fn register_agent_tool(&mut self, agent_tool: AgentTool) -> &mut Self;
}
```

`name` is the stable agent-facing identifier. `method` is the exact backing BRP method passed later to `brp_execute`. `description` is required so every published entry explains its purpose. `params_schema` describes the raw JSON-RPC `params` value forwarded by `brp_execute`; omitting it documents a parameterless method and tells the agent to omit `params`. `result_schema` describes the raw BRP JSON-RPC `result` value; omitting it leaves the result undocumented. Do not wrap either schema in an MCP arguments object or `{ "result": ... }`, and do not introduce input encodings.

The generic schema methods call `schemars::schema_for!(T)` during application construction. `T` generates metadata only and has no type relationship with the separately registered BRP handler. Raw `schemars::Schema` setters remain available so developers are not required to create a Rust type for every BRP payload.

Create private `RegisteredAgentTools(Vec<AgentTool>)` as a passive Bevy resource. Implement `AppAgentToolExt` for `App`; the trait is a downstream-facing extension point and its rustdoc must say so. `register_agent_tool` calls `init_resource::<RegisteredAgentTools>()`, validates, and appends immediately. This must work before or after `BrpExtrasPlugin` is added. Runtime registration after `App::run` begins is unsupported.

Validate the complete agent name as 1–128 ASCII letters, digits, periods, underscores, or hyphens; implement that predicate directly without adding a regular-expression dependency. Reject empty method and description strings after trimming and reject duplicate agent names. The infallible chaining API panics during application construction with the rejected name, field, and reason. Do not reject duplicate backing methods because one BRP method may be documented for more than one agent workflow. Backing method existence and instant/watching classification are deferred until the catalog is requested, after all plugins have registered their methods.

Write complete rustdoc with the API. `AgentTool` must state that it publishes metadata and never creates a native MCP tool. Each schema method must name the raw JSON-RPC value it describes. `AppAgentToolExt` must distinguish BRP handler registration from agent metadata publication. `register_agent_tool` requires a `# Panics` section. Public exports for this feature remain exactly `AgentTool` and `AppAgentToolExt`; wire/resource types remain private.

**Files:**
- `extras/Cargo.toml` — promote/add `schemars` as a normal dependency.
- `extras/src/agent_tools/mod.rs` — feature module boundary and exports.
- `extras/src/agent_tools/registration.rs` — `AgentTool`, `RegisteredAgentTools`, `AppAgentToolExt`, validation, rustdoc, and unit tests.
- `extras/src/lib.rs` — declare the module and re-export only `AgentTool` and `AppAgentToolExt`.

**Constraints from prior phases:** None.

**Acceptance gate:** Unit tests cover owned string inputs, raw and generated parameter/result schemas, omitted schemas, name length boundaries and invalid characters, empty method/description rejection, duplicate-name panic diagnostics, allowed duplicate backing methods, and resource initialization before/after plugin addition. Public docs state the metadata-only and raw-JSON semantics. The public feature surface contains exactly `AgentTool` and `AppAgentToolExt`. Build and Test from Delegation Context are green; the full `clippy` skill is green.

#### Retrospective

**What worked:**

- `AgentTool`, `AppAgentToolExt`, and the passive registry landed with the planned owned/raw-schema semantics and downstream API boundary.
- Unit coverage exercised every validation, schema, duplicate, and plugin-order case; the blind implementation review found no Phase 1 defect.

**What deviated from the plan:**

- `Cargo.lock` changed because `schemars` became a direct `bevy_brp_extras` dependency.

**Surprises:**

- Phase 2's sibling catalog cannot consume the registration module's private fields until that phase deliberately widens their restricted internal visibility.

**Implications for remaining phases:**

- Phase 2 now owns the minimum `pub(super)`/crate-private visibility change needed by `catalog.rs` and `plugin.rs`, while the downstream API remains exactly two exported types.

#### Phase 1 Review

- Phase 2 now specifies a `pub(crate)` resource type, `pub(super)` entries/metadata access, and no downstream visibility increase.
- Phase 2 now fixes missing/watching catalog failures at `INTERNAL_ERROR` with stable `name`, `method`, and `reason` data.
- Phases 3 and 5 now use the actual `rpc_discover` MCP tool name while retaining `rpc.discover` for the BRP method.
- Phase 3 now follows the existing bespoke local-tool registration pattern and re-exports only its handler and parameter types.
- Phase 3 unit tests now inject `ResponseStatus`; Phase 5 owns real missing-endpoint and repeated-fetch coverage with `no_extras_plugin`.
- Phase 4 now augments the public rustdoc and help created earlier instead of recreating them.
- Phase 4 now fixes plugin-before-custom-method ordering in the runnable example.
- Phase 4 now runs a targeted rustdoc-test command because nextest does not execute doctests.
- Phase 5 now adds the direct `schemars` dependency, exact derives/imports, and a typed `test/multiply` result.
- Phase 5 now requires installing the updated MCP and reloading the host before its two-app agent integration test.

### Phase 2 — Publish the passive agent-tool catalog  · status: done (`7ea098b5`)

#### Work Order

**Goal:** `brp_extras/agent_tools` returns a deterministic version-1 catalog of developer-published instant BRP methods without running systems between requests.

**Spec:**

Add method/version constants and serialize this wire document:

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
        "properties": { "product": { "type": "integer" } },
        "required": ["product"]
      }
    }
  ]
}
```

`params_schema` and `result_schema` are optional and omitted when not supplied. They remain the raw schemas from Phase 1; the endpoint performs no MCP transformation and compiles no runtime validator.

Register `brp_extras/agent_tools` as an instant BRP method in `BrpExtrasPlugin`. Both `BrpExtrasPlugin::build` and `register_agent_tool` use `init_resource::<RegisteredAgentTools>()`; neither replaces registrations created by the other. The handler reads the passive resource only when called and returns tools sorted by agent name. An empty registry returns `{ "version": 1, "tools": [] }`.

At this phase boundary, widen only the crate-internal visibility needed by the new catalog module. Declare the resource as `pub(crate) struct RegisteredAgentTools(pub(super) Vec<AgentTool>)` so `agent_tools/mod.rs` can re-export its type to `plugin.rs` without exposing its entries there. Give `catalog.rs` access to each `AgentTool` field through `pub(super)` fields or equivalent `pub(super)` accessors. Re-export `RegisteredAgentTools` from `agent_tools` with `pub(crate) use`. These changes must not add a downstream export or make any field accessible outside the crate.

Before serializing a nonempty catalog, inspect the live `RemoteMethods` resource. Every advertised `method` must exist and be `RemoteMethodSystemId::Instant`. Reject missing and `Watching` methods as application-configuration failures using `bevy_remote::error_codes::INTERNAL_ERROR` (`-32603`). Both errors carry data with exact fields `name`, `method`, and `reason`; use stable reason values `backing_method_missing` and `backing_method_watching`, respectively. Their deterministic messages also name the agent entry and backing method. This request-time check permits any construction order while preventing the catalog from advertising methods that `brp_execute` cannot invoke as an instant call.

Add no update systems, polling, watchers, channels, background synchronization, retries, caches, or automatic refresh. The catalog response is the current application state at the time of the request.

**Files:**
- `extras/src/constants.rs` — add the `brp_extras/agent_tools` method and version constants.
- `extras/src/agent_tools/mod.rs` — expose crate-private catalog items to the plugin.
- `extras/src/agent_tools/registration.rs` — widen only sibling/plugin visibility needed by the catalog while preserving the two-type downstream API.
- `extras/src/agent_tools/catalog.rs` — wire types, deterministic serialization, live instant-method checks, BRP errors, and unit tests.
- `extras/src/plugin.rs` — initialize the resource without replacement and register the instant catalog method.
- `extras/src/lib.rs` — reference the catalog endpoint in crate rustdoc without exporting wire types.

**Constraints from prior phases:** Phase 1 provides owned `AgentTool`, passive `RegisteredAgentTools`, exact validation rules, raw optional schemas, and order-independent resource initialization. Those implementation details remain private inside `registration.rs` until this phase gives the sibling catalog and plugin the minimum restricted visibility they need. Preserve all Phase 1 semantics and downstream privacy; the catalog adds only request-time backing-method validation and serialization.

**Acceptance gate:** Tests prove empty and populated catalogs, exact version/field names, omitted optional schemas, deterministic agent-name order, raw object/array/primitive/boolean schemas without rewriting, registration before and after plugin addition, live instant acceptance, and exact missing/watching `INTERNAL_ERROR` messages plus `name`/`method`/`reason` data. Tests and code inspection also prove absence of any added scheduled system or per-frame work; the on-demand catalog handler is the only new registered system. `AgentTool` fields and `RegisteredAgentTools` entries remain inaccessible to downstream crates, and the only downstream feature exports remain `AgentTool` and `AppAgentToolExt`. Build and Test from Delegation Context are green; the full `clippy` skill is green.

#### Retrospective

**What worked:**

- `brp_extras/agent_tools` serializes the exact version-1 document from borrowed metadata, preserving raw schemas without cloning or transformation.
- Request-time checks accept instant methods, reject missing/watching methods with the planned stable errors, and add no scheduled or per-frame work.
- Both implementation reviews found no defect; the full workspace gate and focused 70-test extras suite passed.

**Implications for remaining phases:**

- Phase 3 must decode the exact `version`/`tools` document and preserve the catalog's optional raw schemas and BRP error details.
- Phase 4 can exercise the catalog handler through `RemoteMethods` without exposing its private wire/resource types.

#### Review

- Phase 3 now specifies an exact private version-1 decoder, required catalog fields, omitted optional schemas, and preservation of catalog order and raw schema forms.
- Phase 3 error tests use `ResponseStatus` rather than exporting `BrpClientError`, and cover the shipped method-not-found plus missing/watching backing-method errors.
- Phase 4's public integration test invokes the literal catalog endpoint through `RemoteMethods` and `RemoteMethodSystemId::Instant`, without exposing private catalog internals.
- Phase 4 documents the all-or-error catalog contract and requires the `RemoteMethods` mutable guard to end before agent metadata registration.
- The static-registry proof that catalog entries do not become native MCP tools belongs to Phase 3; Phase 5 verifies the returned list-to-execute guidance against the live application.
- The remaining phase order is still valid: MCP behavior, documentation/example coverage, then live repository integration.

### Phase 3 — Add fixed agent-tool discovery to MCP  · status: todo

#### Work Order

**Goal:** Agents can fetch the running application's curated catalog through one ordinary built-in MCP tool and then invoke a selected backing method with `brp_execute`.

**Spec:**

Add the fixed `brp_list_agent_tools` tool through the existing `ToolName`/`ToolFn` registry, following the bespoke local-tool pattern in `world_find_entities_by_name.rs` rather than a `#[brp_tool]`-generated direct method wrapper. Add an unannotated `BrpListAgentTools` enum variant, give it a read-only `Discovery` annotation, and add explicit parameter-builder and handler match arms. It accepts only the standard BRP port:

```rust
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListAgentToolsParams {
    #[serde(default)]
    pub port: Port,
}
```

The handler calls `brp_extras/agent_tools` at that port through the existing arbitrary application-method `BrpClient` path. Factor interpretation of the returned `ResponseStatus` into a pure helper so unit tests can inject successful, malformed, and BRP-error responses without a network mock. Define private `AgentToolCatalogWire { version: u32, tools: Vec<AgentToolWire> }` and `AgentToolWire` decoder structs. Every entry requires string `name`, `method`, and `description` fields and has optional `Option<serde_json::Value>` `params_schema`/`result_schema` fields; do not add serde defaults to the required document fields. Require `version == 1`, preserve catalog order, and preserve every JSON schema form without rewriting. These are MCP-local types; `bevy_brp_mcp` must not depend on `bevy_brp_extras`. Reject unsupported catalog versions and malformed documents with an error that identifies the catalog stage and port. Preserve BRP connection, JSON-RPC code/data, and method-not-found details through existing error conventions. If the endpoint is absent, tell the agent that the running application must add `BrpExtrasPlugin`; an installed plugin with no published entries instead returns a valid empty catalog.

Use `BrpListAgentTools` for the handler, `ListAgentToolsParams` for its public parameter type, `ListAgentToolsResult` for its result type, and `ListedAgentTool` for each serialized result entry. Re-export only `BrpListAgentTools` and `ListAgentToolsParams` through `tools/mod.rs` and the `brp_tools` facade because static registration names only those types. Declare `ListAgentToolsResult` and `ListedAgentTool` as `pub` inside the private `brp_list_agent_tools` module only as required by the public `ToolFn` associated type and result fields; do not re-export them. `ToolDef` continues to advertise the generic `ToolCallJsonResponse` output schema, so do not add per-tool output-schema machinery. Keep the versioned BRP response decoder private. `ListedAgentTool` carries public `name`, `method`, `description`, and optional `serde_json::Value` parameter/result schema fields; annotate both optional schema fields with `#[serde(skip_serializing_if = "Option::is_none")]` so omitted catalog schemas remain absent rather than becoming `null`. `ListAgentToolsResult` carries `tools`, the fixed `usage` string below, and the existing message-template/metadata fields needed by `ResultStruct`; mark both `tools` and `usage` with `#[to_result]` so the catalog records and instruction appear in the structured result rather than nesting the raw BRP response.

Return one typed structured result plus serialized text using the existing result conventions:

```json
{
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
```

Each invocation fetches the live catalog. Add no `RemoteCatalog`, cache, service lock, sync/clear tools, diff, runtime schema compiler, `tools/list_changed` capability, native per-entry `Tool`, dispatcher, or alternate call path. Do not change `McpService` listing/dispatch behavior or `brp_execute` execution behavior. `brp_execute` continues to perform live exact-name `rpc.discover` validation immediately before sending the raw `params` value.

Write mutually cross-linked agent help:

- `brp_list_agent_tools`: lists only methods deliberately documented for agents, returns raw parameter/result schemas and a backing method, and hands execution to `brp_execute`; entries are catalog records, not native MCP tools.
- `rpc_discover`: lists every registered BRP method at the transport level; application methods may supply only names, so use `brp_list_agent_tools` for curated descriptions and schemas.
- `brp_execute`: invokes a method by exact name with raw `params`; use `brp_list_agent_tools` first when the application publishes agent guidance.

The help and result must distinguish the MCP workflow from developer debugging: direct HTTP JSON-RPC to `http://127.0.0.1:<port>/jsonrpc` remains valid outside MCP, but agents should use `brp_execute` for MCP-native invocation, timeout/error handling, and method validation.

**Files:**
- `mcp/src/brp_tools/tools/brp_list_agent_tools.rs` — parameters, private wire decoding, typed result, BRP request, errors, and unit tests.
- `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — read-only model for a bespoke local tool's result and `ToolFn` implementation.
- `mcp/src/brp_tools/tools/mod.rs` — export the handler types.
- `mcp/src/brp_tools/mod.rs` — facade exports required by static registration.
- `mcp/src/tool/name.rs` — add the ordinary static `BrpListAgentTools` variant, parameters, annotations, and handler.
- `mcp/src/tool/registry.rs` — read-only registry construction reference; do not add a parallel registry.
- `mcp/src/mcp_service.rs` — read-only immutable listing/dispatch reference; do not add mutable catalog state.
- `mcp/src/brp_tools/brp_client/client.rs` — reuse the existing arbitrary application-method request path.
- `mcp/src/brp_tools/tools/brp_execute.rs` — preserve execution behavior; add only documentation cross-reference if owned here.
- `mcp/src/brp_tools/tools/rpc_discover.rs` — preserve transport discovery behavior; add only documentation cross-reference if owned here.
- `mcp/help_text/brp_list_agent_tools.txt` — concise catalog/list-to-execute contract.
- `mcp/help_text/brp_execute.txt` — agent-catalog cross-reference.
- `mcp/help_text/rpc_discover.txt` — exhaustive-method versus curated-agent-catalog distinction.

**Constraints from prior phases:** Phase 2 supplies the exact version-1 document and guarantees each returned method is a live instant `RemoteMethod`. The MCP still decodes defensively and supports no other version. Phase 1 schemas describe raw `brp_execute.params` and BRP results; never reinterpret them as native MCP per-entry schemas.

**Acceptance gate:** Unit tests cover the static registry containing `brp_list_agent_tools` and explicitly not containing catalog entry names such as `test_multiply`; its read-only `Discovery` annotation; default and explicit ports; required wire fields; empty and populated decoding; exact preservation of order, names, methods, descriptions, omitted optional fields, and object/array/primitive/boolean raw schemas; unsupported version/malformed response; exact `#[to_result]` usage guidance; and the absence of MCP service/catalog state. Construct BRP error statuses in tests by deserializing `ResponseStatus` JSON or through another test-private factory—do not export `BrpClientError`. Cover method-not-found plus both shipped `-32603` errors and their exact `name`/`method`/`reason` data (`backing_method_missing`, `backing_method_watching`). Do not add a network-mock dependency in this phase. Help-text tests prove all three cross-references and do not imply native dynamic tools. Existing `rpc_discover`, `brp_execute`, static listing, and built-in dispatch tests remain green. Phase 5 owns live missing-endpoint and repeated-fetch coverage against real applications. Build and Test from Delegation Context are green; the full `clippy` skill is green.

### Phase 4 — Ship developer and agent documentation  · status: todo

#### Work Order

**Goal:** Developers can publish one typed agent catalog entry and agents can discover and invoke it from the documented workflow without reading implementation source.

**Spec:**

Create `extras/examples/agent_tool_registration.rs` as a complete runnable example. It defines `MultiplyParams: Deserialize + JsonSchema` and `MultiplyResult: Serialize + JsonSchema`. Build the app with `DefaultPlugins`, add `BrpExtrasPlugin` first so `RemoteMethods` exists, then register the instant `example/multiply` system and insert it into `RemoteMethods`. Scope or explicitly drop the mutable `RemoteMethods` guard before calling `app.register_agent_tool`, which needs mutable access to `App`; then publish the metadata and run the app. A custom method-registration plugin added after `BrpExtrasPlugin`, matching the test app's ordering pattern, is also acceptable. Publish:

```rust
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

The example and docs show this exact sequence:

```text
cargo run -p bevy_brp_extras --example agent_tool_registration
brp_list_agent_tools(port: 15702)
brp_execute(
    port: 15702,
    method: "example/multiply",
    params: { "value": 6, "factor": 7 }
)
```

Explain the two separate developer actions under a `BRP methods and agent tools` heading: registering a remote method makes it callable and visible through `rpc.discover`; calling `register_agent_tool` publishes the description and raw schemas that teach an agent how to call that existing method. It does not create a native MCP tool. State the subset relation: every published agent entry names a BRP method, while most BRP methods need not be agent tools.

Phase 1 already supplied complete public API rustdoc. Augment and cross-link it only where the shipped catalog endpoint now adds request-time live instant validation, plugin requirements, or runnable-example links, then perform a final consistency audit of `AgentTool`, every builder method, `AppAgentToolExt`, and `register_agent_tool`. Preserve the existing coverage of owned values, name validation, required description, raw schema semantics, compile-time-only `JsonSchema` types, parameterless methods, optional result documentation, construction-time panics, and unsupported runtime registration. State the catalog's all-or-error contract: if any published backing method is missing or watching, the request returns no partial catalog and identifies the rejected entry through stable `name`, `method`, and `reason` error data. Private resource/wire types document their invariants without becoming public.

Add a focused public-API/example support test that constructs an app through only exported APIs and publishes the typed example entry. Retrieve the literal `"brp_extras/agent_tools"` from the public `RemoteMethods` resource, require `RemoteMethodSystemId::Instant`, invoke it with `world.run_system_with(system_id, None)`, decode the returned `serde_json::Value`, and assert the registered name/method/description plus generated raw parameter/result schemas. This deliberately proves the test needs no public catalog constant, handler, resource, or wire type. Update `extras/README.md` and `mcp/README.md` with the same concise list-then-execute workflow and document the catalog's all-or-error validation behavior. The MCP README may include the curl-equivalent JSON-RPC request only as a developer debugging alternative; it must recommend `brp_execute` to agents. Phase 3 already created the three required MCP help files; update them here only for endpoint facts learned from the completed extras implementation and run a final cross-file consistency audit.

Add public feature entries to `extras/CHANGELOG.md` and `mcp/CHANGELOG.md`. Do not describe synchronization, replacement, clearing, notifications, dynamic native tools, MCP-side schema validation, or a second executor.

**Files:**
- `extras/examples/agent_tool_registration.rs` — runnable common-case example.
- `extras/tests/agent_tool_registration.rs` — public API and generated-schema assertions.
- `extras/src/lib.rs` — crate-level workflow and links.
- `extras/src/agent_tools/mod.rs`, `extras/src/agent_tools/registration.rs`, and `extras/src/agent_tools/catalog.rs` — complete/cross-link rustdoc and private invariant comments.
- `extras/README.md` — `BRP methods and agent tools` developer workflow.
- `extras/CHANGELOG.md` — public `AgentTool`, extension trait, and catalog endpoint entry.
- `mcp/README.md` — `brp_list_agent_tools` → `brp_execute` agent workflow and optional developer curl example.
- `mcp/help_text/brp_list_agent_tools.txt`, `mcp/help_text/brp_execute.txt`, and `mcp/help_text/rpc_discover.txt` — final concise agent contracts.
- `mcp/CHANGELOG.md` — fixed discovery tool and documented handoff entry.

**Constraints from prior phases:** Use the exact Phase 1 public API and raw schema meanings, Phase 2 endpoint/name/version/all-or-error behavior, and Phase 3 fixed MCP tool/result/help vocabulary. Phase 1 public rustdoc and Phase 3 help files are existing inputs to audit and augment, not documentation to recreate. `RemoteMethods` is available only after `RemotePlugin`/`BrpExtrasPlugin` is added, and its mutable guard must end before agent metadata registration, so the example must preserve both ordering and borrow scope. Documentation must not restore any removed dynamic-registration concept.

**Acceptance gate:** The example builds and its support test passes; `cargo test --doc -p bevy_brp_extras --all-features` compiles the public rustdoc examples in addition to the workspace nextest gate; all snippets use exact shipped names and fields; READMEs and help text distinguish exhaustive `rpc.discover` from curated `brp_list_agent_tools`; both teach `brp_execute` as the MCP invocation path; curl is labeled developer-only; both changelogs identify their public additions; no documentation claims that per-entry native tools appear. Build and Test from Delegation Context are green; the full `clippy` skill, including documentation checks, is green.

### Phase 5 — Add repository integration coverage  · status: todo

#### Work Order

**Goal:** The real extras test application and integration agent prove curated discovery followed by successful and failing `brp_execute` invocation.

**Spec:**

Add `schemars.workspace = true` unconditionally to `test-app/Cargo.toml`. In `test-app/examples/extras_plugin.rs`, import `schemars::JsonSchema`, `serde::Serialize`, and `bevy_brp_extras::{AgentTool, AppAgentToolExt}` alongside the existing deserialization imports. Extend `ParameterizedBrpPlugin` while keeping the existing instant `test/multiply` method and checked-multiplication error behavior. Derive `Deserialize + JsonSchema` for `MultiplyParams`, define `MultiplyResult` with `value`, `factor`, and `product` fields and derive `Serialize + JsonSchema`, serialize that result from the handler, then publish after inserting the backing method:

```rust
app.register_agent_tool(
    AgentTool::new(
        "test_multiply",
        "test/multiply",
        "Multiply two signed integers with overflow checking",
    )
    .params_schema_for::<MultiplyParams>()
    .result_schema_for::<MultiplyResult>(),
);
```

The mutable `RemoteMethods` resource guard used to insert `test/multiply` must be scoped or explicitly dropped before `app.register_agent_tool`, which needs mutable access to `App`.

Create `.claude/integration_tests/agent_tools.md` and register it as a two-application case with labels `extras_app` (`extras_plugin`) and `no_extras_app` (`no_extras_plugin`). The runner assigns dynamic ports; every step must use the supplied port for its application rather than assuming 15702. Update the integration-tester agent permissions for the fixed `mcp__brp__brp_list_agent_tools` tool and retain its existing authorization for `mcp__brp__brp_execute` and `mcp__brp__rpc_discover`. The integration case must:

1. Call `rpc_discover` on `[extras_app port]` and verify that `test/multiply` is one member of the complete BRP method inventory.
2. Call `brp_list_agent_tools` on `[extras_app port]` and verify `test_multiply`, its backing `method`, description, raw parameter schema, raw result schema, and usage guidance.
3. Verify that the curated catalog is not presented as the exhaustive `rpc.discover` document and that its usage guidance directs the agent to `brp_execute`; Phase 3's static registry test owns the proof that `test_multiply` is not a native MCP tool.
4. Call `brp_list_agent_tools` on `[no_extras_app port]`; verify the mapped method-not-found response tells the agent to add `BrpExtrasPlugin` and does not confuse an absent endpoint with an installed empty catalog.
5. Call `brp_execute` on `[extras_app port]` with `method: "test/multiply"` and `{ "value": 6, "factor": 7 }`; verify the expected product and result fields.
6. Send schema-valid integer overflow through `brp_execute`; verify the application BRP error code/data are preserved.
7. Call `brp_list_agent_tools` again on `[extras_app port]` and verify the same live catalog without any sync, clear, reconnect, or mutable MCP state.

The test requires no initial/final catalog cleanup because the MCP owns no catalog state. Do not add notification timing, host refresh, dynamic tool names, or direct curl to the agent integration case.

The live integration gate has one external prerequisite required by `CLAUDE.md`: after the MCP implementation is complete and locally validated, run `cargo install --path mcp`, then stop and have the user reload the host session so it starts the newly installed MCP binary and obtains the new static tool schema. Do not claim `/integration_tests agent_tools` passed from a session still connected to the old MCP process. The two catalog calls inside the reloaded test session still occur without a reconnect between them. If the host cannot be reloaded inside the current delegation run, complete the repository changes and Rust checks, report this prerequisite, and leave the phase uncheckpointed until the integration case can run.

**Files:**
- `CLAUDE.md` — read-only source for the install/reload verification prerequisite.
- `test-app/Cargo.toml` — add unconditional workspace `schemars` for fixture derives.
- `test-app/examples/extras_plugin.rs` — add the typed result, publish agent metadata for the existing `test/multiply` handler, and preserve its overflow diagnostics.
- `test-app/examples/no_extras_plugin.rs` — unchanged live fixture proving the catalog endpoint is absent without extras.
- `.claude/integration_tests/agent_tools.md` — two-app transport discovery, curated/missing catalog behavior, successful execution, repeated stateless listing, and handler-error sequence.
- `.claude/config/integration_tests.json` — register the two-app case for `extras_plugin` and `no_extras_plugin`.
- `.claude/agents/integration-tester.md` — authorize the fixed discovery tool and retain execution/discovery permissions.
- `.claude/commands/integration_tests.md` — read-only runner contract; edit only if the new case exposes a general omission.
- `docs/brp-registration.md` — phase review records final observed behavior in the Retrospective.

**Constraints from prior phases:** Phase 1 supplies the exact fixture API; Phase 2 supplies deterministic live catalog serialization; Phase 3 supplies the fixed MCP discovery result, unchanged `brp_execute`, and static proof that catalog entries do not become native MCP tools, while deliberately deferring true live fetch tests here; Phase 4 supplies final names and documented workflow. The fixture's mutable `RemoteMethods` guard must end before `register_agent_tool`. The integration test proves the live list-to-execute contracts without dynamic tool refresh, but the host must start a new installed MCP process once before the test so the fixed `brp_list_agent_tools` schema exists.

**Acceptance gate:** `cargo nextest run --all-features --workspace --tests` is green; `cargo build --release --all-features --workspace --examples` is green; the updated MCP is installed with `cargo install --path mcp` and the host is reloaded onto that binary; `/integration_tests agent_tools` then passes against both assigned ports. The agent distinguishes `rpc.discover` from the curated catalog, reads exact schemas and list-to-execute guidance, receives the documented missing-plugin response from `no_extras_plugin`, invokes `test/multiply` through `brp_execute`, observes the expected product, preserves overflow BRP code/data, and repeats live catalog discovery without state management. Phase 3's registry test remains the authoritative proof that no catalog entry is registered as a native MCP tool; the full `clippy` skill is green.
