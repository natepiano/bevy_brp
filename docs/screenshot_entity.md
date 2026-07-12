# Entity-scoped screenshots

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Extends the existing screenshot operation with optional entity/name scope while making full and cropped PNG delivery terminal and reliable.

## Delegation Context

- **Project:** `bevy_brp` workspace — `bevy_brp_extras` supplies extra Bevy Remote Protocol methods, `bevy_brp_mcp` exposes them as MCP tools, and `bevy_brp_test_apps` supplies runtime fixtures.
- **Stack:** Rust 2024 edition (`rustc 1.97.0`; formatting toolchain `rustc 1.98.0-nightly`), Bevy `0.19.0`, `bevy_remote 0.19.0`, workspace crates `0.21.0-dev`, `rmcp 1.7.0`, Serde/JSON, Schemars, and Bevy render/UI/camera APIs.
- **Layout:** `extras/` — BRP implementation, feature gates, docs, and unit tests; `mcp/` — typed MCP facade, local composite name resolution, registry/help, docs, and changelog; `test-app/` — deterministic Bevy fixtures; `.claude/integration_tests/`, `.claude/scripts/integration_tests/`, `.claude/agents/` — runtime specifications, screenshot helpers, and tester authorization.
- **Key files:**
  - `Cargo.toml` — workspace dependency versions and lint policy.
  - `extras/Cargo.toml` — add default-enabled `ui = ["bevy/bevy_ui"]` while retaining no-default-feature AABB support; Bevy UI brings its required text/sprite family, but add no separate textual-snapshot or PBR feature.
  - `extras/src/constants.rs` — existing screenshot method fields, terminal status fields, and zero-padding default.
  - `extras/src/plugin.rs` — existing watching screenshot registration and `CapturePlugin` installation.
  - `extras/src/screenshot.rs` — current flat screenshot handler replaced by the `extras/src/screenshot/` module in Phase 1.
  - `extras/src/screenshot/mod.rs` — existing screenshot module root, watching handler, scope dispatch, and common `EntityCapture`.
  - `extras/src/screenshot/capture/` — shipped capture facade, tokens, batches, terminal state machine, task completion, path generations, publication, and focused child modules.
  - `extras/src/screenshot/request.rs` — raw wire decoding and typed `ScreenshotRequest`/`ScreenshotScope`.
  - `extras/src/screenshot/aabb.rs` — AABB projection and selected-view visibility.
  - `extras/src/screenshot/ui.rs` — feature-gated UI bounds resolution.
  - `extras/src/lib.rs` — existing screenshot BRP documentation.
  - `extras/README.md` — screenshot API, feature, camera, completion, and composited-crop documentation.
  - `extras/CHANGELOG.md` — extras release entry.
  - `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — extend the existing MCP screenshot params and implement local name-to-ID composition before the extras call.
  - `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — generic MCP-only name-discovery params/result and standard `world.query` composition.
  - `mcp/src/brp_tools/tools/mod.rs` — declare/re-export generic name discovery; preserve the existing screenshot module.
  - `mcp/src/brp_tools/mod.rs` — facade re-exports.
  - `mcp/src/tool/name.rs` — existing screenshot tool metadata plus generic local name-discovery registration/routing.
  - `mcp/help_text/brp_extras_screenshot.txt` — full/entity/name scope, terminal publication, camera, feature, and composited-crop help.
  - `mcp/help_text/world_find_entities_by_name.txt` — exact/prefix/suffix/contains discovery help.
  - `mcp/README.md` — MCP tool documentation.
  - `mcp/CHANGELOG.md` — MCP release entry.
  - `test-app/examples/extras_plugin.rs` — deterministic UI, 2D, 3D, name, unsupported-bounds, viewport, and camera-ambiguity fixtures.
  - `test-app/examples/no_extras_plugin.rs` — isolated-port standard-BRP fixture proving name discovery does not depend on extras.
  - `.claude/integration_tests/extras_capture.md` — terminal full/entity captures, dimensions, pixels, errors, and cleanup.
  - `.claude/integration_tests/introspection.md` — verify the existing screenshot BRP method remains discoverable and no extras name method is introduced.
  - `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` — retire or repurpose obsolete screenshot polling after terminal completion ships.
  - `.claude/scripts/integration_tests/extras_assert_png.py` — new read-only complete-PNG, dimensions, uniformity, marker-pixel, and exact-crop comparison helper.
  - `.claude/scripts/integration_tests/test_extras_assert_png.py` — focused unit tests for the PNG assertion helper.
  - `.claude/agents/integration-tester.md` — retain screenshot authorization, authorize `mcp__brp__world_find_entities_by_name`, and permit the helper only through its exact relative `python3 .claude/scripts/integration_tests/extras_assert_png.py ...` invocation.
  - `.claude/config/integration_tests.json` — existing `extras_capture` and `introspection` runner registration.
  - `.github/workflows/ci.yml` — existing Ubuntu CI; add targeted Windows terminal-publication replacement coverage.
- **Build:** `cargo check --workspace` and `cargo check -p bevy_brp_extras --no-default-features`.
- **Test:** `cargo nextest run --workspace`, then run `/test extras_capture,introspection` with isolated ports after installing the updated MCP binary and restarting the MCP client session.
- **Test ownership:**
  - Phase 1 — `extras/src/screenshot/capture/` tests: explicit/legacy identity, watching cleanup order, same-target production, RGB/crop pixels, temporary ownership, destination generations, atomic publication, deadlines, disconnect commit boundary, stale workers, WASM rejection, and Windows replacement behavior.
  - Phase 2 — request, camera, AABB, capture-bridge, metadata, and method-contract tests: typed scope conversion, target eligibility, transformed bounds, scope fingerprints, inherited deadlines, terminal entity metadata, and no-default-feature compilation.
  - Phase 3 — `extras/src/screenshot/ui.rs` and method-contract tests: transformed/clipped UI bounds, viewport translation, hard bounds, UI visibility, target camera, precedence, feature gating, metadata, and shared capture regression.
  - Phase 4 — `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` tests: standard `world.query` composition, typed match modes, literal asterisks, deterministic ordering, malformed data, and operation without extras.
  - Phase 5 — `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` tests: full/ID/name MCP scope, unique/ambiguous name resolution, hidden capture token, extras request format, terminal result preservation, and registry contents.
  - Phase 6 — both runtime fixtures plus `.claude/scripts/integration_tests/test_extras_assert_png.py`: deterministic geometry/colors, isolated ports, and PNG helper success/failure behavior.
  - Phase 7 — `.claude/integration_tests/extras_capture.md` and `.claude/integration_tests/introspection.md`: live MCP/BRP behavior, exact crop identity, extras-independent discovery, method discovery/absence, authorization, and cleanup.
- **Lint:** Full `clippy` skill.
- **Style:** `zsh ~/.claude/scripts/rust_style/load-rust-style.sh --project-root /Users/natemccoy/rust/bevy_brp`
- **Invariants:** Use `cargo +nightly fmt --all -- --check`, never plain `cargo fmt`; extend the existing `brp_extras/screenshot` BRP method and `brp_extras_screenshot` MCP tool rather than creating screenshot-entity variants; both full and entity captures are internally asynchronous but terminal to BRP/MCP callers; neither success nor the final path is published before a complete PNG; extras accepts only optional entity ID scope, while MCP accepts optional entity ID or unique exact name and resolves names through standard `world.query`; generic name discovery is MCP-only and never requires extras; neither entity nor name means full primary-window capture, both is invalid, and camera/padding are invalid without entity scope; padding defaults to zero physical pixels; UI support is default-enabled but optional, and `--no-default-features` retains generic AABB screenshots without Bevy's UI dependency family; enabling UI necessarily enables the Bevy 0.19 text/sprite dependencies required by Bevy UI, but this feature adds no direct textual-snapshot scope; complete UI components take precedence over AABB and partial UI returns an initialization error; crops use the selected camera target and physical target coordinates, honor viewport/clip/target hard bounds, and contain partially covered pixels; output is a crop of the final composited target, not isolated rendering; coordinate one Bevy screenshot entity per normalized target, fan out same-target jobs, convert the complete image once using Bevy-compatible RGB semantics, reserve destination paths by generation through worker acknowledgement, and commit completed temporary files only after verifying current ownership; name matches are case-sensitive and deterministically ordered by entity ID, with explicit match modes rather than wildcard syntax; keep direct BRP payloads in `result: Option<Value>` for `ResultStruct`; `snapshot`/textual UI-tree inspection is separate scope and must not be added here; do not add lint suppressions, generic helper modules, speculative caches, benchmark infrastructure, or single-implementation traits.

## Phases

### Phase 1 — Terminal watching lifecycle, capture production, and publication  · status: done (`0b77a96a`)

#### Work Order

**Goal:** The existing full screenshot request has a deterministic one-shot watching lifecycle, shared target capture production, and generation-checked terminal PNG publication.

**Spec:**

Create the screenshot module layout and concrete lifecycle types:

```rust
enum CaptureIdentity {
    Token(CaptureToken),
    Legacy(RequestFingerprint),
}

enum CaptureState {
    Pending,
    Encoding,
    ReadyToPublish,
    Publishing,
    Completed,
    Failed,
    TimedOut,
    Abandoned,
}

#[derive(Resource)]
struct PendingScreenshotCaptures { /* identity state, path generations, target batches */ }
```

Accept an optional bounded nonempty `capture_id`. Explicit IDs become `CaptureIdentity::Token`; tokenless direct BRP calls use normalized path plus immutable request fingerprint. Identical active legacy requests coalesce; explicit tokens isolate otherwise identical calls; same-token/different-fingerprint is invalid. Keep destination reservations and target batches separate from watcher identity. Do not add capacity limits.

Define the frame protocol against Bevy 0.19 `RemoteSystems::ProcessRequests`: ingest controlled completion input before that set; each handler invocation stamps its identity seen and performs only an indexed state read; handlers never remove terminal entries. After the set, mark unseen identities abandoned and advance only seen/unexpired work. A later delivery pass exposes immutable `Completed`/`Failed`, then cleanup removes delivered entries. Replacement is the commit point: pre-commit disconnect can suppress publication; post-commit disconnect may leave the PNG.

Destination reservations are generation-owned independently of watcher identity. Otherwise identical active requests for one normalized destination and immutable fingerprint share the current reservation even when explicit tokens differ; their watcher results remain isolated. A different active fingerprint for the same destination returns an explicit destination-conflict error rather than silently superseding or timing out either request. Sequential reuse after terminal cleanup advances the generation, and stale workers from older generations are ignored. This is destination correctness, not an admission or capacity limit.

Bevy 0.19 may invoke a watching handler once after its response receiver closes and removes that watcher only in `RemoteSystems::Cleanup`. Therefore a request that first reaches `ReadyToPublish` in one frame is not eligible to publish in that frame. It must be stamped seen by a subsequent `ProcessRequests` pass; otherwise mark it abandoned. This one-frame liveness confirmation is required before the replacement commit point and does not delay terminal delivery after publication.

Add `ScreenshotJob { path, crop: Option<URect>, identity, path_generation, deadline }`. Key batches by `NormalizedRenderTarget`; join an active batch or spawn the target's sole Bevy `Screenshot` entity. Never spawn duplicate same-target screenshot entities in one frame because Bevy discards duplicates without `ScreenshotCaptured`.

The observer takes the captured `Image` once. Use `AsyncComputeTaskPool` for one Bevy-compatible RGB conversion per target, crop creation, and PNG encoding; use `IoTaskPool` only for directory and temporary-file I/O. Add native `tempfile.workspace = true`; create a same-directory `NamedTempFile`, close it, and return owned `TempPath` with identity/generation/metadata. Workers never rename the destination. Intersect requested crop with the actual captured extent and fail if it becomes empty or smaller than promised. No concurrency or memory admission policy is added; same-target batching exists for Bevy correctness and work reuse.

Change existing `brp_extras/screenshot` registration to `RemoteMethodSystemId::Watching`. First invocation validates/enqueues and returns `Ok(None)`; later calls return `None` while pending, `Some(response)` after publication, or terminal `Err`. Drain worker success to `ReadyToPublish` before `RemoteSystems::ProcessRequests`; after the set, abandon unseen identities and publish only seen/unexpired generations. Verify token/generation still owns the destination, claim `ReadyToPublish -> Publishing`, call short main-thread `TempPath::persist(destination)`, then record `Completed`. Persist failure preserves the old destination and cleans the retained temp path. Stale workers never publish.

Add a named server deadline shorter than MCP's 30-second HTTP timeout. Expiration returns a terminal error but retains generation ownership until worker acknowledgement/temp cleanup. A frozen app may hit the outer timeout, which remains failure. Preserve existing full-capture response fields and add terminal status only additively. Full scope uses `Screenshot::primary_window()` with no crop. Remove MCP polling/initiation guidance.

Gate native publication under `cfg(not(target_arch = "wasm32"))`. On WASM, retain registration but immediately return actionable unsupported-publication error before creating a job. Add targeted Windows publication CI when available.

**Files:**
- `extras/Cargo.toml` — add native `tempfile` dependency.
- `extras/src/constants.rs` — capture deadline, terminal fields, and existing response compatibility.
- `extras/src/screenshot.rs` — replace the current flat handler with the anchor-type module layout.
- `extras/src/screenshot/mod.rs` — watching handler root, compatible full response, and primary-window batch dispatch.
- `extras/src/screenshot/capture/` — `CapturePlugin`, identity/state resource, destination generations, target batches, observer, compute/I/O tasks, completion channel, liveness-confirmed publication, deadlines, cleanup, and controlled tests split by ownership.
- `extras/src/screenshot/request.rs` — path/capture-ID decoding and immutable fingerprint.
- `extras/src/plugin.rs` — initialize resources, register the existing method as watching, and order lifecycle systems around `RemoteSystems::ProcessRequests` and cleanup.
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — terminal full-capture wording.
- `mcp/help_text/brp_extras_screenshot.txt` — terminal semantics without polling.
- `.github/workflows/ci.yml` — targeted Windows publication test if repository CI structure permits it.

**Constraints from prior phases:** None.

**Acceptance gate:** Controlled App tests prove repeated calls with one token are idempotent reads; distinct tokens remain isolated; same-token/different-fingerprint fails; legacy single/concurrent calls coalesce; a later legacy call starts fresh; seen/unseen tracking handles disconnect; and terminal cleanup cannot replay stale state. Workspace check and relevant nextest tests pass. Tests prove one screenshot entity and one RGB conversion per target batch; distinct jobs fan out; known RGB/crop dimensions and pixels are correct; HDR alpha is not emitted as ordinary alpha; extent changes fail; temporary ownership is returned on success/error; and same-path generations cannot be confused. Workspace check and relevant nextest tests pass. Tests prove success only after persist; absent/sentinel destinations work; persist failure preserves old content; timeout/disconnect before commit suppresses publication; post-commit disconnect may retain PNG; stale generations cannot overwrite new output; terminal errors propagate; WASM rejects before job creation; and full screenshot fields remain compatible. Workspace/WASM checks and relevant nextest tests pass, including Windows publication coverage when available. Controlled tests using Bevy's real watching-request cleanup order prove a newly ready capture waits for a subsequent live handler invocation and a disconnected caller cannot publish before the replacement commit. Identical active destination/fingerprint requests share one reservation without coupling token delivery; conflicting active fingerprints fail explicitly; sequential reuse advances generation; and stale generations cannot publish. Full clippy, workspace/WASM checks, nightly formatting, and relevant nextest tests pass without transitional dead code or lint suppressions.

#### Retrospective

**What worked:**
- Merging the original lifecycle, worker, and publication phases produced one usable, lint-clean watching method instead of dormant transitional code.
- Real `RemotePlugin` tests now cover live delivery, disconnect cleanup, one-frame publication confirmation, no replay, batching, and one RGB conversion.

**What deviated from the plan:**
- `extras/src/screenshot/capture.rs` became an ownership-based `capture/` module with `CapturePlugin`, identity, pending-state, worker-job, and RGB-image children.
- Terminal results remain as frame-stamped tombstones until Bevy stops invoking their watchers; cleanup cannot occur in the delivery frame.
- One generation-wide worker deadline applies to every watcher sharing a destination reservation; later watchers cannot revive expired work.

**Surprises:**
- Bevy invokes a closed watcher once before `RemoteSystems::Cleanup`, and a successful response send may leave the watcher registered for additional frames.
- A same-path generation must remain owned through late-worker acknowledgement even after every caller has timed out.
- The Windows replacement check needs an exact nextest selector with `--no-tests fail` so CI cannot silently run zero tests.

**Implications for remaining phases:**
- Extend the existing `capture` facade and owned child modules; do not recreate a flat screenshot pipeline or a second plugin.
- Scope fields added to `RequestFingerprint` must participate in destination-sharing and token-reuse checks.
- Entity capture must generalize the existing target batch/job path from primary-window full capture to selected targets and optional crops while preserving tombstones, worker deadlines, and generation ownership.
- Runtime tests must call the terminal method directly and never poll output paths.

### Phase 1 Review

- Collapsed the former typed-scope, camera, AABB, and method-integration phases into one lint-clean AABB vertical slice; the optional UI resolver remains a second vertical slice.
- Made the concrete `RenderTarget` to `NormalizedRenderTarget` bridge, immutable terminal metadata ownership, raw-scope fingerprints, and inherited generation deadline explicit.
- Closed `pending_screenshot_captures.rs` to request, camera, bounds, and response-construction logic; future edits are limited to the generic target/crop/metadata seam.
- Replaced the impossible fixed-image full-screenshot oracle with a full-target reference entity and added a separate isolated-port no-extras fixture for discovery.
- No user decisions remain from this review; every change is mechanical sequencing or clarification.

### Phase 2 — AABB entity capture through the existing extras method  · status: todo

#### Work Order

**Goal:** `brp_extras/screenshot` supports a lint-clean terminal entity-ID crop for AABB-backed 2D and 3D entities while preserving its existing full-screenshot contract.

**Spec:**

Parse the wire representation through a file-private raw request and convert immediately into typed state that cannot represent invalid scope:

```rust
enum ScreenshotScope {
    Full,
    Entity {
        entity: Entity,
        camera: Option<Entity>,
        padding: u32,
    },
}

struct ScreenshotRequest {
    path: PathBuf,
    capture_identity: CaptureIdentity,
    scope: ScreenshotScope,
}
```

The extras wire request has required `path`, optional bounded nonempty `capture_id`, optional `entity`, optional `camera`, and optional `padding`. Convert `capture_id` to `CaptureIdentity::Token`; omission creates `Legacy` from the normalized path and immutable fingerprint. No entity means `Full`; reject supplied camera or padding in that mode. Entity scope defaults omitted padding to `0` physical pixels. Extras never accepts or resolves a name selector.

Validate capture IDs and convert entity/camera IDs from the repository's established `u64` representation. Put raw immutable scope `(entity, requested camera, padding)` in `RequestFingerprint` before capture lookup. Scope conversion is the only code that handles the flat optional wire fields. Always call the shipped existing-identity read before resolving a possibly despawned entity, moving bounds, or camera; repeated watcher calls read stored state and never recompute a different crop. Different active scope at one destination conflicts, same-token/same-scope calls are idempotent, and later watchers joining the same generation inherit rather than extend its original deadline.

Select a screenshot-capable camera for AABB scope. A supplied camera must be active and eligible. Without one, use the only eligible active camera; zero or multiple candidates are errors, with ambiguity data `{ "reason": "ambiguous_camera", "camera_candidates": [<u64>...] }` sorted by entity ID. Eligibility requires nonempty `physical_target_size()` and `physical_viewport_rect()`, successful normalization, and a window/image/manual-texture target rather than `RenderTarget::None`. Bevy has no universal primary camera.

Resolve AABB bounds from the selected entity's `Aabb` and `GlobalTransform`, plus the selected camera, camera transform, frustum, visibility data, and render layers:

1. Generate all eight local corners from center and half extents and transform them through `GlobalTransform`.
2. Reject an oriented box outside the selected camera frustum with `Frustum::intersects_obb`.
3. Project each corner with `Camera::world_to_viewport`, then apply `Camera::target_scaling_factor()`.
4. Reject non-finite coordinates; floor minima, ceil maxima, and treat integer maxima as exclusive.
5. Apply padding with saturating integer arithmetic, then intersect with the physical camera viewport and target bounds.
6. Reject an empty crop.

After the frustum guard, any `PastNearPlane` or `PastFarPlane` corner conservatively uses the complete camera viewport. `NoViewportSize` and `InvalidData` are errors. Reject hidden entities and disjoint render layers. Where CPU visibility supplies selected-view data, use `VisibilityClass` and the selected camera's `VisibleEntities` as an additional membership check. Do not add mesh, sprite, text, or PBR dependencies to infer pipeline compatibility. For `NoCpuCulling` and custom AABB renderers, validate visibility, layers, and frustum while documenting that generic code cannot prove renderer-specific contribution. Procedural and skinned geometry must maintain suitable Bevy bounds.

Route full scope through the shipped primary-window path. Route entity scope through the selected target and `crop: Some(rect)`. Generalize the narrow capture facade to carry both the concrete `RenderTarget` used to spawn `Screenshot` and its `NormalizedRenderTarget` batching key; reject `RenderTarget::None` and keep exactly one screenshot entity per normalized target. Preserve one RGB conversion per target batch.

Snapshot immutable response metadata during the first entity resolution: entity ID, owned `Name` when present, selected camera, `BoundsKind::Aabb`, and actual padded/clipped physical rectangle. Store it on the destination reservation/job so identical-fingerprint watchers share the first snapshot, and build the additive terminal response at publication from that metadata. `request.rs` does not query `World` or read `Name`. Preserve existing success/path/working-directory/note fields and `status: "completed"`; add `capture_kind: "entity"`, `entity`, optional `name`, `camera`, `bounds_kind: "aabb"`, and `{ x, y, width, height }`.

Document that entity output is a crop of the final composited target and may include overlapping UI, geometry, background, post-processing, and occluders. Effects outside bounds require padding. Resolve only the selected entity, never descendants. Do not register a second BRP method and do not repeat already-shipped watching registration, full routing, terminal status, deadline propagation, or no-polling implementation.

**Files:**
- `extras/src/constants.rs` — optional entity/camera/padding request fields, zero default, and additive entity response fields.
- `extras/src/screenshot/request.rs` — raw request, ID conversion, fingerprint input, typed scope, and tests.
- `extras/src/screenshot/mod.rs` — existing-state fast path, camera selection, AABB dispatch, immutable metadata snapshot, and response DTO ownership.
- `extras/src/screenshot/aabb.rs` — AABB resolver, physical containing conversion, selected-view visibility/layer validation, and tests.
- `extras/src/screenshot/capture/identity.rs` — add raw immutable entity scope to `RequestFingerprint`.
- `extras/src/screenshot/capture/mod.rs` — extend the narrow facade with concrete/normalized targets, crop, and metadata.
- `extras/src/screenshot/capture/screenshot_job.rs` — carry crop and immutable response metadata through the existing worker path.
- `extras/src/screenshot/capture/pending_screenshot_captures.rs` — only the generic target/crop/metadata reservation seam; no request decoding, camera queries, bounds logic, or response DTO construction.
- `extras/src/lib.rs` — consolidated screenshot method docs.
- `extras/README.md` — full/AABB entity API, limitations, and terminal semantics.
- `extras/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phase 1 owns `CapturePlugin`, frame-stamped terminal tombstones, the one-frame `ReadyToPublish` liveness confirmation, generation-wide deadlines, destination fingerprint sharing/conflicts, target batching, generation-checked `TempPath::persist`, and stale-worker acknowledgement/ownership. Preserve all of them. Do not add feature logic to the 1,651-line `pending_screenshot_captures.rs`; limit edits to its generic seam, and if it grows materially move inline tests to a focused child test module. Do not add another observer, handler, plugin, or flat capture module.

**Acceptance gate:** Tests cover typed full/entity conversion, zero padding, invalid field combinations, proof extras has no name selector, same-token scope reuse, distinct-scope destination conflicts, and inherited generation deadlines. Camera tests cover explicit, single inferred, zero/multiple candidates with stable data, inactive/uninitialized/unsupported targets, and `RenderTarget::None`. AABB tests cover translated, rotated, non-uniformly scaled, and reflected boxes; logical/physical scaling; viewport offsets; fractional/negative extrema; hard-edge padding; empty/offscreen output; near/far conservative fallback; hidden entities; layers; selected-view membership; `NoCpuCulling`; and custom-renderer limitations. Method tests prove legacy full behavior, terminal entity metadata including snapshotted name, moving/despawned inputs after enqueue, one concrete screenshot entity and RGB conversion per normalized target, shared full/entity capture, deferred errors, and absence of a screenshot-entity method. `cargo check -p bevy_brp_extras --no-default-features`, workspace checks, relevant nextest tests, full clippy, and nightly formatting pass without dormant code or lint suppressions.

### Phase 3 — Optional UI entity bounds  · status: todo

#### Work Order

**Goal:** Default builds extend the existing entity screenshot path to transformed and clipped Bevy UI nodes, while minimal builds retain AABB screenshots without Bevy UI.

**Spec:**

Add `ui = ["bevy/bevy_ui"]` to `bevy_brp_extras` and include it in default features beside `diagnostics`. Gate all UI imports, queries, and resolver code. Bevy 0.19 UI necessarily enables its required text/sprite dependency family; add no separate textual snapshot behavior or direct text/PBR feature. With UI disabled, a non-AABB entity returns an unsupported-bounds error naming disabled UI support as a possible cause.

For a complete UI component family, use UI precedence over an incidental AABB. A partial family returns an uninitialized-UI error rather than falling through. Read `ComputedNode`, `UiGlobalTransform`, `ComputedUiTargetCamera`, `ComputedUiRenderTargetInfo`, optional `CalculatedClip`, and inherited visibility:

1. Build four local corners from `ComputedNode::size()` around the node origin and transform them with `UiGlobalTransform::affine()`.
2. Take axis-aligned physical min/max. UI transform and clip coordinates are camera-viewport-local; do not scale again.
3. Intersect locally with `CalculatedClip::clip` and zero through render-target-info physical size.
4. Add `Camera::physical_viewport_rect().min` to enter target-space pixels.
5. Reject non-finite values; floor minima, ceil maxima, and treat maxima as exclusive.
6. Apply zero-default padding, then intersect with translated clip, physical camera viewport, and target bounds.
7. Reject an empty crop.

Use `ComputedUiTargetCamera` and its concrete screenshot-capable target, including non-primary windows and images; reject a different explicitly supplied camera. Reject UI according to `InheritedVisibility`. Do not apply AABB `RenderLayers` rules to UI. Submit the concrete and normalized target, crop, and immutable metadata through the Phase 2 facade. Snapshot `BoundsKind::Ui` and the final rectangle for terminal response construction.

**Files:**
- `extras/Cargo.toml` — default-enabled optional UI feature.
- `extras/src/screenshot/mod.rs` — UI precedence/dispatch and terminal metadata regression coverage.
- `extras/src/screenshot/ui.rs` — gated UI resolver, target selection, precedence, errors, and tests.
- `extras/src/lib.rs` — UI feature and bounds documentation.
- `extras/README.md` — UI semantics and no-default behavior.
- `extras/CHANGELOG.md` — UI entity-capture entry.

**Constraints from prior phases:** Phase 1 owns `CapturePlugin`, frame-stamped terminal tombstones, one-frame `ReadyToPublish` confirmation, generation-wide deadlines, destination fingerprint sharing/conflicts, generation-checked publication, and stale-worker ownership. Phase 2 owns request/camera/response logic plus the only generic concrete-target/crop/metadata seam. Read existing identities before UI resolution. Do not add request decoding, camera queries, UI logic, or response construction to `pending_screenshot_captures.rs`; do not add a second observer or publication path.

**Acceptance gate:** Tests cover translation, scaling, rotation, fractional extrema, partial initialization, UI-over-AABB precedence, clipping, nonzero viewport origin, target translation/edges, zero and padded hard bounds, camera mismatch, non-primary image/window targets, hidden/offscreen UI, disjoint `RenderLayers` not affecting UI, and empty output. Method regression tests prove immutable UI metadata and the Phase 1/2 terminal lifecycle remain unchanged. Default and no-default workspace checks, relevant nextest tests, full clippy, and nightly formatting pass.

### Phase 4 — Generic MCP entity-name discovery  · status: todo

#### Work Order

**Goal:** `world_find_entities_by_name` discovers entity IDs through standard BRP without requiring `bevy_brp_extras`.

**Spec:**

Implement a local MCP composite tool using existing `BrpClient`/`ToolHandler` patterns. Call standard `world.query`, requiring and retrieving `bevy_ecs::name::Name`, then parse/filter locally. Request fields are `name`, typed `match_mode`, and `port`; modes are `exact`, `prefix`, `suffix`, and `contains`, defaulting to exact. Match case-sensitively and treat `*` literally. Return `{ entity: u64, name: String }` entries sorted by entity ID. Expose the internal query/parse/match operation so the screenshot MCP handler can reuse unique exact resolution without making an MCP sub-tool call.

Register schema, local routing, read-only annotations, help, README, and changelog. The target app needs standard BRP and reflected `Name`, never the extras plugin. Explain non-exact discovery followed by canonical-ID operations.

**Files:**
- `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — parameters, match enum, standard query composition, response, shared exact resolver, and tests.
- `mcp/src/brp_tools/tools/mod.rs` — declaration/re-export.
- `mcp/src/brp_tools/mod.rs` — facade re-export.
- `mcp/src/tool/name.rs` — static local tool registration, parameters, annotation, and routing.
- `mcp/help_text/world_find_entities_by_name.txt` — match modes and output.
- `mcp/README.md` — generic tool documentation.
- `mcp/CHANGELOG.md` — MCP-only entry.

**Constraints from prior phases:** Extras accepts entity IDs only. This tool must use standard `world.query` and remain usable in applications without `bevy_brp_extras`.

**Acceptance gate:** Tests prove standard query parameters, operation without extras discovery, exact/prefix/suffix/contains, default exact, case sensitivity, literal asterisk, stable entity-ID ordering, no matches, duplicates, malformed BRP data, and clear BRP errors. Registry/schema/help tests pass with workspace check and relevant nextest tests.

### Phase 5 — Optional entity/name scope on the existing MCP screenshot tool  · status: todo

#### Work Order

**Goal:** A user can make one `brp_extras_screenshot` call for a full image, an entity ID, or a unique exact name such as `NatesList`.

**Spec:**

Extend existing `ScreenshotParams` with optional `entity`, optional `name`, optional `camera`, optional `padding`, existing required `path`, and `port`. Convert the MCP request to typed scope immediately:

- neither entity nor name: full capture; reject camera or padding;
- entity only: send entity/camera/zero-default padding to extras;
- name only: reuse Phase 4 exact case-sensitive standard-BRP lookup, require exactly one match, then send the resolved ID to extras;
- both entity and name: invalid parameters;
- zero matches: actionable error;
- multiple matches: error with stable matching IDs and instruction to retry using `entity` or generic discovery.

Implement the existing screenshot tool as a concrete local composite. Change the `ToolName::BrpExtrasScreenshot` attribute to keep only `brp_method = "brp_extras/screenshot"` without generated parameter/result fields; define the `BrpExtrasScreenshot` marker and its concrete `ToolFn` implementation in `brp_extras_screenshot.rs`. Keep `ScreenshotParams` as the MCP wire type, convert it to a private full/ID/exact-name enum, use `BrpClient` for `world.query` when required, generate a fresh UUID v4 capture ID for every screenshot invocation, then call `BrpMethod::BrpExtrasScreenshot`. Extras receives only optional entity ID and the hidden capture token, never a name. Apply the repository's corresponding local-marker pattern to `WorldFindEntitiesByName`, which has no BRP method mapping. Preserve direct BRP payload data in the recognized `result: Option<Value>` path rather than duplicating typed fields inconsistently. The tool waits for terminal extras completion and returns resolved ID/name metadata with the final PNG result.

Update the existing help text with one-call examples for full, ID, and `name: "NatesList"`; explain exact uniqueness, generic discovery for non-exact/duplicates, zero padding, optional camera inference, no universal primary camera, optional UI support, AABB support, terminal completion, and composited crop semantics. Update MCP README/changelog. Do not create a screenshot-entity MCP module/tool/help file.

**Files:**
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — expanded params, typed scope, optional standard-BRP name resolution, extras call, and tests.
- `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — shared exact resolver surface if Phase 4 structure requires adjustment.
- `mcp/src/tool/name.rs` — preserve the existing screenshot tool identity while routing its composite handler.
- `mcp/help_text/brp_extras_screenshot.txt` — consolidated examples/semantics.
- `mcp/README.md` — consolidated screenshot documentation.
- `mcp/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phases 2–3 expose one terminal extras method with optional entity ID and AABB/UI bounds; Phase 4 exposes reusable generic name lookup. Phase 1's server deadline is 25 seconds, terminal BRP results retain frame-stamped tombstones until Bevy removes the watcher, later same-generation callers inherit the original deadline, and no caller polls the path. Names terminate in MCP; full and entity requests retain one existing MCP tool name, and every MCP invocation uses a fresh capture token.

**Acceptance gate:** Tests cover full, direct ID, unique exact name, no name match, duplicate IDs, invalid both selectors, invalid camera/padding on full capture, zero padding, final extras request never containing name, terminal result preservation, and registry proof that no screenshot-entity tool exists. Help/README show “screenshot NatesList” as one call. Workspace check and relevant nextest tests pass.

### Phase 6 — Runtime fixtures and PNG assertion tooling  · status: todo

#### Work Order

**Goal:** Deterministic Bevy fixtures and an authorized read-only PNG helper make exact full/entity capture verification executable.

**Spec:**

Add stable named fixtures: fixed-size UI; rotated/clipped UI; transformed 2D AABB with `Camera2d`; transformed 3D AABB with `Camera3d`; unsupported entity; duplicate/unique/unnamed names including `NatesList`; explicit nonzero viewport; hidden entities; disjoint render layers; and distinctive interior/edge colors. Render exact-coordinate cases to a fixed-size image target. Add a named reference entity whose bounds cover that entire image target; compare smaller entity crops against this reference entity's PNG because full scope intentionally captures only the primary window. Retain a separate primary-window full-capture smoke fixture. Keep the second camera inactive except in the ambiguity case so it does not invalidate ordinary inferred-camera cases.

Update `test-app/examples/no_extras_plugin.rs` to accept the runner-provided isolated BRP port rather than hard-coding port 25000. Configure it as a second runtime fixture for extras-independent discovery. It enables standard BRP and reflected `Name`, but never `bevy_brp_extras`.

Add a read-only repository helper that parses a complete PNG, reports dimensions, detects uniform images, checks fixture-marker pixels, and compares every entity-crop pixel with the corresponding rectangle from the full-target reference entity PNG. Authorize only this helper in `integration-tester.md`; it must not call BRP. Assert returned rectangles against known fixed-target coordinates. Keep same-target concurrency, deadlines, disconnects, completion/timeout contention, and late-worker suppression in deterministic Phase 1/2 nextest/App tests.

**Files:**
- `test-app/examples/extras_plugin.rs` — deterministic fixtures.
- `test-app/examples/no_extras_plugin.rs` — runner-provided isolated port and standard-BRP-only named fixtures.
- `.claude/scripts/integration_tests/extras_assert_png.py` — read-only dimensions/uniformity/marker/exact-crop comparison helper.
- `.claude/scripts/integration_tests/test_extras_assert_png.py` — unit tests for valid, malformed, incomplete, uniform, marker-mismatch, and crop-mismatch inputs.
- `.claude/agents/integration-tester.md` — authorize generic find and the PNG assertion helper while retaining screenshot access.
- `.claude/config/integration_tests.json` — configure extras and no-extras applications with isolated ports for the runtime specification.

**Constraints from prior phases:** Phases 1–3 provide terminal full/entity capture through the existing method and ownership-based capture modules; Phases 4–5 provide generic discovery and one-call named scope. The helper is read-only, never calls BRP, and consumes only paths returned after terminal completion. Image-target identity uses the full-target reference entity, never an unavailable arbitrary-target full screenshot.

**Acceptance gate:** Fixture setup is deterministic at fixed target size; expected rectangles/colors are documented; helper tests prove dimensions, uniform detection, marker checks, exact crop equality, malformed/incomplete PNG failure, and nonzero exit on mismatch; integration-tester authorization permits only the helper and required MCP tools. Workspace check and relevant nextest/helper tests pass.

### Phase 7 — MCP runtime integration execution  · status: todo

#### Work Order

**Goal:** Real MCP integration proves terminal full/UI/2D/3D screenshots, one-call named scope, camera behavior, RGB crop identity, and extras-independent discovery.

**Spec:**

Extend `.claude/integration_tests/extras_capture.md` for primary-window full terminal capture; one-call `name: "NatesList"`; direct ID; UI/2D/3D; generic non-exact discovery then ID; duplicate ambiguity; unsupported/uninitialized entity; explicit/ambiguous camera; zero/default/padded bounds; offset viewport; clipping; and write failure. Use unique destinations, remove old files, prove final paths are absent before calls and complete on return, validate terminal fields/resolved ID/rect, and run the Phase 6 helper for dimensions, nonuniformity, marker pixels, and exact crop equality against the fixed-target reference entity. Never poll screenshot paths.

Launch the Phase 6 no-extras fixture on its isolated port before the discovery check. Call `world_find_entities_by_name` against that port and verify it succeeds, then verify `brp_extras_screenshot` is unavailable there. Keep the ambiguity camera inactive until its dedicated case.

Retire/repurpose the polling helper. Introspection proves existing `brp_extras/screenshot` remains and neither screenshot-entity nor extras name methods exist. Keep existing runner registrations unless fixture invocation requires change. Install the MCP binary, restart the MCP client session, use isolated ports, and remove artifacts.

**Files:**
- `.claude/integration_tests/extras_capture.md` — runtime cases and assertions.
- `.claude/integration_tests/introspection.md` — consolidated BRP discovery assertions.
- `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` — retire/repurpose obsolete polling.
- `.claude/config/integration_tests.json` — verify registrations; edit only if necessary.

**Constraints from prior phases:** Phase 6 provides deterministic extras/no-extras fixtures and the PNG helper; Phases 1–5 provide the final terminal APIs. Phase 1 already covers same-target concurrency, frame-stamped delivery tombstones, one-frame publication confirmation, generation-wide deadlines, destination sharing/conflicts, real watcher disconnects, stale-worker ownership, and exact Windows replacement selection; keep those as deterministic App/nextest tests rather than duplicating them in MCP integration. Runtime cases never poll screenshot paths.

**Acceptance gate:** Run the full `clippy` skill, workspace/default/no-default/WASM checks, `cargo nextest run --workspace`, and `/test extras_capture,introspection` after MCP reinstall/restart. Tests prove full and `NatesList` captures, direct ID, UI/2D/3D exact crop identity, terminal failures, camera ambiguity, zero padding, generic discovery without extras, absence of new BRP/MCP variants, authorization, and cleanup.
