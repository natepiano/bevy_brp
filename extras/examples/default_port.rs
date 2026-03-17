//! Default HTTP configuration
//!
//! Uses the `BRP_EXTRAS_PORT` environment variable if set, otherwise port 15702.
//!
//! ```sh
//! # Use default port 15702
//! cargo run --example default_port
//!
//! # Override via environment variable
//! BRP_EXTRAS_PORT=9000 cargo run --example default_port
//! ```

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin::default())
        .run();
}
