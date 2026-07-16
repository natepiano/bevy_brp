# Execute Arbitrary BRP Methods

## What it is

`brp_execute` invokes any BRP method currently registered by a running Bevy application, including
application-defined methods such as `hana/apply_stack` or `test/multiply`. Callers provide a method
name, target port, and optional raw JSON parameters; the MCP verifies that the application advertises
the exact method through `rpc.discover`, then sends it through the existing BRP transport.

## How it works

`mcp/src/brp_tools/tools/brp_execute.rs` exposes:

```rust
pub struct ExecuteParams {
    pub method: String,
    pub params: Option<Value>,
    pub port: Port,
}
```

`BrpExecute::handle_impl` calls `rpc_discover::discover_method_names(port)`. The helper in
`mcp/src/brp_tools/tools/rpc_discover.rs` executes typed `BrpMethod::RpcDiscover`, requires a
successful result, deserializes Bevy's `OpenRpcDocument`, rejects malformed documents and empty
method names, and returns the registered names. `brp_execute` performs a case-sensitive exact-match
scan.

If the name is absent, execution stops before the requested method is sent. The tool error identifies
the `discovery` stage, requested method and port, and includes the discovered names sorted for
deterministic diagnostics. Missing `brp_extras/*` names retain the existing `BrpExtrasPlugin`
guidance.

If present, `brp_execute` constructs `BrpClient::for_application(String, Port, Option<Value>)` and
calls `execute_raw()`. `mcp/src/brp_tools/brp_client/client.rs` stores methods as:

```rust
enum BrpMethodName {
    Known(BrpMethod),
    Application(String),
}
```

`BrpClient::new(BrpMethod, Port, Option<Value>)` remains the typed path for generated tools, watches,
status and shutdown operations, and type-guide work. Both variants lend `&str` to `BrpHttpClient`,
so known and application-defined requests share the URL construction, timeout, tracing, status
checks, and JSON-RPC response parser in `mcp/src/brp_tools/brp_client/http_client.rs`. Known methods
still provide `Option<BrpMethod>` for operation-specific format classification; application names do
not.

Execution success uses the existing `ExecuteResult` response. Execution failure identifies the
`execution` stage and preserves the BRP error code and optional data. A method removed after
successful discovery is therefore reported as an execution-stage race.

Supporting files define and verify the user-facing contract:

- `mcp/help_text/brp_execute.txt` documents discover-then-forward behavior and raw parameters.
- `mcp/help_text/rpc_discover.txt` and `mcp/README.md` explain Bevy 0.19 discovery limits.
- `mcp/src/tool/name.rs` marks `brp_execute` destructive and non-idempotent.
- `test-app/examples/extras_plugin.rs` registers `test/multiply`, validates typed integer operands,
  and returns structured parameter errors.
- `.claude/integration_tests/introspection.md` covers discovery, successful execution, handler
  errors, and discovery rejection.
- `mcp/CHANGELOG.md` records the expanded method input and corrected discovery guidance.

## Invariants

- Validate against a fresh `rpc.discover` result from the same port on every call; do not cache
  registrations.
- Match method names exactly and case-sensitively.
- Only a valid discovery response without an exact match is a missing-method result. Transport
  failures, BRP errors, absent results, and malformed OpenRPC data remain discovery failures.
- Never dispatch the requested method after a discovery miss.
- Forward `params` as caller-supplied JSON; application handlers own parameter validation.
- Keep `BrpMethod` as the closed representation for statically known operations.
- Route known and application methods through the same BRP transport and response handling.
- Preserve BRP error codes and optional data and distinguish discovery from execution failures.
- Retain `BrpExtrasPlugin` guidance for missing `brp_extras/*` methods.
- Keep `brp_execute` annotated as destructive and non-idempotent because arbitrary methods have
  unknown effects.

## Calibration and gotchas

- Bevy 0.19 discovery reports method names, but its entries have empty parameter lists and no result
  schema, description, or examples. Parameter knowledge must come from application source,
  documentation, the curated `brp_list_agent_tools` catalog, user input, or handler errors.
- A successful call makes two HTTP requests: discovery and execution. Each creates a new
  `reqwest::Client` and has its own 30-second timeout. A discovery miss sends only the first request.
- Registrations can change between preflight and execution. Preserve an execution-stage
  `METHOD_NOT_FOUND`, including extras guidance where applicable.
- The full method list is sorted and allocated only for a missing-method error. Success performs only
  the exact-match scan.
- `OpenRpcDocument` deserialization deliberately treats structural drift as a decoding failure
  instead of accepting a partial method list.
- `BrpClient::for_application` is public and does not itself perform discovery. The validation
  invariant lives in `BrpExecute::handle_impl`; new direct callers must decide whether they need
  equivalent validation.
- The test plugin registers `test/multiply` only when the `RemoteMethods` resource exists; otherwise
  it warns and leaves the method absent.

## Why it is this way

Separating known methods from owned application strings preserves the typed, allocation-free surface
of existing tools while opening only the dynamic executor boundary. Lending the resulting method
string to `BrpHttpClient` avoids a second transport implementation.

Live discovery is intentional because `RemoteMethods` is runtime state and registrations may change.
Parsing Bevy's exported `OpenRpcDocument` keeps validation tied to the protocol's real shape.

The preflight validates membership rather than parameters because Bevy 0.19 does not expose
sufficient schemas. Preserving application error codes and data supplies useful diagnostics without
implying that the MCP understands each method. The conservative destructive and non-idempotent
annotation is the only safe classification for runtime-selected behavior.
