# Entity screenshots

> **Status: IMPLEMENTATION PLAN — phased, delegate-ready.** Adds terminal, composited entity screenshots for UI and AABB-backed 2D/3D entities plus deterministic entity-name discovery.

## Delegation Context

- **Project:** `bevy_brp` workspace — `bevy_brp_extras` supplies extra Bevy Remote Protocol methods, `bevy_brp_mcp` exposes them as MCP tools, and `bevy_brp_test_apps` supplies runtime fixtures.
- **Stack:** Rust 2024 edition (`rustc 1.97.0`; formatting toolchain `rustc 1.98.0-nightly`), Bevy `0.19.0`, `bevy_remote 0.19.0`, workspace crates `0.21.0-dev`, `rmcp 1.7.0`, Serde/JSON, Schemars, and Bevy render/UI/camera APIs.
- **Layout:** `extras/` — BRP implementation, features, docs, and unit tests; `mcp/` — typed MCP facade, tool registry/help, docs, and changelog; `test-app/` — deterministic Bevy fixtures; `.claude/integration_tests/`, `.claude/scripts/integration_tests/`, `.claude/agents/` — runtime specifications, screenshot helper, and tester authorization.
- **Key files:**
  - `Cargo.toml` — workspace dependency versions and lint policy.
  - `extras/Cargo.toml` — add the default-enabled `ui = ["bevy/bevy_ui"]` feature while retaining no-default-feature AABB support.
  - `extras/src/constants.rs` — BRP method names, request/response fields, and default padding.
  - `extras/src/plugin.rs` — register the screenshot-entity watching method, name-discovery method, and capture resource/systems.
  - `extras/src/screenshot.rs` — existing screenshot handler; shared target coordination, jobs, crop/save pipeline, entity selector, camera/bounds resolution, completion state, and unit tests; split only into `extras/src/screenshot/` anchor-type modules if required by repository size rules.
  - `extras/src/lib.rs` — crate module wiring and public BRP-method documentation.
  - `extras/README.md` — extras API, feature, and behavior documentation.
  - `extras/CHANGELOG.md` — extras release entry.
  - `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — existing full-screenshot MCP types and corrected asynchronous success wording.
  - `mcp/src/brp_tools/tools/brp_extras_screenshot_entity.rs` — new screenshot-entity parameter/result types.
  - `mcp/src/brp_tools/tools/brp_extras_find_entities_by_name.rs` — new name-discovery parameter/result types.
  - `mcp/src/brp_tools/tools/mod.rs` — declare and re-export both new tool modules.
  - `mcp/src/brp_tools/mod.rs` — facade re-exports for both new tools.
  - `mcp/src/tool/name.rs` — static `ToolName` mappings, annotations, schemas, and handlers.
  - `mcp/help_text/brp_extras_screenshot.txt` — existing asynchronous full-screenshot semantics.
  - `mcp/help_text/brp_extras_screenshot_entity.txt` — selector, camera, terminal publication, feature, and composited-crop help.
  - `mcp/help_text/brp_extras_find_entities_by_name.txt` — exact/prefix/suffix/contains discovery help.
  - `mcp/README.md` — MCP tool documentation.
  - `mcp/CHANGELOG.md` — MCP release entry.
  - `test-app/examples/extras_plugin.rs` — deterministic UI, 2D, 3D, name, unsupported-bounds, viewport, and camera-ambiguity fixtures.
  - `.claude/integration_tests/extras_capture.md` — full-screenshot regression plus terminal entity-capture dimensions, pixels, errors, and cleanup.
  - `.claude/integration_tests/introspection.md` — discovery assertions for both new BRP methods.
  - `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` — existing full-screenshot-only polling timeout and stale-file behavior.
  - `.claude/agents/integration-tester.md` — authorize `mcp__brp__brp_extras_screenshot_entity` and `mcp__brp__brp_extras_find_entities_by_name`.
  - `.claude/config/integration_tests.json` — existing `extras_capture` and `introspection` runner registration.
- **Build:** `cargo check --workspace` and `cargo check -p bevy_brp_extras --no-default-features`.
- **Test:** `cargo nextest run --workspace`, then run `/test extras_capture,introspection` with isolated ports after installing the updated MCP binary and restarting the MCP client session.
- **Lint:** Full `clippy` skill.
- **Style:** `zsh ~/.claude/scripts/rust_style/load-rust-style.sh --project-root /Users/natemccoy/rust/bevy_brp`
- **Invariants:** Use `cargo +nightly fmt --all -- --check`, never plain `cargo fmt`; preserve existing `brp_extras/screenshot` wire behavior and keep polling exclusive to that asynchronous method; `screenshot_entity` accepts exactly one ID or unique exact case-sensitive name and completes only after atomic PNG publication or terminal error; entity ID remains canonical for unnamed/duplicate entities; UI support is default-enabled but optional, and `--no-default-features` retains the generic AABB route without sprite/text/PBR dependencies; resolve only the supplied entity, with complete UI components taking precedence over AABB and partial UI returning an initialization error; crops use the selected camera’s screenshot-capable target and physical target coordinates, honor viewport/clip/target hard bounds, contain partially covered pixels, and use conservative full-viewport fallback for depth-plane-crossing AABBs; reject hidden entities, disjoint render layers, unsupported/uninitialized bounds, ambiguous or invalid cameras, empty crops, target/extent changes, and reserved-path collisions; output is a crop of the final composited target, not isolated rendering; coordinate one Bevy screenshot entity per normalized target, fan out same-target jobs, convert the full image once, reserve paths through observed I/O completion, and publish through a same-directory temporary file plus atomic rename; matching results are deterministically ordered by entity ID and never overload `*`; keep MCP result payloads in `result: Option<Value>` for `ResultStruct`; do not add lint suppressions, generic helper modules, speculative caches, benchmark infrastructure, or single-implementation traits.

## Phases

### Phase 1 — Shared screenshot capture pipeline  · status: todo

#### Work Order

**Goal:** The existing full screenshot method uses a reliable shared capture/save pipeline that safely serves multiple same-target jobs.

**Spec:**

Refactor `extras/src/screenshot.rs` around these concrete internal types:

```rust
struct ScreenshotJob {
    path: PathBuf,
    crop: Option<URect>,
}

#[derive(Resource)]
struct PendingScreenshotCaptures {
    // One active Bevy Screenshot entity and one or more jobs per normalized target.
    // Destination paths stay reserved until their I/O completion is observed.
    // Concrete fields are chosen with the implementation's ownership flow.
}
```

Key the resource by `NormalizedRenderTarget`. A request normalizes its target, reserves its destination path, and joins the existing target batch or spawns the target's sole Bevy `Screenshot` entity. Bevy despawns duplicate same-target screenshot entities in one frame without emitting `ScreenshotCaptured`, so never spawn more than one active screenshot entity per normalized target. Reject a destination path already reserved by an in-flight job.

The `ScreenshotCaptured` observer removes the target batch, clones the captured `Image` once, and moves all jobs to `IoTaskPool`. Convert the complete image once, apply each optional crop, create destination directories, encode PNGs, and publish each file with a same-directory temporary file plus atomic rename. Send per-job completion back to the main world so reserved paths are released. Before cropping, intersect the requested crop with the actual image extent; a changed extent that makes the promised crop empty or smaller is an error, not a silently changed success.

Route the existing `brp_extras/screenshot` through this pipeline with the primary-window target and `crop: None`. Preserve its wire behavior. Correct only its MCP success text so it says asynchronous capture was initiated; this existing full screenshot remains the only method whose integration test polls for file creation.

If `screenshot.rs` exceeds repository module-size criteria, replace it with a `screenshot/` module whose children are named after anchor types. Do not introduce a generic helper module or a trait with one implementation.

**Files:**
- `extras/src/screenshot.rs` — shared jobs, normalized-target coordination, observer, image conversion, crop/save tasks, completion, and focused tests.
- `extras/src/plugin.rs` — initialize the capture resource and any completion system required by the shared pipeline.
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — make existing full-screenshot success wording accurately say capture was initiated.
- `mcp/help_text/brp_extras_screenshot.txt` — retain asynchronous semantics and polling guidance.

**Constraints from prior phases:** None.

**Acceptance gate:** Existing full screenshot behavior remains compatible; focused tests prove uncropped output, a known in-memory crop's dimensions and pixels, same-target fan-out, reserved-path collision rejection, changed captured extent failure, atomic publication, and cleanup. `cargo check --workspace` and the relevant `cargo nextest run` tests pass.

### Phase 2 — Entity selection and camera resolution  · status: todo

#### Work Order

**Goal:** Entity requests become typed, unambiguous selections with deterministic screenshot-capable camera resolution.

**Spec:**

Parse the flat wire request through a file-private raw request and immediately convert its two optional selector fields into:

```rust
enum EntitySelector {
    Id(Entity),
    Name(String),
}

enum BoundsKind {
    Ui,
    Aabb,
}

struct EntityCapture {
    target: RenderTarget,
    camera: Entity,
    bounds_kind: BoundsKind,
    rect: URect,
}
```

The raw request has optional `entity`, optional `name`, required `path`, optional `camera`, and optional `padding`; padding defaults through a named constant in `extras/src/constants.rs`. Require exactly one selector. `entity` uses the same `u64` representation as existing MCP tools. `name` performs unique exact case-sensitive lookup. Resolve to the entity and its optional owned `Name` before camera or bounds work. No match and duplicate matches return structured errors; ambiguity data includes every matching entity ID so the caller can retry with the canonical ID.

Camera selection is deterministic:

1. A later UI resolver uses its `ComputedUiTargetCamera`; an explicitly supplied camera must match.
2. An AABB request uses the supplied active camera when present.
3. Without a supplied camera, use the only screenshot-capable active camera. Zero or multiple candidates are errors listing stable candidate entity IDs and instructing the caller to supply `camera`.

Use one predicate for inferred and explicit cameras: it is active; `Camera::physical_target_size()` and `Camera::physical_viewport_rect()` are nonempty; its target normalizes successfully; and the normalized target is a window, image, or manual texture view. `RenderTarget::None` is never screenshot-capable. Ambiguity is `INVALID_PARAMS` with `BrpError::data` shaped as:

```json
{
  "reason": "ambiguous_camera",
  "camera_candidates": [4294967302, 4294967310]
}
```

The initial operation resolves only the supplied entity, never descendants. Reject nonexistent entities, neither/both selectors, missing or ambiguous names, hidden entities, missing/inactive/uninitialized cameras, and camera mismatch with actionable structured errors.

**Files:**
- `extras/src/constants.rs` — method/field constants and default physical-pixel padding.
- `extras/src/screenshot.rs` — raw request, typed selector, shared capture result types, exact-name resolution, camera predicate/selection, error data, and tests.

**Constraints from prior phases:** Phase 1 provides the normalized-target capture queue and reserves output paths until observed I/O completion; entity requests will enqueue into that pipeline rather than spawn Bevy screenshots directly.

**Acceptance gate:** Unit tests cover ID selection, unique exact name, no match, duplicate names with stable IDs, invalid dual/empty selectors, missing/ambiguous/explicit cameras, screenshot-capable target validation, and hidden entities. `cargo check --workspace` and the relevant `cargo nextest run` tests pass.

### Phase 3 — AABB bounds resolution  · status: todo

#### Work Order

**Goal:** AABB-backed 2D and 3D entities resolve conservative, bounded physical-pixel crop rectangles for a selected camera.

**Spec:**

Read `Aabb` and `GlobalTransform`, plus the selected `Camera` and camera `GlobalTransform`. Use Bevy-generated `Aabb` components as-is; do not synchronously recompute bounds from mesh, sprite, text, or asset data. Procedural geometry must maintain its `Aabb`; skinned meshes needing current bounds use Bevy's dynamic bounds support. The generic route must not require sprite, text, or PBR features.

Resolution algorithm:

1. Generate all eight local corners from `Aabb::center` and `Aabb::half_extents`.
2. Transform every corner through the entity's `GlobalTransform`.
3. Reject an oriented box that does not intersect the selected camera's `Frustum` by calling `Frustum::intersects_obb` with the local AABB and entity affine transform.
4. Project each world corner with `Camera::world_to_viewport`.
5. Convert logical target coordinates to physical pixels with `Camera::target_scaling_factor()`.
6. Reject non-finite coordinates. Floor the minimum, ceil the maximum, and treat the integer maximum as exclusive.
7. Apply padding with saturating integer arithmetic.
8. Intersect as final hard constraints with `Camera::physical_viewport_rect()` and the physical target bounds.
9. Reject an empty result.

After the frustum guard, `PastNearPlane` or `PastFarPlane` for any corner uses the complete camera viewport as a conservative first-release crop. This avoids cutting visible geometry when the box crosses a depth plane or contains the camera. `NoViewportSize` and `InvalidData` remain errors.

Reject an AABB entity and selected camera with disjoint `RenderLayers`, applying Bevy's default-layer semantics when either component is absent. `Aabb` proves bounds exist, not that a custom renderer contributed pixels; retain that documented limitation.

**Files:**
- `extras/src/screenshot.rs` — AABB resolver, projection/rounding/clamping helpers owned by the screenshot domain, visibility/layer validation, and tests.
- `extras/Cargo.toml` — confirm the no-default-feature route has only the Bevy capabilities needed for generic AABB resolution.

**Constraints from prior phases:** Phase 2 supplies `EntityCapture`, typed selectors, validated camera selection, normalized screenshot-capable targets, default padding, and common visibility errors. Return an `EntityCapture` for Phase 1's shared pipeline.

**Acceptance gate:** Unit tests cover translated, rotated, non-uniformly scaled, and reflected AABBs; logical-to-physical scaling; viewport offsets; fractional extrema; padding at all target edges; zero-area/offscreen results; near- and far-plane conservative fallback; boxes wholly outside each depth plane; hidden entities; and disjoint render layers. `cargo check -p bevy_brp_extras --no-default-features` and relevant nextest tests pass.

### Phase 4 — Optional UI bounds resolution  · status: todo

#### Work Order

**Goal:** Default builds resolve clipped, transformed Bevy UI nodes while minimal builds retain AABB screenshots without Bevy UI.

**Spec:**

Add `ui = ["bevy/bevy_ui"]` to `bevy_brp_extras` and include it in default features beside `diagnostics`. Gate UI-specific imports, query data, and resolver code. With `ui` disabled, an entity without `Aabb` returns an unsupported-bounds error that names disabled UI support as a possible cause.

The UI resolver reads `ComputedNode`, `UiGlobalTransform`, `ComputedUiTargetCamera`, `ComputedUiRenderTargetInfo`, and optional `CalculatedClip`:

1. Build four local corners from `ComputedNode::size()` around the node origin.
2. Transform every corner through `UiGlobalTransform::affine()`.
3. Take the axis-aligned min/max of transformed physical-pixel points. These points and `CalculatedClip::clip` are camera-viewport-local; do not multiply them by target scaling again.
4. Intersect the local rectangle with `CalculatedClip::clip` when present and with zero through `ComputedUiRenderTargetInfo::physical_size()`.
5. Add `Camera::physical_viewport_rect().min` to both endpoints, translating into target-space physical pixels.
6. Reject non-finite coordinates. Floor minimum, ceil maximum, and treat the integer maximum as exclusive.
7. Apply padding with saturating arithmetic.
8. As final hard constraints, intersect with the translated clip, camera physical viewport, and physical target bounds.
9. Reject an empty result.

Use `ComputedUiTargetCamera`; reject a supplied camera that differs. Select the target from that camera, including non-primary windows and image targets when capturable. A complete UI component family takes precedence over incidental `Aabb`. A partial UI family returns an uninitialized-UI error instead of falling through. Reject hidden UI.

**Files:**
- `extras/Cargo.toml` — default-enabled optional `ui` feature.
- `extras/src/screenshot.rs` — feature-gated UI query/resolver, precedence, unsupported-build diagnostics, and tests.

**Constraints from prior phases:** Phase 2 supplies typed entity/camera selection and `EntityCapture`; Phase 3 establishes shared physical-pixel containment and hard-bound semantics. UI returns the same `EntityCapture` consumed by Phase 1.

**Acceptance gate:** Unit tests cover UI translation, scaling, rotation, partial UI initialization, precedence over incidental AABB, `CalculatedClip`, nonzero viewport origin, target-space translation, fractional extrema, target edges, camera mismatch, hidden UI, and empty/offscreen results. Both `cargo check --workspace` and `cargo check -p bevy_brp_extras --no-default-features` pass with relevant nextest tests.

### Phase 5 — Terminal screenshot-entity BRP method  · status: todo

#### Work Order

**Goal:** `brp_extras/screenshot_entity` accepts an ID or unique exact name and returns only after its cropped PNG is atomically published or a terminal error occurs.

**Spec:**

Register `brp_extras/screenshot_entity` as a watching BRP method. On first execution, validate and resolve the selector, entity family, camera, target, and physical rectangle, then enqueue one `ScreenshotJob { path, crop: Some(rect) }` into Phase 1. Maintain per-request watching state through capture and `IoTaskPool` completion. Return `Some(Ok(...))` only after atomic publication; return `Some(Err(...))` for capture, conversion, PNG encoding, directory creation, or publication failures. A successful response guarantees the returned path is a complete PNG.

The typed successful response contains `status: "completed"`, `path`, resolved `entity`, optional resolved `name`, selected `camera`, `bounds_kind` (`"ui"` or `"aabb"`), and padded/clipped physical-pixel `rect { x, y, width, height }`. The raw request has optional `entity`, optional `name`, required `path`, optional `camera`, and optional physical-pixel `padding`; exactly one selector is required.

The crop is from the final composited render target, not an isolated entity render. It may contain overlapping nodes, meshes, backgrounds, post-processing, and occluders. Shadows, bloom, outlines, particles, and shader displacement outside resolved bounds require padding. Reject unsupported/uninitialized bounds, hidden or layer-incompatible entities, ambiguous selection/camera, invalid target, empty crop, unavailable PNG support, extent changes, and reserved destination collisions rather than writing misleading output.

Register constants and plugin wiring alongside the existing screenshot method. Document the method, optional UI feature, AABB-backed 2D/3D applicability, terminal completion, composited limitation, custom-renderer limitation, and depth-plane conservative fallback in extras crate docs, README, and changelog.

**Files:**
- `extras/src/constants.rs` — screenshot-entity method and response constants.
- `extras/src/plugin.rs` — watching-method registration and required systems/resources.
- `extras/src/screenshot.rs` — request lifecycle, queue integration, terminal response/error construction.
- `extras/src/lib.rs` — public method documentation.
- `extras/README.md` — request/response, supported families, feature, camera, and limitation documentation.
- `extras/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phase 1 owns all capture/save work and per-job completion; Phase 2 resolves selector/camera; Phase 3 resolves AABB crops without UI; Phase 4 adds default-enabled UI precedence and resolution. Do not duplicate those paths in the BRP handler.

**Acceptance gate:** Unit/system tests prove terminal success occurs after the PNG exists, deferred I/O failures return through the same request, output fields reflect actual capture extent, the existing asynchronous full screenshot still works, and same-target full/entity requests both complete. Workspace build and relevant nextest tests pass.

### Phase 6 — Entity-name discovery BRP method  · status: todo

#### Work Order

**Goal:** `brp_extras/find_entities_by_name` deterministically discovers entity IDs using exact or simple substring-position matching.

**Spec:**

Add typed request/response data. The request contains `name` and `match_mode`, where `match_mode` is an enum with `exact`, `prefix`, `suffix`, and `contains`, defaulting to `exact`. Match Bevy `Name` values case-sensitively and return entries shaped as `{ "entity": <u64>, "name": <string> }`, ordered deterministically by entity ID. Do not overload `*` or any wildcard syntax; asterisks in entity names remain literal and searchable.

Register `brp_extras/find_entities_by_name`. This is distinct from screenshot selection: `screenshot_entity` always uses unique exact matching, while callers use discovery for non-exact queries or duplicate names and then call the screenshot method with the canonical entity ID. Document and changelog the method beside screenshot entity.

**Files:**
- `extras/src/constants.rs` — discovery method constants.
- `extras/src/plugin.rs` — method registration.
- `extras/src/screenshot.rs` — name-match enum, deterministic query/response, and tests unless repository cohesion rules justify an anchor-type name module.
- `extras/src/lib.rs` — public method documentation.
- `extras/README.md` — discovery contract and screenshot-selection guidance.
- `extras/CHANGELOG.md` — include discovery in the release entry.

**Constraints from prior phases:** Phase 2 already provides unique exact-name resolution and canonical entity-ID conversion used by screenshot requests. Reuse compatible matching/ordering primitives without coupling discovery to capture state.

**Acceptance gate:** Tests prove exact, prefix, suffix, and contains behavior; default exact mode; case sensitivity; literal `*`; deterministic entity-ID ordering; no matches; and duplicate-name results. Workspace check and relevant nextest tests pass.

### Phase 7 — MCP tool surface  · status: todo

#### Work Order

**Goal:** Both new BRP methods are exposed as statically registered, documented MCP tools with accurate schemas and result semantics.

**Spec:**

Add `ScreenshotEntityParams` with optional `entity`, optional `name`, required `path`, optional `camera`, optional `padding`, and `port`. Help text and runtime validation require exactly one selector. Add `ScreenshotEntityResult` compatible with `ResultStruct`: BRP response data remains only in the recognized `result: Option<Value>` field, not duplicated as typed MCP fields. Register static `ToolName::BrpExtrasScreenshotEntity` mapped to `brp_extras/screenshot_entity`; mark it additive and non-idempotent because it writes a file.

Add `FindEntitiesByNameParams`, `FindEntitiesByNameResult`, and static `ToolName::BrpExtrasFindEntitiesByName` mapped to `brp_extras/find_entities_by_name`. Follow existing macro architecture for parameter schemas, handlers, module declarations, facade re-exports, annotations, and help text.

Screenshot help must say a successful response means capture completed and the PNG is available at `path`; explain exactly-one ID/name selection, unique exact names, discovery for non-exact/ambiguous names, canonical ID retry, camera requirements, physical-pixel padding, UI's default feature, AABB availability without UI, conservative depth-plane crop, and final-composited rather than isolated output. Discovery help describes all four literal matching modes and deterministic IDs. Update MCP README and changelog.

**Files:**
- `mcp/src/brp_tools/tools/brp_extras_screenshot_entity.rs` — screenshot entity params/results and handler-facing types.
- `mcp/src/brp_tools/tools/brp_extras_find_entities_by_name.rs` — discovery params/results.
- `mcp/src/brp_tools/tools/mod.rs` — module declarations/re-exports.
- `mcp/src/brp_tools/mod.rs` — facade re-exports.
- `mcp/src/tool/name.rs` — static tool names, annotations, schemas, and handlers.
- `mcp/help_text/brp_extras_screenshot_entity.txt` — complete screenshot tool guidance.
- `mcp/help_text/brp_extras_find_entities_by_name.txt` — discovery guidance.
- `mcp/README.md` — tool documentation.
- `mcp/CHANGELOG.md` — release entry.

**Constraints from prior phases:** Phase 5 defines terminal screenshot BRP fields and errors; Phase 6 defines discovery fields/modes/order. Preserve those wire contracts exactly, and keep `ResultStruct` BRP data in `result: Option<Value>`.

**Acceptance gate:** MCP schema/registry tests prove both tools are statically discoverable with correct method mappings and annotations; handler tests prove parameters serialize to the established BRP contracts and results preserve nested BRP data. Help and README accurately distinguish terminal entity screenshots from asynchronous full screenshots. Workspace check and relevant nextest tests pass.

### Phase 8 — Runtime fixtures and integration verification  · status: todo

#### Work Order

**Goal:** Deterministic integration coverage proves UI, 2D, and 3D crops, selection/camera errors, complete PNG identity, and tool discovery through the real MCP path.

**Spec:**

Add stable named fixtures to `test-app/examples/extras_plugin.rs`: a fixed-size UI node; rotated/clipped UI; transformed 2D AABB with `Camera2d`; transformed 3D AABB with `Camera3d`; unsupported entity; duplicate, unique, and unnamed entities; explicit nonzero viewport; second active camera; hidden entities; disjoint render layers; and distinctive interior/edge colors. Prefer a fixed-size image render target for exact coordinate assertions and retain one window-target smoke test.

Extend `.claude/integration_tests/extras_capture.md` to invoke both new MCP tools by unique exact name and canonical ID. Cover UI, 2D, 3D, name ambiguity/discovery, unsupported/uninitialized targets, explicit/ambiguous cameras, offset viewport, clipping, same-target concurrency, and write failure. Verify terminal response fields, returned rectangles, decoded PNG dimensions, representative interior/edge pixels, and cleanup. Use unique paths and remove destinations before invocation so stale files cannot satisfy tests. Because `screenshot_entity` is terminal, do not poll for its output.

Keep polling only for the existing asynchronous full-window screenshot regression. Make `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` documented/actual timeout agree, remove stale destination before invocation, and accept only a fully parseable PNG through `IEND`, not merely a nonempty file.

Extend `.claude/integration_tests/introspection.md` to require `brp_extras/screenshot_entity` and `brp_extras/find_entities_by_name`. Add `mcp__brp__brp_extras_screenshot_entity` and `mcp__brp__brp_extras_find_entities_by_name` to `.claude/agents/integration-tester.md` frontmatter while preserving the direct-MCP workflow. The existing `extras_capture` and `introspection` entries in `.claude/config/integration_tests.json` remain the runner registrations; adjust them only if fixture command requirements actually change.

After installing the updated MCP binary, restart the MCP client session before invoking the tools because the current session retains its previously launched MCP subprocess. Run integration specifications on isolated ports and clean every generated artifact.

**Files:**
- `test-app/examples/extras_plugin.rs` — deterministic capture/name/camera fixtures.
- `.claude/integration_tests/extras_capture.md` — runtime MCP cases and PNG assertions.
- `.claude/integration_tests/introspection.md` — new method discovery assertions.
- `.claude/scripts/integration_tests/extras_test_poll_screenshot.sh` — existing full-screenshot stale-file, complete-PNG, and timeout correction.
- `.claude/agents/integration-tester.md` — authorize both new MCP tool names.
- `.claude/config/integration_tests.json` — verify existing test registrations; edit only if necessary for fixture invocation.

**Constraints from prior phases:** Phase 5's entity screenshot is terminal and atomically publishes complete PNGs; Phase 6 provides discovery and canonical IDs; Phase 7 supplies exact MCP tool names and schemas. Polling remains exclusive to the old asynchronous full screenshot.

**Acceptance gate:** Run the full `clippy` skill, `cargo check --workspace`, `cargo check -p bevy_brp_extras --no-default-features`, `cargo nextest run --workspace`, and `/test extras_capture,introspection` on isolated ports after MCP reinstall/restart. Tests prove returned rectangles, PNG dimensions and representative pixels, terminal I/O errors, ambiguity/unsupported/camera behavior, same-target completion, old screenshot regression, discovery output, integration-tester authorization, artifact cleanup, and introspection of both methods.
