use super::constants::DEFAULT_PROFILE;
use super::launch_params::LaunchBevyBinaryParams;
use super::support::Example;
use super::support::GenericLaunchHandler;
use super::support::LaunchConfig;

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>, LaunchBevyBinaryParams>;

/// Create a new `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new(DEFAULT_PROFILE)
}
