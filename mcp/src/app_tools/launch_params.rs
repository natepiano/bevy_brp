use bevy_brp_mcp_macros::ParamStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;

use super::instance_count::InstanceCount;
use super::support::LaunchParams;
use super::support::ToLaunchParams;
use crate::brp_tools::Port;

/// Shared parameters for launching Bevy binaries (apps or examples)
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct LaunchBevyBinaryParams {
    /// Name of the Bevy target to launch (app or example)
    pub target_name:    String,
    /// Build profile to use (debug or release)
    #[to_metadata(skip_if_none)]
    pub profile:        Option<String>,
    /// Path to use when multiple targets with the same name exist
    #[to_metadata(skip_if_none)]
    pub path:           Option<String>,
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port:           Port,
    /// Number of instances to launch (default: 1)
    #[serde(default)]
    pub instance_count: InstanceCount,
}

impl ToLaunchParams for LaunchBevyBinaryParams {
    fn to_launch_params(&self, default_profile: &str) -> LaunchParams {
        LaunchParams {
            target_name:    self.target_name.clone(),
            profile:        self
                .profile
                .clone()
                .unwrap_or_else(|| default_profile.to_string()),
            path:           self.path.clone(),
            port:           self.port,
            instance_count: self.instance_count,
        }
    }
}
