# Publishing BRP agent tools from Bevy applications

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Build a passive developer-published catalog that agents inspect with `brp_list_agent_tools` and invoke through the existing `brp_execute` tool.

## Delegation Context

- **Project:** `bevy_brp` workspace: `bevy_brp_extras` 0.23.0-dev lets Bevy developers publish curated agent-facing metadata for existing BRP methods; `bevy_brp_mcp` 0.23.0-dev exposes the fixed `brp_list_agent_tools` discovery tool and retains `brp_execute` as the invocation path; unpublished `bevy_brp_test_apps` supplies the end-to-end fixture.
- **Stack:** Rust 2024; Bevy and `bevy_remote` 0.19.0; `rmcp` 2.2.0; `schemars` 1.2.1; `serde` and `serde_json`; existing `reqwest` BRP client.
- **Layout:** `extras/{src,examples,tests,README.md,CHANGELOG.md,Cargo.toml}` — passive registration API, catalog BRP endpoint, example, tests, and developer guidance; `mcp/{src,help_text,README.md,CHANGELOG.md}` — one fixed catalog-listing MCP tool and cross-linked discovery/invocation guidance; `test-app/{Cargo.toml,examples/extras_plugin.rs}` — `test/multiply` fixture; `.claude/{integration_tests,config,agents,commands}` — real agent integration coverage; `docs/brp-registration.md` — authoritative phased plan.
- **Key files:** `docs/brp-registration.md` — approved architecture and delegate-ready plan; `Cargo.toml` — workspace dependency/version source; `extras/Cargo.toml` — add `schemars` as a normal dependency; `extras/src/lib.rs` — module declaration, public `AgentTool`/`AppAgentToolExt` exports, and crate rustdoc; `extras/src/constants.rs` — `brp_extras/agent_tools` method and catalog-version constants; `extras/src/plugin.rs` — order-independent resource initialization and instant catalog-method registration; `extras/src/agent_tools/mod.rs` — feature module boundary and crate-private wire exports; `extras/src/agent_tools/registration.rs` — owned builder, extension trait, passive resource, construction-time validation, rustdoc, and unit tests; `extras/src/agent_tools/catalog.rs` — deterministic versioned catalog encoding, live instant-method checks, handler, and unit tests; `extras/examples/agent_tool_registration.rs` — runnable typed-schema registration example; `extras/tests/agent_tool_registration.rs` — focused public-API/example support tests; `extras/README.md` — concise developer workflow; `extras/CHANGELOG.md` — public API/catalog entry; `mcp/src/brp_tools/tools/brp_list_agent_tools.rs` — fixed port-parameterized catalog reader and typed result; `mcp/src/brp_tools/tools/mod.rs` — tool exports; `mcp/src/brp_tools/mod.rs` — facade exports; `mcp/src/tool/name.rs` — static `ToolName` registration, parameters, handler, and discovery annotation; `mcp/src/tool/registry.rs` — read-only static registry assembly reference; `mcp/src/mcp_service.rs` — read-only immutable built-in listing/dispatch reference; `mcp/src/brp_tools/brp_client/client.rs` — existing BRP request path; `mcp/src/brp_tools/tools/brp_execute.rs` — existing exact-name discovery and invocation behavior that must remain unchanged; `mcp/src/brp_tools/tools/rpc_discover.rs` — existing transport-level method discovery boundary; `mcp/help_text/brp_list_agent_tools.txt` — catalog purpose and `brp_execute` handoff; `mcp/help_text/brp_execute.txt` — catalog cross-reference; `mcp/help_text/rpc_discover.txt` — distinction between all BRP methods and curated agent tools; `mcp/README.md` — list-then-execute workflow; `mcp/CHANGELOG.md` — fixed discovery-tool entry; `test-app/Cargo.toml` — schema derive dependency for the fixture; `test-app/examples/extras_plugin.rs` — publish metadata for existing `test/multiply`; `.claude/integration_tests/agent_tools.md` — catalog distinction, schema, successful execution, and BRP error coverage; `.claude/config/integration_tests.json` — register the `agent_tools` case; `.claude/agents/integration-tester.md` — authorize `brp_list_agent_tools` and retain `brp_execute`; `.claude/commands/integration_tests.md` — read-only integration-runner contract; `.cargo/config.toml` — workspace `mcp-debug` cfg; `.github/workflows/ci.yml` — canonical build/test/format commands.
- **Build:** `cargo build --release --all-features --workspace --examples`
- **Test:** `cargo nextest run --all-features --workspace --tests`; final repository integration phase: `/integration_tests agent_tools`.
- **Lint:** Full `clippy` skill, dispatched with `auto-proceed`; direct formatting, if needed, is `cargo +nightly fmt --all`, never plain `cargo fmt`.
- **Style:** `zsh ~/.claude/scripts/rust_style/load-rust-style.sh --project-root /Users/natemccoy/rust/bevy_brp`
- **Invariants:** Public feature surface is exactly owned `AgentTool` plus `AppAgentToolExt`; publishing metadata is separate from registering the backing BRP handler, and schema-derived Rust types generate documentation only—they never decode requests, encode responses, or require typed support in MCP. The passive version-1 `brp_extras/agent_tools` catalog is a curated subset of `RemoteMethods`, sorted deterministically, initialized regardless of plugin/registration order, and read only when requested; every advertised backing method must exist and be instant. Add no systems, polling, watchers, channels, background synchronization, or per-frame work. Catalog schemas describe the raw JSON-RPC `params` and raw BRP `result` values consumed and returned by the backing method; preserve raw JSON Schema support and perform no MCP-side compilation or runtime validation. `brp_list_agent_tools` is an ordinary immutable built-in MCP tool that fetches the live catalog on every call and returns `name`, `description`, `method`, `params_schema`, and `result_schema`; it creates no native per-entry tools, mutable catalog state, overlay, sync/clear lifecycle, `tools/list_changed`, dispatcher, or alternate execution path. `rpc.discover` continues to list every registered BRP method; `brp_list_agent_tools` lists only developer-published agent guidance; `brp_execute` remains the sole supported MCP invocation path and retains its live exact-method validation, raw parameter forwarding, and BRP error preservation. Documentation and tool results must explicitly teach `brp_list_agent_tools` → `brp_execute`, while noting that direct JSON-RPC HTTP remains possible outside MCP. Completion requires full public rustdoc, the runnable example and support test, concise README/help text, both changelogs, and an integration case proving the catalog/raw schemas/backing method plus successful and failing `brp_execute` calls.

## Phases

### Phase 1 — Add the typed agent-tool publication API  · status: todo

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

### Phase 2 — Publish the passive agent-tool catalog  · status: todo

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

Before serializing a nonempty catalog, inspect the live `RemoteMethods` resource. Every advertised `method` must exist and be `RemoteMethodSystemId::Instant`. Reject missing and `Watching` methods with deterministic BRP errors that identify the agent name and backing method. This request-time check permits any construction order while preventing the catalog from advertising methods that `brp_execute` cannot invoke as an instant call.

Add no update systems, polling, watchers, channels, background synchronization, retries, caches, or automatic refresh. The catalog response is the current application state at the time of the request.

**Files:**
- `extras/src/constants.rs` — add the `brp_extras/agent_tools` method and version constants.
- `extras/src/agent_tools/mod.rs` — expose crate-private catalog items to the plugin.
- `extras/src/agent_tools/catalog.rs` — wire types, deterministic serialization, live instant-method checks, BRP errors, and unit tests.
- `extras/src/plugin.rs` — initialize the resource without replacement and register the instant catalog method.
- `extras/src/lib.rs` — reference the catalog endpoint in crate rustdoc without exporting wire types.

**Constraints from prior phases:** Phase 1 provides owned `AgentTool`, passive `RegisteredAgentTools`, exact validation rules, raw optional schemas, and order-independent resource initialization. Preserve those semantics; the catalog adds only request-time backing-method validation and serialization.

**Acceptance gate:** Tests prove empty and populated catalogs, exact version/field names, omitted optional schemas, deterministic agent-name order, raw object/array/primitive/boolean schemas without rewriting, registration before and after plugin addition, live instant acceptance, missing-method rejection, watching-method rejection, and absence of any added scheduled system or per-frame work. The on-demand catalog handler is the only new registered system. Build and Test from Delegation Context are green; the full `clippy` skill is green.

### Phase 3 — Add fixed agent-tool discovery to MCP  · status: todo

#### Work Order

**Goal:** Agents can fetch the running application's curated catalog through one ordinary built-in MCP tool and then invoke a selected backing method with `brp_execute`.

**Spec:**

Add the fixed `brp_list_agent_tools` tool through the existing `ToolName`/`ToolFn` registry. It accepts only the standard BRP port:

```rust
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct ListAgentToolsParams {
    #[serde(default)]
    pub port: Port,
}
```

The handler calls `brp_extras/agent_tools` at that port through the existing arbitrary application-method `BrpClient` path. Define MCP-local private wire types; `bevy_brp_mcp` must not depend on `bevy_brp_extras`. Reject unsupported catalog versions and malformed documents with an error that identifies the catalog stage and port. Preserve BRP connection, JSON-RPC code/data, and method-not-found details through existing error conventions. If the endpoint is absent, tell the agent that the running application must add `BrpExtrasPlugin`; an installed plugin with no published entries instead returns a valid empty catalog.

Use `BrpListAgentTools` for the handler, `ListAgentToolsParams` for its public parameter type, `ListAgentToolsResult` for its public result type, and `ListedAgentTool` for each serialized result entry. Export those four MCP types through the existing tool facades because static registration and result-schema generation consume them. Keep the versioned BRP response decoder private. `ListedAgentTool` carries public `name`, `method`, `description`, and optional `serde_json::Value` parameter/result schema fields. `ListAgentToolsResult` carries `tools`, the fixed `usage` string below, and the existing message-template/metadata fields needed by `ResultStruct`; it must expose the catalog records as the structured result rather than nesting the raw BRP response.

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
- `bevy_rpc_discover`: lists every registered BRP method at the transport level; application methods may supply only names, so use `brp_list_agent_tools` for curated descriptions and schemas.
- `brp_execute`: invokes a method by exact name with raw `params`; use `brp_list_agent_tools` first when the application publishes agent guidance.

The help and result must distinguish the MCP workflow from developer debugging: direct HTTP JSON-RPC to `http://127.0.0.1:<port>/jsonrpc` remains valid outside MCP, but agents should use `brp_execute` for MCP-native invocation, timeout/error handling, and method validation.

**Files:**
- `mcp/src/brp_tools/tools/brp_list_agent_tools.rs` — parameters, private wire decoding, typed result, BRP request, errors, and unit tests.
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

**Acceptance gate:** Tests cover the static registry containing `brp_list_agent_tools`; default and explicit ports; empty and populated decoding; exact preservation of names, methods, descriptions, and optional raw schemas; unsupported version/malformed response; missing extras endpoint; preserved BRP errors; exact usage guidance; and repeated calls fetching current data without service state. Help-text tests prove all three cross-references and do not imply native dynamic tools. Existing `bevy_rpc_discover`, `brp_execute`, static listing, and built-in dispatch tests remain green. Build and Test from Delegation Context are green; the full `clippy` skill is green.

### Phase 4 — Ship developer and agent documentation  · status: todo

#### Work Order

**Goal:** Developers can publish one typed agent catalog entry and agents can discover and invoke it from the documented workflow without reading implementation source.

**Spec:**

Create `extras/examples/agent_tool_registration.rs` as a complete runnable example. It defines `MultiplyParams: Deserialize + JsonSchema` and `MultiplyResult: Serialize + JsonSchema`, registers an instant `example/multiply` BRP handler, adds `BrpExtrasPlugin`, and publishes:

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

Complete and cross-link rustdoc for `AgentTool`, every builder method, `AppAgentToolExt`, and `register_agent_tool`. Cover owned values, name validation, required description, raw schema semantics, compile-time-only `JsonSchema` types, parameterless methods, optional result documentation, construction-time panics, request-time live instant validation, plugin requirement, and unsupported runtime registration. Private resource/wire types document their invariants without becoming public.

Add a focused public-API/example support test that constructs an app through only exported APIs, runs the registered catalog method, decodes its JSON value, and asserts the registered name/method/description plus generated raw parameter/result schemas without inspecting source text or exposing private resource/wire types. Update `extras/README.md` and `mcp/README.md` with the same concise list-then-execute workflow. The MCP README may include the curl-equivalent JSON-RPC request only as a developer debugging alternative; it must recommend `brp_execute` to agents.

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

**Constraints from prior phases:** Use the exact Phase 1 public API and raw schema meanings, Phase 2 endpoint/name/version, and Phase 3 fixed MCP tool/result/help vocabulary. Documentation must not restore any removed dynamic-registration concept.

**Acceptance gate:** The example builds and its support test passes; public rustdoc examples compile; all snippets use exact shipped names and fields; READMEs and help text distinguish exhaustive `rpc.discover` from curated `brp_list_agent_tools`; both teach `brp_execute` as the MCP invocation path; curl is labeled developer-only; both changelogs identify their public additions; no documentation claims that per-entry native tools appear. Build and Test from Delegation Context are green; the full `clippy` skill, including documentation checks, is green.

### Phase 5 — Add repository integration coverage  · status: todo

#### Work Order

**Goal:** The real extras test application and integration agent prove curated discovery followed by successful and failing `brp_execute` invocation.

**Spec:**

Extend `ParameterizedBrpPlugin` in `test-app/examples/extras_plugin.rs`. Keep the existing instant `test/multiply` method and checked-multiplication error behavior. Derive/use JSON schemas for its `{ value: i64, factor: i64 }` parameters and `{ value, factor, product }` result, then publish:

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

Create `.claude/integration_tests/agent_tools.md` and register it for `extras_plugin`. Update the integration-tester agent permissions for the fixed `mcp__brp__brp_list_agent_tools` tool and retain its existing authorization for `mcp__brp__brp_execute` and transport discovery. The integration case must:

1. Call `bevy_rpc_discover` on port 15702 and verify that `test/multiply` is one member of the complete BRP method inventory.
2. Call `brp_list_agent_tools` and verify `test_multiply`, its backing `method`, description, raw parameter schema, raw result schema, and usage guidance.
3. Verify that the curated catalog is not presented as the exhaustive `rpc.discover` document and that its entry is not a newly callable native MCP tool.
4. Call `brp_execute` with `method: "test/multiply"` and `{ "value": 6, "factor": 7 }`; verify the expected product and result fields.
5. Send schema-valid integer overflow through `brp_execute`; verify the application BRP error code/data are preserved.
6. Call `brp_list_agent_tools` again and verify the same live catalog without any sync, clear, reconnect, or mutable MCP state.

The test requires no initial/final catalog cleanup because the MCP owns no catalog state. Do not add notification timing, host refresh, dynamic tool names, or direct curl to the agent integration case.

**Files:**
- `test-app/Cargo.toml` — add/use workspace `schemars` for fixture derives if required.
- `test-app/examples/extras_plugin.rs` — publish agent metadata for the existing `test/multiply` handler and preserve its overflow diagnostics.
- `.claude/integration_tests/agent_tools.md` — transport discovery, curated catalog, successful execution, repeated stateless listing, and handler-error sequence.
- `.claude/config/integration_tests.json` — register the case for `extras_plugin`.
- `.claude/agents/integration-tester.md` — authorize the fixed discovery tool and retain execution/discovery permissions.
- `.claude/commands/integration_tests.md` — read-only runner contract; edit only if the new case exposes a general omission.
- `docs/brp-registration.md` — phase review records final observed behavior in the Retrospective.

**Constraints from prior phases:** Phase 1 supplies the exact fixture API; Phase 2 supplies deterministic live catalog serialization; Phase 3 supplies the fixed MCP discovery result and unchanged `brp_execute`; Phase 4 supplies final names and documented workflow. The integration test proves those contracts without depending on dynamic host tool refresh.

**Acceptance gate:** `cargo nextest run --all-features --workspace --tests` is green; `cargo build --release --all-features --workspace --examples` is green; `/integration_tests agent_tools` passes; the agent distinguishes `rpc.discover` from the curated catalog, reads exact schemas, invokes `test/multiply` through `brp_execute`, observes the expected product, preserves overflow BRP code/data, and repeats catalog discovery without state management; the full `clippy` skill is green.
