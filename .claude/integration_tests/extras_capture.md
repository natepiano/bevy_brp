# BRP Extras Capture and Diagnostics Tests

## Objective

Prove terminal full-window, camera-viewport, and entity screenshot capture, MCP-local
name resolution, exact RGB crop output, error behavior, diagnostics, and name
discovery when `bevy_brp_extras` is absent.

## Runner-Managed App Context

The runner pre-launches both app instances and supplies one isolated port for each
label:

- **extras_app**: `extras_plugin`, with `bevy_brp_extras`
- **no_extras_app**: `no_extras_plugin`, with standard BRP only

Use `[extras_app port]` or `[no_extras_app port]` on every MCP call as directed.
Do not launch, stop, or restart either app.
The screenshot fixtures are always registered; no environment variable or special
launch mode is required. At startup, the original primary-window camera and the
offscreen 2D/UI fixture camera are both active, while the offscreen 3D fixture camera
is inactive.

## Destination Naming

Every screenshot call uses a distinct absolute destination under `<cwd>` named
`<cwd>/extras_capture_<label>_<port>_<case>.png`, where `<case>` is the case name
given in each step. Because every destination is unique to one case, a destination
proven absent in step 0 and later found present can only have been published by that
case's screenshot call.

## Batched Assertion Protocol

`extras_assert_png.py batch` reads one JSON spec on stdin, runs every entry in
order, prints one line per entry, and stops at the first failure with a nonzero exit
status. Batch every assertion that belongs to a single capture into one call:

```text
python3 .claude/scripts/integration_tests/extras_assert_png.py batch <<'JSON'
{
  "assert": [
    {"mode": "present",    "path": "<abs>"},
    {"mode": "dimensions", "path": "<abs>", "width": 224, "height": 168},
    {"mode": "nonuniform", "path": "<abs>"},
    {"mode": "marker",     "path": "<abs>", "image_x": 16, "image_y": 12,
                           "marker_x": 52, "marker_y": 44, "rgb": [255, 255, 0]},
    {"mode": "crop", "crop": "<abs>", "reference": "<ref abs>",
                     "crop_x": 40, "crop_y": 32,
                     "reference_x": 16, "reference_y": 12,
                     "width": 64, "height": 48}
  ]
}
JSON
```

A `"prepare"` array removes each listed destination and asserts it is absent.
Treat a nonzero exit status from any batch call as a test failure and report the
helper's stderr verbatim. A successful screenshot call is terminal; validate its
returned file immediately and never poll the path.

## Shared Screenshot Success Assertions

For every successful `mcp__brp__brp_extras_screenshot` call:

- Assert top-level `status` is `"success"`.
- Assert raw BRP fields remain under top-level `result`.
- Assert `result.success` is `true`, `result.status` is `"completed"`, `result.path`
  is the requested absolute destination, `result.note` is
  `"Screenshot capture completed and the PNG was published."`, and
  `result.working_directory` is `<cwd>/test-app`.
- Assert `dimensions` reports `RGB`, proving the complete file uses the expected
  three-channel output. Use `nonuniform` for every retained offscreen reference
  image, but not for the full-window smoke capture.
- For an entity capture, assert `result.capture_kind` is `"entity"`,
  `result.entity` is the canonical selected entity ID, and `metadata.entity` is the
  same ID.
- Only a screenshot requested with `name` may contain `metadata.name`. For a
  name-selected capture it must equal the requested exact name. For a direct-ID
  capture it must be absent, even when `result.name` reports the entity's Bevy
  `Name`.

For every entity crop, assert `result.rect` exactly matches the case's
`(x, y, width, height)` below. Run the batch `crop` entry against the reference
captured during the same unchanged camera epoch, always supplying reference origin
`16 12`.

Use these exact zero-padding rectangles both when inspecting `result.rect` and when
supplying the batch crop origin and dimensions:

| Fixture | `(x, y, width, height)` |
|---|---|
| `NatesList` | `(40, 32, 64, 48)` |
| `ScreenshotRotatedClippedUi` | `(132, 40, 32, 56)` |
| `Screenshot2dAabb` | `(106, 98, 12, 60)` |
| `Screenshot2dAabb` with padding 4 | `(102, 94, 20, 68)` |
| `Screenshot3dReference` | `(16, 12, 224, 168)` |
| `Screenshot3dAabb` | `(162, 90, 12, 48)` |

## Camera Epochs

Each camera epoch below uses three `mcp__brp__world_mutate_components` calls on
`[extras_app port]`, one per stored camera ID, with component
`bevy_camera::camera::Camera`, path `.is_active`, and a JSON boolean `value`, applied
in the listed order. Assert each mutation succeeds before continuing.

## Test Steps

### 0. Prepare every destination once

Build the full list of the 22 destinations used by this test: the eleven capture
cases in steps 2-9 (`full`, `2dui_reference`, `nateslist_name`, `nateslist_id`,
`rotated_clipped_ui`, `2daabb_default`, `2daabb_pad0`, `2daabb_pad4`,
`3d_reference`, `3daabb_default`, `3daabb_explicit`), the ten extras-app negative
cases in steps 7 and 9 (`neg_duplicate_name`, `neg_partial_ui`, `neg_unsupported`,
`neg_hidden_ui`, `neg_hidden_aabb`, `neg_disjoint_layer`, `neg_ui_wrong_camera`,
`neg_entity_and_name`, `neg_padding_without_selector`, `neg_ambiguous_camera`), and
the no-extras case in step 12 (`no_extras_nateslist`, named with the
`no_extras_app` label and `[no_extras_app port]`).

Issue one batch call whose `"prepare"` array holds all 22 absolute paths. Assert the
call exits zero and prints a `prepared:` line for every path. This is the only
cleanup required before any capture; `<cwd>/mcp` is never a destination and must
never appear in this list.

### 1. Resolve all extras-app fixture IDs

Call `mcp__brp__world_query` once on `[extras_app port]` with
`data: {components: ["bevy_ecs::name::Name"]}` and no filter. From that single
response build a name-to-entity map and store the canonical IDs for:
`ScreenshotPrimaryWindowCamera`, `ScreenshotPrimaryWindowTarget`,
`Screenshot2dUiCamera`, `Screenshot3dCamera`, `NatesList`,
`ScreenshotRotatedClippedUi`, `Screenshot2dAabb`, `Screenshot3dReference`,
`Screenshot3dAabb`, `ScreenshotPartialUi`, `ScreenshotUnsupported`,
`ScreenshotHiddenUi`, `ScreenshotHiddenAabb`, and `ScreenshotDisjointLayer`.
Assert each of those names matches exactly one entity in the response.

Then make exactly three `mcp__brp__world_find_entities_by_name` calls on
`[extras_app port]` to pin the tool's own exact-match contract:

1. `name: "NatesList"`, `match_mode: "exact"` — assert exactly one result and that
   its ID equals the ID resolved from the query above.
2. `name: "ScreenshotDuplicateName"`, `match_mode: "exact"` — assert exactly two
   results in ascending entity-ID order; store both IDs.
3. `name: "nateslist"`, `match_mode: "exact"` — assert zero matches, proving
   exact-name matching is case-sensitive.

### 2. Primary-window smoke epoch

Apply the camera epoch:

1. `ScreenshotPrimaryWindowCamera`: `true`
2. `Screenshot2dUiCamera`: `false`
3. `Screenshot3dCamera`: `false`

Immediately before capture, call `mcp__brp__world_get_components` on
`[extras_app port]` for the stored `ScreenshotPrimaryWindowTarget` ID and component
`bevy_window::window::Window`. Store `resolution.physical_width` as
`pre_capture_width` and `resolution.physical_height` as `pre_capture_height`;
require both values to be positive integers.

Capture case `full` without `entity`, `name`, `camera`, or `padding` on
`[extras_app port]`. Immediately after the terminal screenshot call returns, before
running any helper or making any other BRP call, read the same component from the
same stored target ID again. Store the corresponding values as `post_capture_width`
and `post_capture_height`, again requiring positive integers. Require
`pre_capture_width == post_capture_width` and
`pre_capture_height == post_capture_height`; a mismatch is an explicit resize-race
failure.

- Apply the shared screenshot success assertions.
- Assert `metadata.entity` and `metadata.name` are absent.
- Assert no entity-only fields (`capture_kind`, `entity`, `name`, `camera`,
  `bounds_kind`, or `rect`) were added to `result`.
- After the stable-size comparison, run one batch call asserting `present` and
  `dimensions` at the stored `pre_capture_width` by `pre_capture_height`. Require
  the dimensions line to report `RGB`, proving the complete PNG exactly matches the
  stable live primary-window physical dimensions. Do not assert nonuniform content:
  platforms that stop presenting a minimized, hidden, or fully occluded
  primary-window surface may legitimately produce a black image.

### 3. 2D/UI epoch and reference

Apply the camera epoch:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `true`
3. `Screenshot3dCamera`: `false`

Keep this camera state unchanged through all positive 2D/UI captures in steps 3-6
and the 2D/UI negative cases in step 7.

Capture case `2dui_reference` for the active `Screenshot2dUiCamera` viewport by
supplying only its canonical camera ID, with no `entity`, `name`, or `padding`.

- Apply the shared screenshot success assertions.
- Assert `metadata.entity` and `metadata.name` are absent.
- Assert no entity-only fields (`capture_kind`, `entity`, `name`, `camera`,
  `bounds_kind`, or `rect`) were added to `result`.
- In one batch call assert `present`, `dimensions` `224x168`, `nonuniform`, and
  three markers with image origin `(16, 12)`: yellow `(255, 255, 0)` at target
  pixels `(52, 44)` and `(112, 128)`, and magenta `(255, 0, 255)` at `(100, 56)`.

Retain this PNG as the reference for every later 2D/UI crop.

### 4. Name and direct-ID UI captures

Capture case `nateslist_name` with `name: "NatesList"`, omitting `entity`,
`camera`, and `padding`. This must be one screenshot tool call: do not replace it
with a caller-side name lookup plus an ID request.

- Apply the shared screenshot success assertions.
- Assert `metadata.entity` is the stored `NatesList` ID and `metadata.name` is
  `"NatesList"`.
- Assert UI precedence through `result.bounds_kind: "ui"`.
- Assert `result.camera` is the stored `Screenshot2dUiCamera` ID.
- Assert the final clipped `result.rect` is
  `{ "x": 40, "y": 32, "width": 64, "height": 48 }`.

Then capture case `nateslist_id` for the same entity using its canonical ID and
explicit `padding: 0`, without `name` or `camera`.

- Apply the shared screenshot success assertions.
- Assert the same UI camera, bounds kind, and rectangle as the name-selected
  capture.
- Assert `metadata.entity` is present and `metadata.name` is absent.
- Permit `result.name: "NatesList"`; it is raw extras data, not synthesized MCP
  name metadata.

Verify both PNGs in one batch call: for each path assert `present`, `dimensions`
`64x48`, markers with image origin `(40, 32)` for yellow `(255, 255, 0)` at
`(52, 44)` and magenta `(255, 0, 255)` at `(100, 56)`, and `crop` against the
2D/UI reference rectangle `(40, 32, 64, 48)` with reference origin `(16, 12)`.

### 5. Offset viewport and clipped UI capture

Capture case `rotated_clipped_ui` for `ScreenshotRotatedClippedUi` by direct
canonical ID with default padding.

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"ui"`, `result.camera` is the stored 2D/UI
  camera ID, and `result.rect` is
  `{ "x": 132, "y": 40, "width": 32, "height": 56 }`.
- In one batch call assert `present`, `dimensions` `32x56`, and `crop` against the
  2D/UI reference rectangle `(132, 40, 32, 56)` with reference origin `(16, 12)`.
  This proves viewport offset, transformed containing pixels, and UI clipping all
  use physical target coordinates.

### 6. Generic contains discovery, default padding, explicit zero, and padding four

Call `mcp__brp__world_find_entities_by_name` on `[extras_app port]` with
`name: "2dAabb"` and `match_mode: "contains"`. Assert it returns exactly the stored
`Screenshot2dAabb` canonical ID, then use that ID for all three captures:

1. Case `2daabb_default`: omit `padding` and `camera`. Assert rectangle
   `(106, 98, 12, 60)`.
2. Case `2daabb_pad0`: send `padding: 0` and omit `camera`. Assert the identical
   rectangle.
3. Case `2daabb_pad4`: send `padding: 4` and explicit `camera` equal to the stored
   `Screenshot2dUiCamera` ID. Assert rectangle `(102, 94, 20, 68)`.

For all three, apply the shared screenshot success assertions, assert
`result.bounds_kind` is `"aabb"`, assert `result.camera` is the stored 2D/UI camera
ID, and assert `metadata.name` is absent.

Verify all three PNGs in one batch call:

- `2daabb_default` and `2daabb_pad0`: `present`, `dimensions` `12x60`, yellow
  `(255, 255, 0)` at target pixel `(112, 128)` with image origin `(106, 98)`, and
  `crop` against reference rectangle `(106, 98, 12, 60)`.
- `2daabb_pad4`: `present`, `dimensions` `20x68`, yellow at target pixel
  `(112, 128)` with image origin `(102, 94)`, and `crop` against reference
  rectangle `(102, 94, 20, 68)`.

All crops use the 2D/UI reference with reference origin `(16, 12)`.

### 7. 2D/UI negative cases

Run these nine calls back to back, each with its own already-prepared destination.
Each is expected to fail, so none may publish a file.

1. Case `neg_duplicate_name`: capture with `name: "ScreenshotDuplicateName"`.
   Assert top-level status `"error"`, the message identifies both stored matching
   IDs in ascending order, and it directs callers to retry with `entity` or use
   generic name discovery.
2. Case `neg_partial_ui`: capture the stored `ScreenshotPartialUi` ID. Assert
   JSON-RPC code `-32602` and error text containing `partially initialized UI
   bounds`.
3. Case `neg_unsupported`: capture the stored `ScreenshotUnsupported` ID. Assert
   code `-32602` and text stating that the entity does not have an `Aabb`
   component.
4. Case `neg_hidden_ui`: capture the stored `ScreenshotHiddenUi` ID. Assert code
   `-32602` and text stating that the screenshot entity is hidden.
5. Case `neg_hidden_aabb`: capture the stored `ScreenshotHiddenAabb` ID with
   explicit 2D/UI camera ID. Assert code `-32602` and text stating that the
   screenshot entity is hidden.
6. Case `neg_disjoint_layer`: capture the stored `ScreenshotDisjointLayer` ID with
   explicit 2D/UI camera ID. Assert code `-32602` and text stating that the entity
   and camera do not share a `RenderLayers` entry.
7. Case `neg_ui_wrong_camera`: capture `NatesList` by direct ID while explicitly
   requesting the stored 3D camera ID. Assert code `-32602` and text stating that
   the UI entity targets a different camera than the requested camera.
8. Case `neg_entity_and_name`: send both the stored `NatesList` `entity` and
   `name: "NatesList"`. Assert a local MCP error explaining that the selectors are
   mutually exclusive.
9. Case `neg_padding_without_selector`: send `padding: 0` without `entity` or
   `name`. Assert a local MCP error explaining that padding requires an entity or
   name selector.

For raw BRP errors in cases 2-7, assert `metadata.method` is
`"brp_extras/screenshot"`, `metadata.port` is `[extras_app port]`, and
`metadata.code` is `-32602`.

After all nine calls return, issue one batch call asserting `absent` for all nine
destinations, proving no failed capture published a file.

### 8. 3D epoch and reference

Apply the camera epoch:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `false`
3. `Screenshot3dCamera`: `true`

Capture case `3d_reference` for `Screenshot3dReference` by direct canonical ID
without padding or an explicit camera.

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"aabb"`, `result.camera` is the stored
  `Screenshot3dCamera` ID, and `result.rect` is
  `{ "x": 16, "y": 12, "width": 224, "height": 168 }`.
- In one batch call assert `present`, `dimensions` `224x168`, `nonuniform`, and
  yellow `(255, 255, 0)` at target pixel `(168, 114)` with image origin `(16, 12)`.

Retain this PNG as the reference for every later 3D crop.

Then capture `Screenshot3dAabb` twice by direct ID:

1. Case `3daabb_default`: omit `camera` and `padding`.
2. Case `3daabb_explicit`: send explicit `camera` equal to the stored
   `Screenshot3dCamera` ID and `padding: 0`.

For both calls:

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"aabb"`, `result.camera` is the stored 3D
  camera ID, and `result.rect` is
  `{ "x": 162, "y": 90, "width": 12, "height": 48 }`.
- Assert `metadata.name` is absent.

Verify both PNGs in one batch call: for each assert `present`, `dimensions`
`12x48`, yellow `(255, 255, 0)` at target pixel `(168, 114)` with image origin
`(162, 90)`, and `crop` against the 3D reference rectangle `(162, 90, 12, 48)` with
reference origin `(16, 12)`.

### 9. Both-active camera ambiguity

Apply the camera epoch for this case only:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `true`
3. `Screenshot3dCamera`: `true`

Capture case `neg_ambiguous_camera` for `Screenshot2dAabb` by direct ID without an
explicit camera.

- Assert top-level status is `"error"`, `metadata.method` is
  `"brp_extras/screenshot"`, `metadata.port` is `[extras_app port]`, and
  `metadata.code` is `-32602`.
- Assert `metadata.data.reason` is `"ambiguous_camera"`.
- Assert `metadata.data.camera_candidates` contains exactly the stored 2D/UI and
  3D camera IDs in ascending entity-ID order.
- Assert the output path remains absent with a batch `absent` entry.

### 10. Deterministic publication failure

Use the existing `<cwd>/mcp` directory as `path` for a full screenshot on
`[extras_app port]`. This is the only screenshot call exempt from preparation and
the `absent` assertion because the directory must already exist.

- Before the call, execute the exact Bash command `test -d <cwd>/mcp`. Assert its
  exit status is zero, proving that `<cwd>/mcp` exists as a directory.
- Assert top-level status is `"error"`, `metadata.method` is
  `"brp_extras/screenshot"`, `metadata.port` is `[extras_app port]`, and
  `metadata.code` is `-32603`.
- Assert the error text contains `Failed to publish screenshot` and names
  `<cwd>/mcp`.
- After the call, execute the exact Bash command `test -d <cwd>/mcp` again. Assert
  its exit status is zero, proving that `<cwd>/mcp` remains a directory and was
  not replaced by a file.

### 11. FPS diagnostics

On `[extras_app port]`, execute `mcp__brp__brp_execute` with method
`brp_extras/get_diagnostics` and no params.

- Assert `result.fps` contains numeric `current`, `average`, and `smoothed`, plus
  `history_len`, `max_history_len`, and `history_duration_secs`.
- Assert `result.frame_time_ms` contains numeric `current`, `average`, and
  `smoothed`.
- Assert `result.frame_count` is numeric, `result.fps.max_history_len` is `120`,
  and `result.fps.current` is positive.

### 12. Standard-BRP name discovery without extras

On `[no_extras_app port]`:

1. Call `mcp__brp__world_find_entities_by_name` with `name: "NatesList"` and
   `match_mode: "exact"`. Assert one entity is returned.
2. Call it with `name: "NoExtrasDuplicate"` and `match_mode: "exact"`. Assert two
   entities are returned in ascending canonical entity-ID order.

Then call the still-registered `mcp__brp__brp_extras_screenshot` tool with
`name: "NatesList"`, the already-prepared `no_extras_nateslist` path, and
`[no_extras_app port]`.

- Assert this is an invoked-tool error, not an unavailable-MCP-tool error.
- Assert top-level `status` is `"error"`.
- Assert `metadata.method` is `"brp_extras/screenshot"`, `metadata.code` is
  `-32601`, and `metadata.port` is `[no_extras_app port]`.
- Assert the destination remains absent.

This proves name discovery and exact-name resolution use standard
`world.query`; only the final screenshot BRP method depends on extras.

### 13. Mandatory cleanup and final camera assertion

Before reporting results, always attempt these actions even after an earlier
failure, and continue through every action even if one of them fails:

1. Restore the camera values to primary `true`, 2D/UI `true`, and 3D `false`,
   using the same `world_mutate_components` component and path. This is the
   required initial state.
2. Issue one batch call whose `"prepare"` array holds all 22 destinations from
   step 0. This removes every generated PNG and asserts each is absent in the same
   call.
3. Do not remove or alter the `<cwd>/mcp` directory, and never include it in a
   prepare list.

## Expected Results

- Full capture is a terminal RGB PNG whose dimensions exactly match the stable
  pre/post-capture live primary-window physical dimensions.
- The camera-only 2D/UI reference and retained 3D reference are nonuniform and carry
  the marker and crop-identity assertions above.
- Name-selected and direct-ID UI captures preserve raw BRP result fields and keep
  MCP resolution metadata separate.
- 2D/UI and 3D crops match same-epoch reference pixels at every coordinate.
- Default and explicit zero padding agree; padding four expands to the pinned
  rectangle.
- UI precedence, viewport offset, clipping, explicit cameras, and sorted camera
  ambiguity data are verified.
- Every failed capture leaves its destination absent.
- Publication failure is terminal code `-32603` and preserves the existing
  directory.
- FPS diagnostics remain valid.
- Standard-BRP name discovery works without extras, while screenshot invocation on
  that app returns BRP method-not-found code `-32601`.
- Every generated path is removed and the app ends with its original window UI and
  offscreen 2D/UI fixture cameras active and the offscreen 3D camera inactive.

## Failure Criteria

Stop the functional sequence on malformed responses, nonzero batch exit statuses, or
pixel mismatches, then still perform the mandatory camera restoration and path
cleanup in step 13 before reporting the failure.
