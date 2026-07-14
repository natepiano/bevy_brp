# Terminal screenshot capture

> **Status: AS BUILT.** The existing screenshot operation can capture the primary window, an active camera viewport, or an entity crop. One MCP call remains open until the PNG is published or the request fails.

## Executive summary

This work extends the existing `brp_extras/screenshot` BRP method and `brp_extras_screenshot` MCP tool. It does not add a second screenshot method or tool.

The caller supplies a destination and any applicable selectors:

| Request | Result |
|---|---|
| `path` | Full primary-window screenshot |
| `path`, `camera` | Selected active camera's physical viewport |
| `path`, `entity` | Entity crop using the only eligible active camera |
| `path`, `camera`, `entity` | Entity crop using the selected camera |

`padding` is optional for entity capture and defaults to zero physical pixels. It is rejected without `entity` in the extras request or without `entity`/`name` in the MCP request.

The capture implementation has one active request slot. It does not use request IDs, coalescing, per-destination ownership, path generations, or same-target job batches. A different request received while a capture is active returns `A screenshot capture is already in progress`.

## Public boundaries

### `brp_extras/screenshot`

The extras request has these fields:

- `path: String` — required destination; relative paths are resolved from the app's working directory.
- `camera: Option<u64>` — selects a camera viewport or the camera for an entity crop.
- `entity: Option<u64>` — selects an entity crop.
- `padding: Option<u32>` — expands entity bounds in physical pixels; defaults to zero.

Extras accepts an entity ID, not a name. Unknown fields are rejected.

### `brp_extras_screenshot`

The MCP tool adds:

- `name: Option<String>` — case-sensitive exact `Name` selection.
- `port` — selects the BRP application.

`entity` and `name` are mutually exclusive. A name is resolved inside the MCP server through the standard `world.query` path, then only the canonical entity ID is sent to extras. Generic `world_find_entities_by_name` remains independent of `bevy_brp_extras` and supports exact, prefix, suffix, and contains discovery.

## Terminal request flow

The MCP tool sends one BRP request and awaits its terminal result. The Bevy application still performs capture and file work asynchronously:

1. `RemoteMethodSystemId::Watching` invokes the screenshot handler.
2. The first handler call validates the request, resolves its camera and optional entity bounds, creates one Bevy `Screenshot` entity, records one active capture, and returns `Ok(None)`.
3. Bevy emits `ScreenshotCaptured`. The observer moves the single job to a worker that converts the captured image, applies the optional crop, encodes RGB PNG data, and writes a same-directory temporary file.
4. A completion channel returns the owned temporary file to the main world. The channel is created only when the worker starts.
5. The main world atomically persists the temporary file to the requested destination and records either the completed response or a terminal error.
6. The watching handler returns the terminal result. The MCP call then returns to the agent.

The output path is never reported as successful before publication. The worker cannot publish the destination itself.

## Idle behavior

`PendingScreenshotCapture` is created with the plugin and holds an empty active slot. The completion channel is absent until a screenshot event starts a worker.

The completion-ingest and lifecycle systems remain registered, but both use `run_if(screenshot_capture_active)`. When no capture is active, neither system runs. An App test updates multiple idle frames and verifies that the lifecycle frame counter stays unchanged and the completion channel remains absent.

## Camera and crop behavior

A camera-only request validates that the selected camera is active, initialized, and targets a screenshot-capable window, image, or manual texture. It captures the final composited target and crops it to `Camera::physical_viewport_rect()`.

An AABB entity request requires `Aabb` and `GlobalTransform`. It checks inherited visibility, render-layer compatibility, selected-view visibility when available, camera frustum coverage, viewport bounds, and target extent. With no explicit camera, exactly one eligible active camera must be available.

With the default `ui` feature, a complete Bevy UI component set takes precedence over an incidental AABB. UI bounds account for transformed node bounds, inherited clipping, visibility, the computed target camera, viewport, and target extent. Partial UI state is rejected. Disabling default features removes this crate's UI resolver while retaining AABB capture.

Entity output is a crop of the final composited target. It can include overlapping geometry, UI, background, post-processing, and occluders. The entity must contribute bounds visible to the selected camera. Descendants are not included automatically.

## Responses

Every successful response retains the existing fields:

- `success: true`
- `status: "completed"`
- `path`
- `working_directory`
- completion `note`

Entity responses also include `capture_kind: "entity"`, canonical `entity`, optional Bevy `name`, selected `camera`, `bounds_kind` (`"aabb"` or `"ui"`), and physical `rect` coordinates.

The MCP wrapper preserves the raw BRP result. For exact-name requests it adds the resolved entity and requested name to MCP metadata. For direct-ID requests it adds only the entity metadata.

## Test coverage

The extras tests send real watching requests through `RemotePlugin`. Each test uses a new App and proves that no terminal response or destination exists before `ScreenshotCaptured`, then verifies that the returned file is already a complete RGB PNG:

1. Default primary-window request.
2. Camera-only request.
3. Entity-only request with camera inference.
4. Camera-and-entity request with an explicit camera.

Additional focused tests cover request decoding, path normalization, camera validation and inference, AABB projection, UI bounds, viewport and target clipping, visibility, render layers, RGB conversion, crops, temporary-file ownership, publication failure, timeout behavior, and idle system guards.

MCP tests cover the same four request forms at the typed-scope and extras-payload boundary, plus exact-name resolution, missing and duplicate names, public schema fields, raw BRP result preservation, and tool registration.

The runtime integration specification in `.claude/integration_tests/extras_capture.md` validates a terminal full-window request, a terminal camera-only reference request, entity-ID and exact-name crops, explicit and inferred cameras, padding, AABB and UI bounds, PNG pixels, failure paths, and name discovery without extras. The integration tester already permits the existing screenshot and name-discovery tools.

## Implementation map

- `extras/src/screenshot/request.rs` — extras wire decoding and the four request forms.
- `extras/src/screenshot/mod.rs` — watching handler, camera/entity resolution, response construction, and terminal request tests.
- `extras/src/screenshot/capture/pending_screenshot_capture.rs` — one active capture and its terminal lifecycle.
- `extras/src/screenshot/capture/screenshot_job.rs` — image conversion, cropping, encoding, and temporary-file work.
- `extras/src/screenshot/aabb.rs` — projected AABB bounds.
- `extras/src/screenshot/ui.rs` — feature-gated UI bounds.
- `mcp/src/brp_tools/tools/brp_extras_screenshot.rs` — MCP parameters, exact-name composition, and one awaited BRP call.
- `mcp/src/brp_tools/tools/world_find_entities_by_name.rs` — extras-independent name discovery.
- `test-app/examples/extras_plugin/screenshot_fixtures.rs` — runtime screenshot fixtures.
- `.claude/integration_tests/extras_capture.md` — terminal runtime verification.
