use super::constants::DEFAULT_PROFILE;
use super::support::{Example, GenericLaunchHandler, LaunchConfig};

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>>;

/// Create a new `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new("example_name", "example name", DEFAULT_PROFILE)
}
