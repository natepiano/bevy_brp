use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::constants::DEFAULT_PROFILE;
use super::support::{App, GenericLaunchHandler, LaunchConfig, LaunchParams, ToLaunchParams};
use crate::brp_tools::Port;

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct LaunchBevyAppParams {
    /// Name of the Bevy app to launch
    pub app_name: String,
    /// Build profile to use (debug or release)
    #[to_metadata(skip_if_none)]
    pub profile:  Option<String>,
    /// Path to use when multiple apps with the same name exist
    #[to_metadata(skip_if_none)]
    pub path:     Option<String>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:     Port,
}

impl ToLaunchParams for LaunchBevyAppParams {
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams {
        LaunchParams {
            target_name: self.app_name.clone(),
            profile:     self
                .profile
                .clone()
                .unwrap_or_else(|| default_profile.to_string()),
            path:        self.path.clone(),
            port:        self.port,
        }
    }
}

/// Handler for launching Bevy apps
pub type LaunchBevyApp = GenericLaunchHandler<LaunchConfig<App>, LaunchBevyAppParams>;

/// Create a new `LaunchBevyApp` handler instance
pub const fn create_launch_bevy_app_handler() -> LaunchBevyApp {
    GenericLaunchHandler::new(DEFAULT_PROFILE)
}
