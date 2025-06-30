use bevy::prelude::*;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    let port = std::env::var("BRP_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(15702);

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(BrpExtrasPlugin)
        .add_systems(Startup, move || {
            println!("Test app from workspace-b running on port {}", port);
        })
        .run();
}