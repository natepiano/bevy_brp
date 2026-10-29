# Workspace Dev Features

## What it is

A `brp_launch` build for any profile other than `release` adds the features that the workspace root
`Cargo.toml` lists under:

```toml
[workspace.metadata.dev]
features = ["bevy/dynamic_linking"]
```

The list goes to cargo as one comma-joined `--features` argument. Release builds never get it. A
workspace without the key builds exactly as before. A value that is not an array of strings fails a
non-release launch with an `InvalidArgument` error; release ignores it.

## How it works

`mcp/src/app_tools/targets/dev_features.rs` holds the type:

```rust
pub enum DevFeatures {
    Unset,                 // no [workspace.metadata.dev] features key
    Listed(Vec<String>),   // the listed features, in order; may be empty
    Malformed(String),     // "expected an array of strings, found <value>"
}

impl DevFeatures {
    pub(crate) fn from_workspace_metadata(value: &Value) -> Self;
    pub(crate) fn for_launch(&self, workspace_manifest: &Path) -> Result<&[String]>;
}
```

`for_launch` returns empty for `Unset`, the list for `Listed`, and for `Malformed` an
`Error::InvalidArgument` with the text `<workspace manifest path>: [workspace.metadata.dev] features:
<found>`.

Data flow:

1. `CargoDetector::try_from` (`targets/cargo_detector.rs`) runs `cargo metadata` once and parses
   `metadata.workspace_metadata` into a `DevFeatures`, stored on the detector.
2. Each `BevyTarget` the detector yields carries a clone in `dev_features`.
3. `LaunchConfigTrait::ensure_built` (`launch/config.rs`) calls
   `dev_features_for_profile(&target.dev_features, profile, &target.workspace_root)` first, before
   the app freshness check. It returns `&[]` for `release` and otherwise `for_launch` on
   `<workspace_root>/Cargo.toml`.
4. `run_cargo_build` passes the slice to `build_cargo_command` (`launch/build.rs`), which emits:
   `build [--workspace] --bin|--example <name> [--release] [--features a,b] --message-format=json`.
   `--features` is omitted when the slice is empty.

Constants: `WORKSPACE_DEV_TABLE` (`"dev"`), `WORKSPACE_DEV_FEATURES_KEY` (`"features"`) and
`WORKSPACE_DEV_FEATURES_LABEL` in `app_tools/constants.rs`; `CARGO_FEATURES_FLAG` in
`launch/constants.rs`. `mcp/help_text/brp_launch.txt` documents the behavior for agents.

## Invariants

- Release builds never add dev features, and a malformed value never fails a release launch.
- Every non-release profile, custom profiles included, adds the listed features.
- The key is read once per `CargoDetector` from `cargo metadata`; no TOML file is parsed directly.
- List order is kept and duplicates are not removed.
- `--features` comes after target selection (and `--release`) and before `--message-format=json`,
  in both workspace and package build scopes.
- A malformed value fails the launch even when the app binary is fresh, because the profile check
  runs before the freshness check.

## Gotchas

- The launched binary runs directly with no library path set. With `bevy/dynamic_linking` it finds
  the Bevy dylib only through rpaths that the app's own `build.rs` adds.
- In a package-scope build (no `--workspace`, used when a target name occurs in more than one
  member), a `<crate>/<feature>` entry may fail if the selected member does not depend on that
  crate. Unverified.
- The app freshness check compares mtimes only and does not know which features a binary was built
  with. A binary from a plain `cargo build` without the dev features that is newer than every input
  launches as-is. Editing the workspace `Cargo.toml` makes it stale, since that file is a freshness
  input.

## Why

A launch shares the dylib set that the workspace's own dev builds use with `bevy/dynamic_linking`,
so a BRP launch does not rebuild Bevy statically.
