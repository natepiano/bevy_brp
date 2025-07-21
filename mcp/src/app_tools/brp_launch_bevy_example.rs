use schemars::JsonSchema;
use serde::Deserialize;

use super::constants::DEFAULT_PROFILE;
use super::support::{Example, GenericLaunchHandler, LaunchConfig};

#[derive(Deserialize, JsonSchema)]
pub struct LaunchBevyExampleParams {
    /// Name of the Bevy example to launch
    pub example_name: String,
    /// Build profile to use (debug or release)
    pub profile:      Option<String>,
    /// Path to use when multiple examples with the same name exist
    pub path:         Option<String>,
}

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>>;

/// Create a new `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new("example_name", "example name", DEFAULT_PROFILE)
}
