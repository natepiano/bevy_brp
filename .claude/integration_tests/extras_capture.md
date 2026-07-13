# BRP Extras Capture and Diagnostics Tests

## Objective

Prove terminal full-window and entity screenshot capture, MCP-local name resolution,
camera selection, exact RGB crop output, error behavior, diagnostics, and name
discovery when `bevy_brp_extras` is absent.

## Runner-Managed App Context

The runner pre-launches both app instances and supplies one isolated port for each
label:

- **extras_app**: `extras_plugin`, with `bevy_brp_extras`
- **no_extras_app**: `no_extras_plugin`, with standard BRP only

Use `[extras_app port]` or `[no_extras_app port]` on every MCP call as directed.
Do not launch, stop, or restart either app.

Use a distinct absolute destination under `<cwd>` for every screenshot call. Include
the app label, assigned port, and case name in each filename, for example
`<cwd>/extras_capture_extras_app_[extras_app port]_full.png`. Before every screenshot
call except the publication-failure case, remove any previous file with the exact
cleanup command and immediately assert absence with:

```text
bash .claude/scripts/integration_tests/cleanup_screenshots.sh <absolute_path>
python3 .claude/scripts/integration_tests/extras_assert_png.py absent <absolute_path>
```

Never send `capture_id`. Assert that `capture_id` is absent from every public request
and every success or error response inspected below. A successful screenshot call is
terminal; validate its returned file immediately and never poll the path.

## Shared Screenshot Success Assertions

For every successful `mcp__brp__brp_extras_screenshot` call:

- Assert top-level `status` is `"success"`.
- Assert raw BRP fields remain under top-level `result`.
- Assert `result.success` is `true`, `result.status` is `"completed"`, `result.path`
  is the requested absolute destination, `result.note` is
  `"Screenshot capture completed and the PNG was published."`, and
  `result.working_directory` is `<cwd>/test-app`.
- Run the PNG helper's `present` and `dimensions` modes after the MCP response.
  For the primary-window smoke capture, first perform the mandatory post-capture
  `Window` read and stable-size comparison in step 2; that read must precede the
  helper calls. Assert `dimensions` reports `RGB`, proving the complete file uses
  the expected three-channel output. Use `nonuniform` for every retained offscreen
  reference image, but not for the full-window smoke capture.
- For an entity capture, assert `result.capture_kind` is `"entity"`,
  `result.entity` is the canonical selected entity ID, and `metadata.entity` is the
  same ID.
- Only a screenshot requested with `name` may contain `metadata.name`. For a
  name-selected capture it must equal the requested exact name. For a direct-ID
  capture it must be absent, even when `result.name` reports the entity's Bevy
  `Name`.

For every entity crop, assert `result.rect` exactly matches the case's
`(x, y, width, height)` below. Run the helper's `crop` mode against the reference
captured during the same unchanged camera epoch, always supplying reference origin
`16 12`.

Use these exact zero-padding rectangles both when inspecting `result.rect` and when
supplying the helper's crop origin and dimensions:

| Fixture | `(x, y, width, height)` |
|---|---|
| `Screenshot2dUiReference` | `(16, 12, 224, 168)` |
| `NatesList` | `(40, 32, 64, 48)` |
| `ScreenshotRotatedClippedUi` | `(132, 40, 32, 56)` |
| `Screenshot2dAabb` | `(106, 98, 12, 60)` |
| `Screenshot2dAabb` with padding 4 | `(102, 94, 20, 68)` |
| `Screenshot3dReference` | `(16, 12, 224, 168)` |
| `Screenshot3dAabb` | `(162, 90, 12, 48)` |

## Test Steps

### 1. Resolve all extras-app fixture IDs before capture

On `[extras_app port]`, call `mcp__brp__world_find_entities_by_name` with
`match_mode: "exact"` for each name below. Store the returned canonical IDs and
verify results are sorted by entity ID:

- Exactly one each: `ScreenshotPrimaryWindowCamera`,
  `ScreenshotPrimaryWindowTarget`, `Screenshot2dUiCamera`, `Screenshot3dCamera`,
  `Screenshot2dUiReference`, `NatesList`,
  `ScreenshotRotatedClippedUi`, `Screenshot2dAabb`, `Screenshot3dReference`,
  `Screenshot3dAabb`, `ScreenshotPartialUi`, `ScreenshotUnsupported`,
  `ScreenshotHiddenUi`, `ScreenshotHiddenAabb`, and `ScreenshotDisjointLayer`.
- Exactly two for `ScreenshotDuplicateName`; store both IDs and assert ascending
  entity-ID order.

Also query exact lowercase `nateslist` and assert zero matches, proving exact-name
matching is case-sensitive.

Every camera epoch change below uses three
`mcp__brp__world_mutate_components` calls on `[extras_app port]`, one per stored
camera ID, with component `bevy_camera::camera::Camera`, path `.is_active`, and a
JSON boolean `value`. Assert each mutation succeeds before continuing.

### 2. Primary-window smoke epoch

Set the cameras in this order and to these values:

1. `ScreenshotPrimaryWindowCamera`: `true`
2. `Screenshot2dUiCamera`: `false`
3. `Screenshot3dCamera`: `false`

After all three camera mutations succeed, prepare the unique full-window destination
with the normal cleanup followed immediately by the `absent` assertion. Then,
immediately before capture, call `mcp__brp__world_get_components` on
`[extras_app port]` for the stored `ScreenshotPrimaryWindowTarget` ID and component
`bevy_window::window::Window`. Store `resolution.physical_width` as
`pre_capture_width` and `resolution.physical_height` as `pre_capture_height`; require
both values to be positive integers.

Capture without `entity`, `name`, `camera`, or `padding` to that destination on
`[extras_app port]`. Immediately after the terminal screenshot call returns, before
running a PNG helper or making any other BRP call, read the same component from the
same stored target ID with `mcp__brp__world_get_components`. Store the corresponding
values as `post_capture_width` and `post_capture_height`, again requiring positive
integers. Require `pre_capture_width == post_capture_width` and
`pre_capture_height == post_capture_height`; a mismatch is an explicit resize-race
failure.

- Apply the shared screenshot success assertions, using the exact helper sequence
  below for their PNG-helper portion.
- Assert `metadata.entity` and `metadata.name` are absent.
- Assert no entity-only fields (`capture_kind`, `entity`, `name`, `camera`,
  `bounds_kind`, or `rect`) were added to `result`.
- After the stable-size comparison, run these authorized helper forms in this exact
  order, substituting the stored positive integer values for the final two arguments:

  ```text
  python3 .claude/scripts/integration_tests/extras_assert_png.py present <full_window_path>
  python3 .claude/scripts/integration_tests/extras_assert_png.py dimensions <full_window_path> <pre_capture_width> <pre_capture_height>
  ```

  Require the dimensions result to report `RGB`. The dimensions assertion proves the
  complete PNG exactly matches the stable live primary-window physical dimensions.
  Do not assert nonuniform content: platforms that stop presenting a minimized,
  hidden, or fully occluded primary-window surface may legitimately produce a black
  image.

### 3. 2D/UI epoch and reference

Set the cameras in this order and to these values:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `true`
3. `Screenshot3dCamera`: `false`

Keep this camera state unchanged through all positive 2D/UI captures and the 2D/UI
negative cases.

Capture `Screenshot2dUiReference` by direct canonical ID with no `padding` or
explicit `camera`.

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"ui"`, `result.camera` is the stored
  `Screenshot2dUiCamera` ID, and `result.rect` is
  `{ "x": 16, "y": 12, "width": 224, "height": 168 }`.
- Assert `metadata.name` is absent; `result.name` may be
  `"Screenshot2dUiReference"`.
- Assert the PNG is `224x168` and nonuniform.
- Run marker checks with image origin `(16, 12)` for yellow `(255, 255, 0)` at
  target pixels `(52, 44)` and `(112, 128)`, and magenta `(255, 0, 255)` at
  `(100, 56)`.

Retain this PNG as the reference for every later 2D/UI crop.

### 4. One-call exact-name UI capture

Capture with `name: "NatesList"`, omitting `entity`, `camera`, and `padding`.
This must be one screenshot tool call: do not replace it with a caller-side name
lookup plus an ID request.

- Apply the shared screenshot success assertions.
- Assert `metadata.entity` is the stored `NatesList` ID and `metadata.name` is
  `"NatesList"`.
- Assert UI precedence through `result.bounds_kind: "ui"`.
- Assert `result.camera` is the stored `Screenshot2dUiCamera` ID.
- Assert the final clipped `result.rect` is
  `{ "x": 40, "y": 32, "width": 64, "height": 48 }`.
- Assert the PNG is `64x48`.
- Run marker checks with image origin `(40, 32)` for yellow `(255, 255, 0)` at
  `(52, 44)` and magenta `(255, 0, 255)` at `(100, 56)`.
- Compare all crop pixels to the 2D/UI reference rectangle `(40, 32, 64, 48)`
  using reference origin `(16, 12)`.

### 5. Direct-ID UI capture

Capture the same `NatesList` entity using its canonical ID and explicit
`padding: 0`, without `name` or `camera`.

- Apply the shared screenshot success assertions.
- Assert the same UI camera, bounds kind, `64x48` dimensions, rectangle, marker
  pixels, and reference-crop identity as the name-selected capture.
- Assert `metadata.entity` is present and `metadata.name` is absent.
- Permit `result.name: "NatesList"`; it is raw extras data, not synthesized MCP
  name metadata.

### 6. Offset viewport and clipped UI capture

Capture `ScreenshotRotatedClippedUi` by direct canonical ID with default padding.

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"ui"`, `result.camera` is the stored 2D/UI
  camera ID, and `result.rect` is
  `{ "x": 132, "y": 40, "width": 32, "height": 56 }`.
- Assert the PNG is `32x56`.
- Compare every pixel to the 2D/UI reference rectangle `(132, 40, 32, 56)` with
  reference origin `(16, 12)`. This proves viewport offset, transformed containing
  pixels, and UI clipping all use physical target coordinates.

### 7. Generic contains discovery, default padding, explicit zero, and padding four

Call `mcp__brp__world_find_entities_by_name` on `[extras_app port]` with
`name: "2dAabb"` and `match_mode: "contains"`. Assert it returns exactly the stored
`Screenshot2dAabb` canonical ID, then use that ID for all three captures:

1. Omit `padding` and `camera`. Assert AABB bounds, the stored 2D/UI camera ID,
   rectangle `(106, 98, 12, 60)`, dimensions `12x60`, yellow at target pixel
   `(112, 128)` using image origin `(106, 98)`, and exact equality with that
   reference rectangle.
2. Send `padding: 0` and omit `camera`. Assert the identical rectangle,
   dimensions, marker, and reference pixels.
3. Send `padding: 4` and explicit `camera` equal to the stored
   `Screenshot2dUiCamera` ID. Assert rectangle `(102, 94, 20, 68)`, dimensions
   `20x68`, yellow at target pixel `(112, 128)` using image origin `(102, 94)`,
   and exact equality with that reference rectangle.

For all three, apply the shared screenshot success assertions, assert
`result.bounds_kind` is `"aabb"`, and assert `metadata.name` is absent.

### 8. 2D/UI negative cases

Give every call below a different absolute PNG path. Clean that path and run the
helper's `absent` mode immediately before the screenshot call. After the expected
error, run `absent` again to prove no destination was published.

1. Capture with `name: "ScreenshotDuplicateName"`. Assert top-level status
   `"error"`, the message identifies both stored matching IDs in ascending order,
   and it directs callers to retry with `entity` or use generic name discovery.
2. Capture the stored `ScreenshotPartialUi` ID. Assert JSON-RPC code `-32602` and
   error text containing `partially initialized UI bounds`.
3. Capture the stored `ScreenshotUnsupported` ID. Assert code `-32602` and text
   stating that the entity does not have an `Aabb` component.
4. Capture the stored `ScreenshotHiddenUi` ID. Assert code `-32602` and text
   stating that the screenshot entity is hidden.
5. Capture the stored `ScreenshotHiddenAabb` ID with explicit 2D/UI camera ID.
   Assert code `-32602` and text stating that the screenshot entity is hidden.
6. Capture the stored `ScreenshotDisjointLayer` ID with explicit 2D/UI camera ID.
   Assert code `-32602` and text stating that the entity and camera do not share a
   `RenderLayers` entry.
7. Capture `NatesList` by direct ID while explicitly requesting the stored 3D
   camera ID. Assert code `-32602` and text stating that the UI entity targets a
   different camera than the requested camera.
8. Send both the stored `NatesList` `entity` and `name: "NatesList"`. Assert a
   local MCP error explaining that the selectors are mutually exclusive.
9. Send `camera` without `entity` or `name`. Assert a local MCP error explaining
   that camera requires an entity or name selector.
10. Send `padding: 0` without `entity` or `name`. Assert a local MCP error
    explaining that padding requires an entity or name selector.

For raw BRP errors in cases 2-7, assert `metadata.method` is
`"brp_extras/screenshot"`, `metadata.port` is `[extras_app port]`, and
`metadata.code` is `-32602`.

### 9. 3D epoch and reference

Set the cameras in this order and to these values:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `false`
3. `Screenshot3dCamera`: `true`

Capture `Screenshot3dReference` by direct canonical ID without padding or an
explicit camera.

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"aabb"`, `result.camera` is the stored
  `Screenshot3dCamera` ID, and `result.rect` is
  `{ "x": 16, "y": 12, "width": 224, "height": 168 }`.
- Assert the PNG is `224x168` and nonuniform.
- Assert yellow `(255, 255, 0)` at target pixel `(168, 114)` using image origin
  `(16, 12)`.

Retain this PNG as the reference for every later 3D crop.

Capture `Screenshot3dAabb` twice by direct ID:

1. Omit `camera` and `padding`.
2. Send explicit `camera` equal to the stored `Screenshot3dCamera` ID and
   `padding: 0`.

For both calls:

- Apply the shared screenshot success assertions.
- Assert `result.bounds_kind` is `"aabb"`, `result.camera` is the stored 3D
  camera ID, and `result.rect` is
  `{ "x": 162, "y": 90, "width": 12, "height": 48 }`.
- Assert the PNG is `12x48`.
- Assert yellow `(255, 255, 0)` at target pixel `(168, 114)` using image origin
  `(162, 90)`.
- Compare every pixel to the 3D reference rectangle `(162, 90, 12, 48)` with
  reference origin `(16, 12)`.
- Assert `metadata.name` is absent.

### 10. Both-active camera ambiguity

Set the cameras in this order and to these values for this case only:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `true`
3. `Screenshot3dCamera`: `true`

Clean a unique path, assert it is absent, and capture `Screenshot2dAabb` by direct
ID without an explicit camera.

- Assert top-level status is `"error"`, `metadata.method` is
  `"brp_extras/screenshot"`, `metadata.port` is `[extras_app port]`, and
  `metadata.code` is `-32602`.
- Assert `metadata.data.reason` is `"ambiguous_camera"`.
- Assert `metadata.data.camera_candidates` contains exactly the stored 2D/UI and
  3D camera IDs in ascending entity-ID order.
- Assert the output path remains absent.

### 11. Restore the initial 2D-only camera state

Set the cameras in this order and to these values:

1. `ScreenshotPrimaryWindowCamera`: `false`
2. `Screenshot2dUiCamera`: `true`
3. `Screenshot3dCamera`: `false`

Assert every mutation succeeds. This is the required initial state and must be
restored even if a negative case failed.

### 12. Deterministic publication failure

Use the existing `<cwd>/mcp` directory as `path` for a full screenshot on
`[extras_app port]`. This is the only screenshot call exempt from cleanup and the
pre-call `absent` assertion because the directory must already exist.

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

### 13. FPS diagnostics

On `[extras_app port]`, execute `mcp__brp__brp_execute` with method
`brp_extras/get_diagnostics` and no params.

- Assert `result.fps` contains numeric `current`, `average`, and `smoothed`, plus
  `history_len`, `max_history_len`, and `history_duration_secs`.
- Assert `result.frame_time_ms` contains numeric `current`, `average`, and
  `smoothed`.
- Assert `result.frame_count` is numeric, `result.fps.max_history_len` is `120`,
  and `result.fps.current` is positive.

### 14. Standard-BRP name discovery without extras

On `[no_extras_app port]`:

1. Call `mcp__brp__world_find_entities_by_name` with `name: "NatesList"` and
   `match_mode: "exact"`. Assert one entity is returned.
2. Call it with `name: "NoExtrasDuplicate"` and `match_mode: "exact"`. Assert two
   entities are returned in ascending canonical entity-ID order.

Clean a unique no-extras screenshot destination and immediately assert it is absent.
Then call the still-registered `mcp__brp__brp_extras_screenshot` tool with
`name: "NatesList"`, that path, and `[no_extras_app port]`.

- Assert this is an invoked-tool error, not an unavailable-MCP-tool error.
- Assert top-level `status` is `"error"`.
- Assert `metadata.method` is `"brp_extras/screenshot"`, `metadata.code` is
  `-32601`, and `metadata.port` is `[no_extras_app port]`.
- Assert the destination remains absent.

This proves name discovery and exact-name resolution use standard
`world.query`; only the final screenshot BRP method depends on extras.

### 15. Mandatory cleanup and final camera assertion

Before reporting results, always attempt these actions even after an earlier failure:

1. Restore the camera values to primary `false`, 2D/UI `true`, and 3D `false`,
   using the same `world_mutate_components` component and path.
2. Run the exact cleanup command once for every PNG destination used by this test,
   including paths from negative cases.
3. Run the PNG helper's `absent` mode for every destination and assert all are
   absent.
4. Do not remove or alter the `<cwd>/mcp` directory.

## Expected Results

- Full capture is a terminal RGB PNG whose dimensions exactly match the stable
  pre/post-capture live primary-window physical dimensions.
- Every retained 2D/UI and 3D offscreen reference is nonuniform and carries the
  marker and crop-identity assertions below.
- Name-selected and direct-ID UI captures preserve raw BRP result fields and keep
  MCP resolution metadata separate.
- 2D/UI and 3D crops match same-epoch reference pixels at every coordinate.
- Default and explicit zero padding agree; padding four expands to the pinned
  rectangle.
- UI precedence, viewport offset, clipping, explicit cameras, and sorted camera
  ambiguity data are verified.
- Publication failure is terminal code `-32603` and preserves the existing
  directory.
- FPS diagnostics remain valid.
- Standard-BRP name discovery works without extras, while screenshot invocation on
  that app returns BRP method-not-found code `-32601`.
- Every generated path is removed and the app ends in its initial 2D-only camera
  state.

## Failure Criteria

Stop the functional sequence on malformed responses or pixel mismatches, then still
perform the mandatory camera restoration and path cleanup before reporting the
failure.
