# Execute arbitrary BRP methods

## Status

Implemented and validated through the installed MCP binary.

## Objective

Allow `brp_execute` to call any BRP method currently reported by `rpc.discover` on the requested
port. This makes application-defined methods such as `hana/apply_stack` callable without adding a
statically compiled MCP tool for each method.

The MCP continues to treat `params` as caller-supplied JSON. `rpc.discover` proves that the method
is registered, but Bevy 0.19 does not provide the method's parameter schema. Agents obtain that
information from the application's source, documentation, catalog methods, user input, or handler
errors.

## Current failure

`ExecuteParams::method` is the generated `BrpMethod` enum. Although the MCP input schema presents
the field as a string, Serde rejects any string not represented by a statically declared
`ToolName` BRP mapping. A live Hana app reports `hana/apply_stack` through `rpc.discover`, but
`brp_execute` rejects that value before sending an HTTP request.

The generated `BrpMethod` enum remains useful for statically declared tools, watch tasks, and
format-error classification. Arbitrary execution must not weaken those typed call sites.

## User-visible contract

`brp_execute` keeps its existing request fields:

```json
{
  "port": 15720,
  "method": "hana/apply_stack",
  "params": {
    "effects": [
      {
        "id": "tile",
        "parameters": {
          "repeats": [5, 5],
          "amount": 1.0
        }
      }
    ],
    "tools": []
  }
}
```

The `method` field becomes an arbitrary string at the MCP boundary. Before forwarding `params`,
`brp_execute` calls `rpc.discover` on the same `port` and requires an exact method-name match.

If discovery does not report the method, the call returns a tool error that names the requested
method and port and lists the discovered methods in deterministic order. The requested method is
not sent. Discovery connection and decoding failures remain distinct errors rather than being
reported as a missing method.

A successful OpenRPC response with no exact match is the only missing-method case. A BRP error
from `rpc.discover`, a missing or malformed `methods` collection, or a malformed method entry is a
discovery failure. Missing arbitrary `brp_extras/*` methods retain the existing guidance to add
`BrpExtrasPlugin`.

If discovery reports the method, `brp_execute` sends the method name and `params` unchanged through
the existing JSON-RPC transport and returns the existing raw response format. Parameter validation
belongs to the application-defined BRP handler. Execution errors preserve the JSON-RPC error code
and optional data so the agent can distinguish invalid parameters from other handler failures.
Errors identify whether they occurred during discovery or execution. A registration can still
change between the preflight and execution requests; an execution-time `METHOD_NOT_FOUND` remains
an execution error and retains the extras-specific guidance when applicable.

The preflight adds one local HTTP request, one `reqwest::Client` construction, OpenRPC response
download and deserialization, and a method-name scan to every `brp_execute` call. Do not cache the
result: applications may add or replace `RemoteMethods` registrations while running, and
`brp_execute` is a diagnostic escape hatch rather than a per-frame path. Sort and allocate the
complete name list only when constructing a missing-method error.

## Internal design

Keep the generated `BrpMethod` enum as the closed set used by statically declared MCP tools.
Add a private transport-level enum with known and application-defined variants. The known variant
stores `BrpMethod` without allocating; the application variant owns a `String`. Keep the existing
public `const BrpClient::new(BrpMethod, Port, Option<Value>)` constructor unchanged and add one
crate-private constructor for an application-defined name. The enum provides the borrowed method
string and an optional known `BrpMethod` for operation classification.

`BrpHttpClient` borrows the method string from `BrpClient` rather than owning or cloning the
transport-level enum. Operation-specific format discovery receives only the optional known
`BrpMethod`. Existing known-method calls therefore retain their allocation and classification
behavior.

`ExecuteParams::method` uses the owned application-defined form. Existing generated tool handlers,
watch tasks, status checks, shutdown handling, and type-guide calls continue passing known
`BrpMethod` variants.

Place discovery parsing with the `rpc_discover` tool's types. Deserialize the successful result as
Bevy's `OpenRpcDocument` and add one internal operation that checks an exact method-name match for a
port. `BrpExecute` uses that operation for membership validation. The public `rpc_discover`
response remains unchanged.

Do not add a second HTTP implementation or duplicate JSON-RPC response parsing for arbitrary
methods. Both known and application-defined names must use the existing request, timeout, error,
and tracing behavior.

## Repository changes

### MCP implementation

- Change `ExecuteParams::method` from `BrpMethod` to the arbitrary method-name type exposed as a
  JSON string.
- Preserve `BrpMethod` as the generated enum for statically known operations.
- Generalize the `BrpClient` to `BrpHttpClient` boundary so both known and owned method names use
  the same transport without adding allocations to known-method calls.
- Add internal `rpc.discover` result parsing and exact membership validation.
- Preserve the existing `brp_extras/*` missing-plugin explanation for known and arbitrary extras
  names.
- Preserve execution error codes and optional data, and identify discovery versus execution errors.
- Change the static `brp_execute` environment-impact annotation from `AdditiveIdempotent` to
  `DestructiveNonIdempotent`; an arbitrary method cannot safely promise read-only, additive, or
  repeatable behavior.

### Documentation

- Update `mcp/help_text/brp_execute.txt` to describe the discovery validation and explain that the
  caller supplies raw parameters.
- Correct `mcp/help_text/rpc_discover.txt`: Bevy 0.19 enumerates registered method names but returns
  empty parameter lists and no per-method result schema, description, or examples.
- Update `mcp/README.md` with a discover-then-execute example for an application-defined method.
- Add an `mcp/CHANGELOG.md` entry because the accepted `method` input expands beyond the previous
  enum.

### Tests

- Add unit coverage proving `ExecuteParams` deserializes an application-defined method name.
- Add unit coverage for discovery parsing, exact membership, deterministic missing-method output,
  malformed discovery responses, discovery BRP errors, transport errors, and missing arbitrary
  `brp_extras/*` guidance.
- Keep coverage showing statically known `BrpMethod` values retain their current JSON-RPC method
  strings and format-discovery classifications.
- Add a transport-level test fixture that records requested JSON-RPC methods. When discovery omits
  a sentinel method, assert that the fixture receives only `rpc.discover`. Also cover the race in
  which discovery includes a method but execution returns `METHOD_NOT_FOUND`.
- Register a test-only instant BRP method in `test-app/examples/extras_plugin.rs` that accepts
  typed integer operands and returns their product. Add an integration specification that verifies:
  - `rpc_discover` reports the test method;
  - `brp_execute` invokes it and round-trips nested JSON;
  - an unregistered method returns the discovery-stage error;
  - an application-handler invalid-parameter error remains distinguishable from discovery
    rejection and retains its JSON-RPC code and optional data.
- Add a tool-definition test asserting that `brp_execute` is marked destructive and
  non-idempotent.

Do not add lint suppressions. Tests use `cargo nextest run` under the repository instructions.

## Non-goals

- Dynamic MCP tool registration or `tools/list_changed` notifications.
- Parameter or result schema registration for application-defined BRP methods.
- Changes to `bevy_brp_extras`.
- Inferring parameters from an erased `RemoteMethodSystemId`.
- Caching method lists across calls or ports.
- Adding application-defined methods to the generated `BrpMethod` enum.
- Changing the response framework to derive dynamic `call_info` from request parameters. The
  existing response already includes the supplied `method` under `parameters`; arbitrary dispatch
  does not require a broader response-plumbing change.

## Implementation sequence

1. Add the transport-level method-name representation and route existing `BrpMethod` calls through
   it without changing their behavior.
2. Change `ExecuteParams`, add discovery membership validation, and preserve raw parameter
   forwarding and response reporting.
3. Add unit and test-app integration coverage for application-defined success and failure paths.
4. Update help text, README documentation, and the changelog.
5. Run `cargo +nightly fmt --all`, the focused MCP tests with `cargo nextest run`, and the affected
   integration specification. Build and installation testing follows the repository's MCP reload
   procedure so the running MCP subprocess is not mistaken for the newly built binary.

## Acceptance criteria

- `rpc_discover` reports a test application-defined method.
- `brp_execute` accepts that method name, validates it on the requested port, and returns the
  handler's response.
- `brp_execute` rejects a name absent from that port without invoking another handler.
- Existing statically declared MCP tools and watch tools retain their current behavior.
- `ExecuteParams` accepts application-defined method strings even though the exposed MCP schema
  already renders the current enum as a string.
- `brp_execute` is marked destructive and non-idempotent.
- Discovery and execution failures remain distinct and execution preserves BRP error details.
- Documentation does not claim that Bevy 0.19 discovery supplies parameter or result schemas.

## Team review

Cycle 1 completed with three reviewers covering correctness, Rust type design, performance,
failure handling, simplicity, and caller ergonomics.

- **F1 — accepted:** Preserve `BrpClient::new` and the allocation-free known-method path; use a
  private known-or-application transport enum and lend its string to `BrpHttpClient`.
- **F2 — accepted:** Mark `brp_execute` destructive and non-idempotent because arbitrary methods
  have unknown effects.
- **F3 — accepted:** Treat discovery transport, BRP, and decoding failures separately from a valid
  missing-method result; retain extras-specific guidance.
- **F4 — accepted:** Preserve JSON-RPC code and data on execution failures and identify the failing
  stage, including the discovery-to-execution registration race.
- **F5 — accepted:** Add a request-recording transport test to prove a discovery rejection does not
  dispatch the requested method.
- **F6 — accepted:** Parse the OpenRPC result with Bevy's exported document type and record the
  complete preflight cost without adding caching or shared-client work.
- **F7 — dropped from scope:** Dynamic `call_info.brp_method` would require request-aware response
  plumbing across the shared tool framework. The input `parameters` already identify the requested
  method, so this is not required to execute application-defined BRP methods.

No unresolved product or design decision survived the review cycle.

## Implementation verification

- `cargo +nightly fmt --all`
- `cargo check -p bevy_brp_mcp`
- `cargo check -p bevy_brp_test_apps --example extras_plugin`
- `cargo nextest run -p bevy_brp_mcp` — 49 tests passed
- `cargo clippy -p bevy_brp_mcp -p bevy_brp_test_apps --all-targets -- -D warnings`
- `cargo install --path mcp`

- Launched `extras_plugin` with the debug profile on port 15802.
- `rpc_discover` reported `test/multiply` among 39 methods.
- `brp_execute` called `test/multiply` with `{ "value": 6, "factor": 7 }` and returned product 42.
- Omitting `factor` preserved execution-stage code `-32602` and the handler's expected-field data.
- Calling `test/missing` was rejected at the discovery stage and returned the sorted method list.
- The example shut down cleanly through `brp_extras/shutdown`.
