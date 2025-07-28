use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::constants::DEFAULT_PROFILE;
use super::support::{Example, GenericLaunchHandler, LaunchConfig, LaunchParams, ToLaunchParams};
use crate::brp_tools::Port;

#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct LaunchBevyExampleParams {
    /// Name of the Bevy example to launch
    pub example_name: String,
    /// Build profile to use (debug or release)
    #[to_metadata(skip_if_none)]
    pub profile:      Option<String>,
    /// Path to use when multiple examples with the same name exist
    #[to_metadata(skip_if_none)]
    pub path:         Option<String>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:         Port,
}

impl ToLaunchParams for LaunchBevyExampleParams {
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams {
        LaunchParams {
            target_name: self.example_name.clone(),
            profile:     self
                .profile
                .clone()
                .unwrap_or_else(|| default_profile.to_string()),
            path:        self.path.clone(),
            port:        self.port,
        }
    }
}

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>, LaunchBevyExampleParams>;

/// Create a new `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new(DEFAULT_PROFILE)
}
