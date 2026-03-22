//! Explicit port configuration
//!
//! Sets a specific port for HTTP transport. The `BRP_EXTRAS_PORT` environment
//! variable still takes precedence if set.
//!
//! ```sh
//! cargo run --example custom_port
//! ```

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin::with_port(9000).port_in_title(PortDisplay::Always))
        .run();
}
