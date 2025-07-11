use super::constants::DEFAULT_PROFILE;
use super::support::launch_common::{Example, GenericLaunchHandler, LaunchConfig};
use crate::constants::PARAM_EXAMPLE_NAME;

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>>;

/// Create a new `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new(PARAM_EXAMPLE_NAME, "example name", DEFAULT_PROFILE)
}
