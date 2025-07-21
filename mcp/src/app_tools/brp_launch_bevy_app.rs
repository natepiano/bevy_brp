use schemars::JsonSchema;
use serde::Deserialize;

use super::constants::DEFAULT_PROFILE;
use super::support::{App, GenericLaunchHandler, LaunchConfig};

#[derive(Deserialize, JsonSchema)]
pub struct LaunchBevyAppParams {
    /// Name of the Bevy app to launch
    pub app_name: String,
    /// Build profile to use (debug or release)
    pub profile:  Option<String>,
    /// Path to use when multiple apps with the same name exist
    pub path:     Option<String>,
}

/// Handler for launching Bevy apps
pub type LaunchBevyApp = GenericLaunchHandler<LaunchConfig<App>>;

/// Create a new `LaunchBevyApp` handler instance
pub const fn create_launch_bevy_app_handler() -> LaunchBevyApp {
    GenericLaunchHandler::new("app_name", "app name", DEFAULT_PROFILE)
}
