# Terminal Screenshot Capture

## What it is

`brp_extras/screenshot` is a terminal BRP watching method that publishes a complete RGB PNG before
returning. The existing `brp_extras_screenshot` MCP tool exposes full primary-window capture,
active-camera viewport capture, and entity crops selected by canonical entity ID or unique exact Bevy
`Name`. Capture and file work remain asynchronous inside Bevy, while the agent makes one MCP call and
receives either final publication metadata or a bounded error; no second screenshot method, MCP tool,
or filesystem polling contract exists.

## How it works

The native extras handler is:

```rust
fn handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>>
```

`BrpExtrasPlugin` registers it as `RemoteMethodSystemId::Watching`.
`extras/src/screenshot/request.rs` decodes the extras wire request with unknown-field rejection into:

```rust
enum ScreenshotScope {
    Full {
        camera: Option<Entity>,
    },
    Entity {
        entity: Entity,
        camera: Option<Entity>,
        padding: u32,
    },
}

struct ScreenshotRequest {
    path: PathBuf,
    scope: ScreenshotScope,
}
```

The request forms are:

| Fields | Capture |
| --- | --- |
| `path` | Full primary window |
| `path`, `camera` | Selected camera's physical viewport |
| `path`, `entity` | Entity crop using its computed UI camera, or the only eligible active camera for AABB bounds |
| `path`, `camera`, `entity` | Entity crop using the selected camera |

`padding` defaults to zero physical pixels and is valid only for entity capture. Relative paths are
joined to the application working directory and lexically normalized.

### MCP composition

`mcp/src/brp_tools/tools/brp_extras_screenshot.rs` exposes:

```rust
pub struct ScreenshotParams {
    pub entity: Option<u64>,
    pub name: Option<String>,
    pub camera: Option<u64>,
    pub padding: Option<u32>,
    pub path: String,
    pub port: Port,
}
```

`entity` and `name` are mutually exclusive. Direct-ID, camera-only, and full-window requests issue
one terminal `brp_extras/screenshot` BRP call. A name-selected request first calls standard
`world.query` through `world_find_entities_by_name`, requires one case-sensitive exact match, then
sends only the canonical entity ID to extras. It therefore performs one discovery BRP call followed
by one screenshot BRP call, while remaining a single MCP tool invocation.

`mcp/src/brp_tools/tools/world_find_entities_by_name.rs` is independent of `bevy_brp_extras`. It
queries the reflected `bevy_ecs::name::Name` component, filters locally using exact, prefix, suffix,
or contains matching, and sorts results by canonical entity ID. Matching is case-sensitive and `*`
is literal text.

### Capture lifecycle

`extras/src/screenshot/capture/pending_screenshot_capture.rs` owns one global active slot in
`PendingScreenshotCapture`. The first watching invocation:

1. Validates the request, target, camera, and optional bounds.
2. Spawns one Bevy `Screenshot` entity with an observer.
3. Stores the normalized request, crop, response metadata, screenshot entity, and a 25-second
   deadline.
4. Returns `Ok(None)` so Bevy Remote keeps the request open.

When Bevy emits `ScreenshotCaptured`, the observer transfers the job to the worker pipeline in
`screenshot_job.rs`:

1. `AsyncComputeTaskPool` converts the captured Bevy `Image` to `TargetRgbImage`, applies the
   optional crop, and encodes PNG bytes.
2. `IoTaskPool` writes those bytes to a `NamedTempFile` in the destination directory.
3. A lazily created completion channel returns an owned `TempPath` and snapshotted response metadata
   to the main world.
4. The main-world lifecycle system validates the completion and persists the temporary file to the
   requested destination.
5. The watching handler returns `Some(response)` only after publication, or returns a terminal
   `BrpError`.

The worker never publishes the destination itself. Dropping an unpublished `TempPath` removes the
temporary file. The completion-ingest and lifecycle systems use `run_if(screenshot_capture_active)`,
so their frame counter and completion channel stay dormant when no capture exists.

### Camera and bounds resolution

A camera is eligible only when it has initialized `Camera` and `RenderTarget` state, is active, has a
nonempty physical target and viewport, and points to a live window, render-world image, or manual
texture view. `RenderTarget::None` and stale targets are rejected.

A camera-only request captures the final target and crops it to `Camera::physical_viewport_rect()`.

For entity capture, UI bounds take precedence when the default `ui` feature is enabled:

- `extras/src/screenshot/ui.rs` requires the complete computed UI family: `ComputedNode`,
  `UiGlobalTransform`, `ComputedUiTargetCamera`, and `ComputedUiRenderTargetInfo`.
- It validates inherited visibility and the computed target camera.
- It transforms node corners into physical target coordinates and intersects the result with
  `CalculatedClip`, camera viewport, and live target extent.
- Partial UI state is an error and does not fall back to AABB.
- UI resolution ignores incidental `Aabb` and `RenderLayers`.

Otherwise, `extras/src/screenshot/aabb.rs` requires `Aabb` and `GlobalTransform`. It validates
visibility, compatible render layers, selected-view membership when available, and camera-frustum
intersection. It projects all eight transformed AABB corners into physical target coordinates,
applies padding, and clips to the camera viewport and target. Near- or far-plane crossings
conservatively use the complete viewport intersection because a reliable finite projected rectangle
is unavailable.

With no explicit camera, a UI entity uses its computed UI target camera. An AABB entity requires
exactly one eligible active camera; multiple candidates return deterministic ascending camera IDs.

### Responses and implementation map

Every successful extras response includes:

- `success: true`
- `status: "completed"`
- normalized absolute `path`
- `working_directory`
- the completion `note`

Entity responses additionally include `capture_kind: "entity"`, canonical `entity`, snapshotted
optional Bevy `name`, selected `camera`, `bounds_kind` (`"aabb"` or `"ui"`), and physical `rect`.

The MCP preserves the raw extras response under `result`. It adds top-level entity metadata for
direct-ID and name-selected requests, but adds top-level name metadata only when the MCP resolved a
`name`. A direct-ID result may still contain the entity's Bevy name inside the raw extras result.

Key files and roles:

- `extras/src/screenshot/mod.rs` — watching handler, target and camera validation, bounds routing,
  and response construction.
- `extras/src/screenshot/request.rs` — strict extras request decoding and normalized paths.
- `extras/src/screenshot/capture/pending_screenshot_capture.rs` — single active slot, deadline,
  terminal delivery, cleanup, and main-world publication.
- `extras/src/screenshot/capture/screenshot_job.rs` — compute/I/O worker split and temporary-file
  ownership.
- `extras/src/screenshot/capture/target_rgb_image.rs` — RGB conversion, crop validation, and PNG
  encoding.
- `extras/src/screenshot/aabb.rs` — 3D/2D AABB visibility and physical projection.
- `extras/src/screenshot/ui.rs` — feature-gated computed UI bounds.
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — MCP selectors, exact-name composition, extras
  payload, and result/error preservation.
- `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — extras-independent name discovery.
- `test-app/examples/extras_plugin/screenshot_fixtures.rs` — deterministic window, viewport, UI, 2D,
  3D, visibility, layer, and duplicate-name reference scene.
- `.claude/integration_tests/extras_capture.md` — pinned runtime behavior, crop rectangles, pixel
  identities, publication failures, and no-extras boundary.

## Invariants

- The screenshot BRP method remains `brp_extras/screenshot`; do not add separate entity or name
  screenshot methods.
- Extras accepts canonical entity IDs only. Name matching and ambiguity handling remain MCP-local and
  use standard BRP.
- A successful response means the complete PNG has already been published at the reported
  destination.
- Publication occurs only on the main world from an owned same-directory temporary file.
- Captured images are converted to RGB before cropping and encoding.
- Only one capture may be active. A different request while occupied returns
  `A screenshot capture is already in progress`.
- Repeated watching invocations for the same normalized request observe the existing lifecycle
  rather than spawning another screenshot.
- The active request has a 25-second server deadline, shorter than the MCP transport's 30-second
  timeout.
- Terminal BRP codes and optional data are preserved by the MCP wrapper.
- Entity and name selectors remain mutually exclusive; padding requires either selector.
- UI capture uses the entity's computed UI target camera. AABB camera inference succeeds only with
  exactly one eligible active camera.
- UI bounds take precedence over AABB when the complete UI family exists. Partial UI initialization
  is an error.
- Crop rectangles are physical target-space coordinates and remain clipped to all applicable hard
  bounds.
- Metadata describing the entity, name, camera, bounds kind, and rectangle is snapshotted when
  capture begins.
- Idle capture systems remain guarded and the completion channel remains absent until encoding
  begins.

## Calibration and gotchas

- The single active slot has no request ID. Request equality is the complete normalized
  `ScreenshotRequest`; concurrent identical external requests cannot be distinguished from Bevy
  Remote reinvoking the same watching request and should not be issued.
- The 25-second deadline is checked while capture is pending and when worker completion is ingested.
  Late worker completions are dropped and their temporary files are cleaned up. Publication is a
  synchronous main-world persist after that completion check, so a persist that starts before the
  deadline is not timed again while it runs.
- Full primary-window capture requires a live primary window. Explicit camera capture may target a
  window, render-world image, or manual texture.
- A render-target image present only in the main world is not eligible; it must include
  `RenderAssetUsages::RENDER_WORLD`.
- Entity crops isolate a rectangle, not rendered ownership. They may contain background,
  descendants, siblings, occluders, post-processing, or overlapping UI, and descendants are not
  gathered automatically.
- AABB visibility uses `Visibility`, `InheritedVisibility`, `ViewVisibility`, render layers, frustum
  state, and available selected-view membership. `NoCpuCulling` intentionally bypasses selected-view
  membership.
- UI padding cannot expand beyond inherited clipping, the camera viewport, or the live target.
- If the `ui` feature is disabled, UI resolution is absent and entities need AABB bounds. A
  non-AABB entity reports that UI support is disabled.
- PNG support must be enabled in Bevy. The bytes are PNG regardless of the destination filename's
  extension.
- WASM returns an immediate actionable error because filesystem PNG publication is unsupported.
- Path normalization is lexical; it does not canonicalize symlinks.
- Name resolution sees reflected `Name` components only. Missing names fail locally; duplicate exact
  names return sorted IDs and require the caller to choose an entity.
- Camera-only and full-window responses do not add entity-only metadata.
- Near- or far-plane AABB crossings intentionally expand to the usable viewport rather than risk
  clipping visible geometry.
- Publication can replace an existing file atomically, but a destination that is an existing
  directory or otherwise cannot be persisted produces a terminal internal error and leaves the
  destination intact.

## Why it is this way

A Bevy Remote watching method is the boundary that makes capture synchronous from the agent's
perspective without blocking Bevy's frame loop. Returning only after main-world publication removes
the former initiated-file ambiguity and prevents agents from observing incomplete output.

The single active slot matches the actual rendering resource: screenshot capture is an infrequent
diagnostic operation with one terminal result, not a throughput pipeline. Keeping the state model to
capturing, encoding, completed, or failed makes ownership, timeout, and cleanup behavior explicit.

Converting to RGB before encoding matches Bevy screenshot semantics and discards HDR brightness data
carried in alpha, avoiding misleading black, white, or translucent PNGs. Writing beside the
destination and transferring an owned temporary path back to the main world provides atomic
publication without allowing detached workers to claim success.

Name lookup remains in the MCP because standard `world.query` already exposes reflected names.
Extras therefore stays focused on the rendering primitive and remains usable by callers that know
canonical IDs, while the MCP provides the one-call natural-language workflow.

Entity capture crops the final composited target because Bevy's screenshot primitive captures render
targets, not isolated entities. UI-first and AABB-fallback resolution use the bounds Bevy actually
maintains for those rendering paths, while conservative clipping and camera ambiguity errors avoid
silently returning misleading regions.
