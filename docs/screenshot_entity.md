# Entity-scoped screenshots

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Extends the existing screenshot operation with optional entity/name scope while making full and cropped PNG delivery terminal and reliable.

## Delegation Context

- **Project:** `bevy_brp` workspace — `bevy_brp_extras` supplies extra Bevy Remote Protocol methods, `bevy_brp_mcp` exposes them as MCP tools, and `bevy_brp_test_apps` supplies runtime fixtures.
- **Stack:** Rust 2024 edition (`rustc 1.97.0`; formatting toolchain `rustc 1.98.0-nightly`), Bevy `0.19.0`, `bevy_remote 0.19.0`, workspace crates `0.21.0-dev`, `rmcp 1.7.0`, Serde/JSON, Schemars, and Bevy render/UI/camera APIs.
- **Layout:** `extras/` — BRP implementation, feature gates, docs, and unit tests; `mcp/` — typed MCP facade, local composite name resolution, registry/help, docs, and changelog; `test-app/` — deterministic Bevy fixtures; `.claude/integration_tests/`, `.claude/scripts/integration_tests/`, `.claude/agents/` — runtime specifications, screenshot helpers, and tester authorization.
- **Key files:**
  - `Cargo.toml` — workspace dependency versions and lint policy.
  - `extras/Cargo.toml` — add default-enabled `ui = ["bevy/bevy_ui"]` while retaining the AABB resolver/API without default features; the flag gates this crate's UI capability, while upstream Bevy 0.19 `bevy_remote` still brings the UI dependency family transitively.
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
- **Invariants:** Use `cargo +nightly fmt --all -- --check`, never plain `cargo fmt`; extend the existing `brp_extras/screenshot` BRP method and `brp_extras_screenshot` MCP tool rather than creating screenshot-entity variants; both full and entity captures are internally asynchronous but terminal to BRP/MCP callers; neither success nor the final path is published before a complete PNG; extras accepts only optional entity ID scope, while MCP accepts optional entity ID or unique exact name and resolves names through standard `world.query`; generic name discovery is MCP-only and never requires extras; neither entity nor name means full primary-window capture, both is invalid, and camera/padding are invalid without entity scope; padding defaults to zero physical pixels; UI support is default-enabled but optional at the `bevy_brp_extras` capability/API layer, and `--no-default-features` retains generic AABB screenshots without compiling this crate's UI resolver or imports; this is not a promise to remove the UI dependency family from `cargo tree`, because upstream Bevy 0.19 `bevy_remote` already brings it transitively; enabling UI necessarily enables the Bevy 0.19 text/sprite dependencies required by Bevy UI, but this feature adds no direct textual-snapshot scope; complete UI components take precedence over AABB and partial UI returns an initialization error; crops use the selected camera target and physical target coordinates, honor viewport/clip/target hard bounds, and contain partially covered pixels; output is a crop of the final composited target, not isolated rendering; coordinate one Bevy screenshot entity per normalized target, fan out same-target jobs, convert the complete image once using Bevy-compatible RGB semantics, reserve destination paths by generation through worker acknowledgement, and commit completed temporary files only after verifying current ownership; name matches are case-sensitive and deterministically ordered by entity ID, with explicit match modes rather than wildcard syntax; keep direct BRP payloads in `result: Option<Value>` for `ResultStruct`; `snapshot`/textual UI-tree inspection is separate scope and must not be added here; do not add lint suppressions, generic helper modules, speculative caches, benchmark infrastructure, or single-implementation traits.

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

### Phase 2 — AABB entity capture through the existing extras method  · status: done (`f37a0211`)

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
- `extras/src/screenshot/capture/pending_screenshot_captures/mod.rs` — only the generic target/crop/metadata reservation seam and capture-stage cancellation; no request decoding, camera queries, bounds logic, or response DTO construction.
- `extras/src/screenshot/capture/pending_screenshot_captures/tests.rs` — ownership-focused lifecycle and entity-capture tests split from the production module.
- `extras/src/lib.rs` — consolidated screenshot method docs.
- `extras/README.md` — full/AABB entity API, limitations, and terminal semantics.
- `extras/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phase 1 owns `CapturePlugin`, frame-stamped terminal tombstones, the one-frame `ReadyToPublish` liveness confirmation, generation-wide deadlines, destination fingerprint sharing/conflicts, target batching, generation-checked `TempPath::persist`, and stale-worker acknowledgement/ownership. Preserve all of them. Do not add feature logic to the 1,651-line `pending_screenshot_captures.rs`; limit edits to its generic seam, and if it grows materially move inline tests to a focused child test module. Do not add another observer, handler, plugin, or flat capture module.

**Acceptance gate:** Tests cover typed full/entity conversion, zero padding, invalid field combinations, proof extras has no name selector, same-token scope reuse, distinct-scope destination conflicts, and inherited generation deadlines. Camera tests cover explicit, single inferred, zero/multiple candidates with stable data, inactive/uninitialized/unsupported targets, and `RenderTarget::None`. AABB tests cover translated, rotated, non-uniformly scaled, and reflected boxes; logical/physical scaling; viewport offsets; fractional/negative extrema; hard-edge padding; empty/offscreen output; near/far conservative fallback; hidden entities; layers; selected-view membership; `NoCpuCulling`; and custom-renderer limitations. Method tests prove legacy full behavior, terminal entity metadata including snapshotted name, moving/despawned inputs after enqueue, one concrete screenshot entity and RGB conversion per normalized target, shared full/entity capture, deferred errors, and absence of a screenshot-entity method. `cargo check -p bevy_brp_extras --no-default-features`, workspace checks, relevant nextest tests, full clippy, and nightly formatting pass without dormant code or lint suppressions.

#### Retrospective

**What worked:**
- The AABB entity path shipped as one lint-clean vertical slice: typed request, camera selection, bounds resolution, generic target/crop submission, immutable terminal metadata, docs, and method-level tests are all production-used.
- Existing-identity and matching-reservation reads happen before entity/camera/bounds resolution, so moving or despawned inputs cannot change an accepted request.
- The capture facade now supports concrete and normalized targets while retaining one screenshot entity and RGB conversion per target batch.

**What deviated from the plan:**
- Live target eligibility checks `Window`, `Assets<Image>`, render-world usage, and `ManualTextureViews`; cached `Camera::physical_target_size()` alone was insufficient.
- Capture-stage batches now own their spawned screenshot entity and can cancel/acknowledge a never-captured generation at deadline. Once a batch enters encoding, the existing late-worker acknowledgement rule still owns cleanup.
- `pending_screenshot_captures.rs` became directory-form `pending_screenshot_captures/mod.rs`, with its large lifecycle tests moved to `tests.rs`. Test-only manual texture resources use a dev-only `wgpu` noop device.

**Surprises:**
- `Entity::from_bits` can panic for invalid remote bit patterns; request decoding must use `Entity::try_from_bits` and return field-specific parameter errors.
- Bevy may retain cached camera target size after an image/manual target disappears, while its screenshot system produces no completion event.
- A valid render-target image may be `RENDER_WORLD`-only; requiring main-world asset usage would reject a supported target.

**Implications for remaining phases:**
- UI target selection must reuse the live concrete/normalized target validation and the single capture facade; it must not trust cached camera target dimensions or create a UI-specific observer.
- UI metadata must be snapshotted before submission and carried through the existing reservation/job response seam.
- Runtime fixtures should use real image/manual target resources and terminal publication paths rather than fabricated camera computed state.
- Capture-stage cancellation and encoding-stage late acknowledgement are now separate invariants and must remain separate.

### Phase 2 Review

- Required Phase 3 to classify complete/partial UI component families before generic AABB camera inference, so `ComputedUiTargetCamera` owns UI selection without false world-camera ambiguity.
- Required UI bounds to pass through Phase 2's live target validation and intersect both the live target extent and physical viewport before capture submission.
- Made Phase 4's internal standard-BRP `world.query` request/response path explicit for a fresh delegate.
- Required Phase 6 image/manual fixtures to retain handles/backing resources for the full runtime test and use one validated target for reference and cropped entities.
- No user decisions or phase resequencing remain.

### Phase 3 — Optional UI entity bounds  · status: done (`053ac9b4`)

#### Work Order

**Goal:** Default builds extend the existing entity screenshot path to transformed and clipped Bevy UI nodes, while no-default builds retain the AABB capability without compiling this crate's UI resolver/API.

**Spec:**

Add `ui = ["bevy/bevy_ui"]` to `bevy_brp_extras` and include it in default features beside `diagnostics`. Gate all UI imports, queries, resolver code, and public capability behavior. This is a capability/API gate, not a guarantee that UI crates disappear from the resolved dependency graph: upstream Bevy 0.19 `bevy_remote` already brings the UI dependency family transitively through `bevy_dev_tools`. Add no separate textual snapshot behavior or direct text/PBR feature. With UI disabled, a non-AABB entity returns an unsupported-bounds error naming disabled UI support as a possible cause.

Classify the selected entity's UI component family before invoking generic AABB camera inference. Define complete/partial UI state from UI-specific components (`ComputedNode`, `UiGlobalTransform`, `ComputedUiTargetCamera`, and `ComputedUiRenderTargetInfo`), not generic `InheritedVisibility`, which AABB entities may also carry. A complete UI family uses UI precedence over an incidental AABB; a partial family returns an uninitialized-UI error rather than falling through. Read the complete family, optional `CalculatedClip`, and inherited visibility:

1. Build four local corners from `ComputedNode::size()` around the node origin and transform them with `UiGlobalTransform::affine()`.
2. Take axis-aligned physical min/max. UI transform and clip coordinates are camera-viewport-local; do not scale again.
3. Intersect locally with `CalculatedClip::clip` and zero through render-target-info physical size.
4. Add `Camera::physical_viewport_rect().min` to enter target-space pixels.
5. Reject non-finite values; floor minima, ceil maxima, and treat maxima as exclusive.
6. Apply zero-default padding, then intersect with translated clip, physical camera viewport, and target bounds.
7. Reject an empty crop.

Use `ComputedUiTargetCamera` and reject a different explicitly supplied camera. Pass that computed camera through Phase 2's live `Window`/`Assets<Image>`/`ManualTextureViews` target validation; do not trust cached physical target size. Intersect viewport-local UI bounds with both the live target extent and physical camera viewport before submission. Reject UI according to `InheritedVisibility`. Do not apply AABB `RenderLayers` rules to UI. Submit the validated concrete and normalized target, crop, and immutable metadata through the Phase 2 facade. Snapshot `BoundsKind::Ui` and the final rectangle for terminal response construction.

**Files:**
- `extras/Cargo.toml` — default-enabled optional UI feature.
- `extras/src/screenshot/mod.rs` — UI precedence/dispatch and terminal metadata regression coverage.
- `extras/src/screenshot/ui.rs` — gated UI resolver, target selection, precedence, errors, and tests.
- `extras/src/lib.rs` — UI feature and bounds documentation.
- `extras/README.md` — UI semantics and no-default behavior.
- `extras/CHANGELOG.md` — UI entity-capture entry.

**Constraints from prior phases:** Phase 1 owns `CapturePlugin`, frame-stamped terminal tombstones, one-frame `ReadyToPublish` confirmation, generation-wide deadlines, destination fingerprint sharing/conflicts, generation-checked publication, and stale-worker ownership. Phase 2 owns request/camera/response logic plus the only generic concrete-target/crop/metadata seam. Read existing identities before UI resolution. Do not add request decoding, camera queries, UI logic, or response construction to `pending_screenshot_captures.rs`; do not add a second observer or publication path.

**Acceptance gate:** Tests cover translation, scaling, rotation, fractional extrema, partial initialization, UI-over-AABB precedence, clipping, nonzero viewport origin, target translation/edges, zero and padded hard bounds, camera mismatch, non-primary image/window targets, hidden/offscreen UI, disjoint `RenderLayers` not affecting UI, and empty output. Method regression tests prove immutable UI metadata and the Phase 1/2 terminal lifecycle remain unchanged. Default and no-default workspace checks, relevant nextest tests, full clippy, and nightly formatting pass.

#### Retrospective

**What worked:**
- The UI extension stayed within a focused feature-gated resolver and dispatches before AABB camera inference, so `ComputedUiTargetCamera` owns UI camera selection without introducing another capture path.
- Transformed and clipped viewport-local UI bounds reuse Phase 2's live target validation, hard bounds, capture facade, and immutable terminal metadata.
- Default, no-default, focused UI, full extras, and workspace test suites all pass without changing Phase 1's capture lifecycle.

**What deviated from the plan:**
- Phase 2's camera validation was split into reusable live-target validation plus AABB-specific camera state so UI can validate its computed camera without requiring a frustum or world transform.
- The `ui` feature gates this crate's resolver, imports, and capability rather than guaranteeing dependency-graph removal. Upstream Bevy 0.19 `bevy_remote` already pulls the UI dependency family transitively through `bevy_dev_tools`.

**Surprises:**
- A complete UI entity must carry coherent computed target-camera and render-target information; constructing useful resolver tests required the same computed resource family Bevy produces during UI layout.
- Disabling `bevy_remote` default features does not remove the upstream `bevy_dev_tools` default-feature dependency family, so a local capability feature cannot provide dependency pruning by itself.
- Blind review exposed two malformed-state edges: renderer-required `InheritedVisibility` must be present, and non-finite `CalculatedClip` coordinates must be rejected before rectangle intersection can normalize them into a plausible crop.

**Implications for remaining phases:**
- MCP help and README wording must describe optional UI screenshot capability without promising that `--no-default-features` removes UI crates from `cargo tree`.
- Name discovery remains independent of extras and UI; the screenshot composite resolves a name to an entity ID before calling the existing extras method.
- Runtime UI fixtures must exercise real computed UI state and live camera targets, then verify the same terminal publication and crop path used by AABB captures.

### Phase 3 Review

- Kept Phases 4–7 in their existing order; none became redundant after the UI vertical slice.
- Required Phase 5 help and README text to carry the settled capability-gate contract without claiming transitive dependency pruning.
- Added a distinct partial/uninitialized UI fixture to Phase 6 and required real Bevy UI layout/propagation before capture.
- Required Phase 7 to prove UI resolver metadata and to distinguish MCP tool registration from an extras BRP method-not-found response on the no-extras fixture.
- No user decisions or phase resequencing remain.

### Phase 4 — Generic MCP entity-name discovery  · status: done (`61061216`)

#### Work Order

**Goal:** `world_find_entities_by_name` discovers entity IDs through standard BRP without requiring `bevy_brp_extras`.

**Spec:**

Implement a local MCP composite tool using existing `BrpClient`/`ToolHandler` patterns. Build the internal standard-BRP `world.query` request with `bevy_ecs::name::Name` in both required and returned components, execute it directly through `BrpClient`/`BrpMethod::WorldQuery`, parse successful raw entity/component rows locally, and propagate raw BRP errors. Do not call the generated `WorldQuery` MCP wrapper or make an MCP sub-tool call. Request fields are `name`, typed `match_mode`, and `port`; modes are `exact`, `prefix`, `suffix`, and `contains`, defaulting to exact. Match case-sensitively and treat `*` literally. Return `{ entity: u64, name: String }` entries sorted by entity ID. Expose the internal query/parse/match operation so the screenshot MCP handler can reuse unique exact resolution.

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

#### Retrospective

**What worked:**
- The generic finder is an MCP-local tool with no BRP method mapping and composes standard `world.query` directly through `BrpClient`, so applications need reflected `Name` but never `bevy_brp_extras`.
- Typed exact/prefix/suffix/contains matching stays case-sensitive, treats asterisks literally, preserves duplicate matches, and sorts canonical entity IDs before returning them.
- Static registration, parameters, read-only annotations, help, README, changelog, raw response decoding, and structured BRP errors landed together with focused and workspace coverage.

**What deviated from the plan:**
- No product or architecture behavior deviated. The implementation uses private minimal query wire structs instead of the public generated `world_query` parameter type, which keeps the local composite independent of the MCP wrapper while producing the specified BRP shape.

**Surprises:**
- The local `ToolName` marker needs no `brp_tool` mapping; explicit parameter registration and `ToolFn` routing make `to_brp_method()` correctly return `None` while retaining a normal MCP schema.
- The style pass required repeated domain-specific test names and entity IDs to move into test-module constants before the final lint gate.

**Implications for remaining phases:**
- Phase 5 should reuse the shipped `pub(super)` async lookup with `NameMatchMode::Exact`, then enforce zero/one/many semantics without changing the generic finder's duplicate-preserving behavior.
- The name resolver already returns sorted canonical IDs and structured query/decode failures; the screenshot composite should preserve those errors rather than issuing another query or calling an MCP sub-tool.
- Runtime no-extras coverage must distinguish this registered local MCP tool from the absent extras BRP screenshot method.

### Phase 4 Review

- Pinned Phase 5 to `find_entities_by_name(&name, NameMatchMode::Exact, port)` and preserved the shipped sorted duplicates plus structured query/decode errors.
- Added the module/facade re-exports Phase 5 needs when the existing screenshot tool becomes a concrete local marker; Phase 4's finder marker is already complete.
- Made the runtime specification runner-managed with labeled extras and no-extras apps on isolated ports rather than launching the second app inside the test.
- Kept MCP registry proof in Phase 5 and narrowed Phase 7 introspection to BRP method discovery, which is the boundary `rpc.discover` can actually prove.
- No user decisions or phase resequencing remain.

### Phase 5 — Optional entity/name scope on the existing MCP screenshot tool  · status: done (`f0371ba9`)

#### Work Order

**Goal:** A user can make one `brp_extras_screenshot` call for a full image, an entity ID, or a unique exact name such as `NatesList`.

**Spec:**

Extend existing `ScreenshotParams` with optional `entity`, optional `name`, optional `camera`, optional `padding`, existing required `path`, and `port`. Convert the MCP request to typed scope immediately:

- neither entity nor name: full capture; reject camera or padding;
- entity only: send entity/camera/zero-default padding to extras;
- name only: call `find_entities_by_name(&name, NameMatchMode::Exact, port)`, preserve its sorted duplicates and structured query/decode errors, require exactly one match, then send the resolved ID to extras;
- both entity and name: invalid parameters;
- zero matches: actionable error;
- multiple matches: error with stable matching IDs and instruction to retry using `entity` or generic discovery.

Implement the existing screenshot tool as a concrete local composite. Change the `ToolName::BrpExtrasScreenshot` attribute to keep only `brp_method = "brp_extras/screenshot"` without generated parameter/result fields; define the `BrpExtrasScreenshot` marker and its concrete `ToolFn` implementation in `brp_extras_screenshot.rs`, then re-export that marker through the tools module and BRP facade for `tool/name.rs`. Keep `ScreenshotParams` as the MCP wire type, convert it to a private full/ID/exact-name enum, call the shipped Phase 4 lookup when name resolution is required, generate a fresh UUID v4 capture ID for every screenshot invocation, then call `BrpMethod::BrpExtrasScreenshot`. Extras receives only optional entity ID and the hidden capture token, never a name. `WorldFindEntitiesByName` already follows the local-marker pattern and must not be reworked. Preserve direct BRP payload data in the recognized `result: Option<Value>` path rather than duplicating typed fields inconsistently. The tool waits for terminal extras completion and returns resolved ID/name metadata with the final PNG result.

Update the existing help text with one-call examples for full, ID, and `name: "NatesList"`; explain exact uniqueness, generic discovery for non-exact/duplicates, zero padding, optional camera inference, no universal primary camera, optional UI support, AABB support, terminal completion, and composited crop semantics. Describe `ui` as the extras UI resolver/import/capability gate, not a guarantee that upstream UI crates disappear from `cargo tree`. Update MCP README/changelog. Do not create a screenshot-entity MCP module/tool/help file.

**Files:**
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — expanded params, typed scope, optional standard-BRP name resolution, extras call, and tests.
- `mcp/src/brp_tools/tools/mod.rs` — re-export the concrete screenshot marker while preserving the shipped finder exports.
- `mcp/src/brp_tools/mod.rs` — facade re-export for screenshot routing.
- `mcp/src/tool/name.rs` — preserve the existing screenshot tool identity while routing its composite handler.
- `mcp/help_text/brp_extras_screenshot.txt` — consolidated examples/semantics.
- `mcp/README.md` — consolidated screenshot documentation.
- `mcp/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phases 2–3 expose one terminal extras method with optional entity ID and AABB/UI bounds; Phase 4 exposes `find_entities_by_name(&name, NameMatchMode::Exact, port)` with sorted duplicate-preserving results and structured query/decode errors. Reuse that seam and add only zero/one/many screenshot semantics. Phase 1's server deadline is 25 seconds, terminal BRP results retain frame-stamped tombstones until Bevy removes the watcher, later same-generation callers inherit the original deadline, and no caller polls the path. Names terminate in MCP; full and entity requests retain one existing MCP tool name, and every MCP invocation uses a fresh capture token.

**Acceptance gate:** Tests cover full, direct ID, unique exact name, no name match, duplicate IDs, invalid both selectors, invalid camera/padding on full capture, zero padding, final extras request never containing name, terminal result preservation, and registry proof that no screenshot-entity tool exists. Help/README show “screenshot NatesList” as one call. Workspace check and relevant nextest tests pass.

#### Retrospective

**What worked:**
- The existing `brp_extras_screenshot` identity now routes through one typed MCP-local composite for full, entity-ID, and unique exact-name capture without adding another tool or extras method.
- Exact-name capture reuses Phase 4's standard-BRP lookup, then sends only the canonical entity ID and a fresh hidden capture token to the terminal extras method.
- Direct BRP payloads remain in `result: Option<Value>` while the response adds resolved entity/name metadata and retains the original path parameter for message rendering.

**What deviated from the plan:**
- The concrete handler overrides `ToolFn::call` through `call_with_typed_params` so the framework retains a clone of `ScreenshotParams`; the default handler path discards parameters and could not substitute `{path}` in the success message.
- The binary-only MCP crate no longer re-exports `ScreenshotResult` through internal facades after generated routing stopped consuming it; only the new concrete marker needs the planned facade re-exports.

**Surprises:**
- The hidden token boundary needed explicit wire/schema coverage in addition to UUID generation tests: `capture_id` is private to the extras request and never appears in public MCP parameters.
- Concrete full and direct-ID README examples made the three supported selector forms unambiguous alongside the one-call `NatesList` example.

**Implications for remaining phases:**
- Runtime fixtures and integration cases must keep using the single `brp_extras_screenshot` tool and assert resolved entity/name metadata without observing or supplying the hidden capture token.
- Phase 7 must treat a returned success as terminal publication and inspect the returned path directly; no filesystem polling is needed or permitted.
- The no-extras application can still use MCP-local name discovery, while screenshot calls against it must surface the underlying BRP method-not-found error.

### Phase 5 Review

- Replaced the contradictory shared-reference assumption with ordered 2D/UI and 3D camera epochs, each using its own full-target reference before the dedicated both-active ambiguity case.
- Moved new screenshot fixture logic into `test-app/examples/extras_plugin/screenshot_fixtures.rs` so the existing large example root remains setup wiring.
- Pinned runner labels to `extras_app` and `no_extras_app`, and preserved port 25000 as the no-extras standalone fallback.
- Made the PNG assertion helper standard-library-only, added explicit path-absence coverage, and required exact authorized command forms.
- Required Phase 7 to assert raw terminal result placement, resolved metadata, exact no-extras error details, and exact prohibited BRP method names without exposing the internal token.
- Made write failure deterministic through an existing-directory destination and preserved the current FPS diagnostics coverage while removing polling.
- No user decisions or phase-order changes remain.

### Phase 6 — Runtime fixtures and PNG assertion tooling  · status: todo

#### Work Order

**Goal:** Deterministic Bevy fixtures and an authorized read-only PNG helper make exact full/entity capture verification executable.

**Spec:**

Add stable named fixtures: fixed-size UI; rotated/clipped UI; a distinct partially initialized UI entity; transformed 2D AABB with `Camera2d`; transformed 3D AABB with `Camera3d`; a wholly unsupported entity; duplicate/unique/unnamed names including `NatesList`; explicit nonzero viewport; hidden entities; disjoint render layers; and distinctive interior/edge colors. Let real Bevy UI layout and propagation produce complete computed node, target-camera, and render-target state before capture; preserve the test app's default UI features rather than fabricating the complete runtime fixture. Keep `test-app/examples/extras_plugin.rs` as setup/registration wiring and put new screenshot fixture state and systems in `test-app/examples/extras_plugin/screenshot_fixtures.rs`.

Render exact-coordinate cases to one retained fixed-size image/manual target and retain its asset handle plus every backing GPU/manual-view resource for the entire runtime test. Give the 2D/UI and 3D cameras stable unique names and drive deterministic camera-state epochs by mutating `Camera::is_active`: first capture a 2D/UI full-target reference and all 2D/UI crops while only the 2D/UI camera is active; then deactivate it, activate the 3D camera, capture a separate 3D full-target reference, and capture all 3D crops; activate both only for the dedicated ambiguity case. Each epoch uses a named reference entity whose bounds cover the complete target and the same live-validated camera/target as its comparison entities. Compare each smaller crop only with the reference PNG from its own unchanged camera epoch. Full scope intentionally captures only the primary window, so retain a separate primary-window full-capture smoke fixture.

Update `test-app/examples/no_extras_plugin.rs` to parse runner-provided `BRP_EXTRAS_PORT`, falling back to the existing port 25000 when the variable is absent so standalone and shutdown-test behavior remain compatible. Change the `extras_capture` integration-test config to an `apps` array with dynamically isolated runner-managed labels named exactly `extras_app` and `no_extras_app`. The no-extras app enables standard BRP and reflected `Name`, but never `bevy_brp_extras`.

Add a read-only Python-standard-library helper for the emitted non-interlaced 8-bit RGB/RGBA PNGs. It must parse PNG chunks, decompress and unfilter rows, report dimensions, detect uniform images, check fixture-marker pixels, compare every entity-crop pixel with the corresponding rectangle from the correct camera-epoch reference PNG, and provide an explicit pre-call path-absence assertion mode. Add focused helper tests for RGB and RGBA input plus valid, malformed, incomplete, uniform, marker-mismatch, crop-mismatch, and expected-present/expected-absent paths. Authorize only the helper's exact `python3 .claude/scripts/integration_tests/extras_assert_png.py ...` command forms in `integration-tester.md`; it must not call BRP. Assert returned rectangles against known fixed-target coordinates. Keep same-target concurrency, deadlines, disconnects, completion/timeout contention, and late-worker suppression in deterministic Phase 1/2 nextest/App tests.

**Files:**
- `test-app/examples/extras_plugin.rs` — register the screenshot fixture module and keep existing example setup.
- `test-app/examples/extras_plugin/screenshot_fixtures.rs` — screenshot target resources, camera epochs, stable named entities, and marker geometry.
- `test-app/examples/no_extras_plugin.rs` — runner-provided isolated port and standard-BRP-only named fixtures.
- `.claude/scripts/integration_tests/extras_assert_png.py` — standard-library PNG decoding, absence/dimensions/uniformity/marker/exact-crop modes.
- `.claude/scripts/integration_tests/test_extras_assert_png.py` — RGB/RGBA, path-state, malformed, incomplete, uniform, marker-mismatch, and crop-mismatch tests.
- `.claude/agents/integration-tester.md` — authorize generic find and exact PNG-helper command forms while retaining screenshot access.
- `.claude/config/integration_tests.json` — configure runner-managed `extras_app` and `no_extras_app` labels on isolated ports.

**Constraints from prior phases:** Phases 1–3 provide terminal full/entity capture through the existing method and ownership-based capture modules. Phase 4 provides MCP-local standard-BRP name discovery. Phase 5 keeps the single `brp_extras_screenshot` identity with optional `entity`, exact `name`, `camera`, and `padding`; it resolves exact names locally, sends only the canonical entity ID plus an internally generated UUID to extras, retains `ScreenshotParams` for response message rendering, and returns raw terminal fields plus resolved entity/name metadata. Fixtures and helpers never supply or observe the hidden UUID. The helper is read-only, never calls BRP, and consumes only paths returned after terminal completion. Image-target identity uses one full-target reference entity per unchanged camera epoch, never an unavailable arbitrary-target full screenshot.

**Acceptance gate:** Fixture setup is deterministic at fixed target size; 2D/UI and 3D comparisons use separate ordered references from unchanged camera epochs; complete UI state comes from real layout/propagation; partial UI and wholly unsupported fixtures remain distinct; expected rectangles/colors are documented; the root example remains setup wiring; and the no-extras port fallback still works. Standard-library helper tests prove RGB/RGBA dimensions, uniform detection, marker checks, exact crop equality, explicit path-absence checks, malformed/incomplete PNG failure, and nonzero exit on mismatch. Integration-tester authorization permits only exact helper command forms and required MCP tools. Workspace check and relevant nextest/helper tests pass.

### Phase 7 — MCP runtime integration execution  · status: todo

#### Work Order

**Goal:** Real MCP integration proves terminal full/UI/2D/3D screenshots, one-call named scope, camera behavior, RGB crop identity, and extras-independent discovery.

**Spec:**

Extend `.claude/integration_tests/extras_capture.md` for primary-window full terminal capture; one-call `name: "NatesList"`; direct ID; UI/2D/3D; generic non-exact discovery then ID; duplicate ambiguity; unsupported/uninitialized entity; explicit/ambiguous camera; zero/default/padded bounds; offset viewport; clipping; deterministic write failure; and the existing FPS diagnostics assertions. Use unique destinations, remove old files, and use the Phase 6 helper's absence mode before each call. A successful call needs no later polling: validate terminal fields under the preserved raw `result` object, resolved `entity`/`name` under MCP metadata, and the returned rectangle, then run the helper for dimensions, nonuniformity, marker pixels, and exact crop equality against the reference from the same camera epoch. Named capture must return both the canonical ID and exact name; direct-ID capture returns the canonical ID without synthesizing a name. Neither test supplies nor expects the internal `capture_id`. UI success must assert `bounds_kind: "ui"`, the computed camera ID, and the final clipped rectangle so the test proves the UI resolver path. Use the existing `<cwd>/mcp` directory itself as the destination for the write-failure case so publication fails deterministically without permission assumptions. Remove every polling instruction while retaining diagnostics coverage.

Use the runner-prelaunched `extras_app` and `no_extras_app` applications and their label-specific isolated ports. Resolve the stable 2D/UI and 3D camera names, mutate `Camera::is_active`, and execute the 2D/UI reference-and-crop epoch followed by the 3D reference-and-crop epoch; activate both cameras only for the ambiguity case. Call `world_find_entities_by_name` against the no-extras port and verify it succeeds. The MCP registry still contains `brp_extras_screenshot`, so invoking it against this app must return JSON-RPC code `-32601` with method `brp_extras/screenshot`, not an unavailable-MCP-tool error.

Retire/repurpose the polling helper. BRP `rpc.discover` introspection must prove `brp_extras/screenshot` remains and the exact prohibited method names `brp_extras/screenshot_entity` and `brp_extras/find_entities_by_name` are absent. Phase 5's MCP registry test owns proof that no screenshot-entity MCP tool was added. Keep existing runner registrations unless fixture invocation requires change. Install the MCP binary, restart the MCP client session, use isolated ports, and remove artifacts.

**Files:**
- `.claude/integration_tests/extras_capture.md` — runtime cases and assertions.
- `.claude/integration_tests/introspection.md` — consolidated BRP discovery assertions.
- `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` — retire/repurpose obsolete polling.
- `.claude/config/integration_tests.json` — verify registrations; edit only if necessary.

**Constraints from prior phases:** Phase 6 provides deterministic `extras_app`/`no_extras_app` fixtures, stable named camera controls, separate 2D/UI and 3D reference epochs on one retained target, and the standard-library PNG helper with path-absence mode. Phases 1–3 provide the terminal full/entity extras method; Phase 4 provides extras-independent name discovery; Phase 5 provides the single MCP screenshot tool, exact-name composition, resolved entity/name metadata, and an internal-only capture UUID. Phase 1 already covers same-target concurrency, frame-stamped delivery tombstones, one-frame publication confirmation, generation-wide deadlines, destination sharing/conflicts, real watcher disconnects, stale-worker ownership, and exact Windows replacement selection; keep those as deterministic App/nextest tests rather than duplicating them in MCP integration. Runtime cases never poll screenshot paths or supply the hidden token.

**Acceptance gate:** Run the full `clippy` skill, workspace/default/no-default/WASM checks, `cargo nextest run --workspace`, and `/test extras_capture,introspection` after MCP reinstall/restart. Tests prove full and `NatesList` captures, direct ID, metadata/raw-result placement, separate 2D/UI and 3D exact crop epochs, UI metadata, terminal and deterministic publication failures, camera ambiguity, zero padding, FPS diagnostics, generic discovery without extras, exact `-32601` BRP method-not-found details for the screenshot tool against the no-extras app, exact absence of prohibited BRP method names, Phase 5's MCP registry assertion, authorization, and cleanup.
