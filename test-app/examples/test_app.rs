//! Minimal example that intentionally shares its name with the `test_app` binary
//! in this same package (`bevy_brp_test_apps`).
//!
//! Cargo allows a `[[bin]]` and `[[example]]` with the same name — bins build to
//! `target/<profile>/test_app` and examples to `target/<profile>/examples/test_app`.
//!
//! This validates that `brp_list_bevy` does not silently deduplicate targets that
//! share a name but differ in kind (app vs example).

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin::new().port_in_title(PortDisplay::Always))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) { commands.spawn(Camera2d); }
