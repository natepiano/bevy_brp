//! Full control over HTTP transport
//!
//! Provides a pre-configured `RemoteHttpPlugin` directly. This bypasses
//! all port resolution logic — the `BRP_EXTRAS_PORT` environment variable
//! and `with_port()` are not used.
//!
//! Use this when you need to configure options beyond just the port,
//! such as the listen address.
//!
//! ```sh
//! cargo run --example custom_http_plugin
//! ```

use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_remote::http::RemoteHttpPlugin;

fn main() {
    let http_plugin = RemoteHttpPlugin::default()
        .with_port(9000)
        .with_address([0, 0, 0, 0]);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(BrpExtrasPlugin::with_http_plugin(http_plugin))
        .run();
}
