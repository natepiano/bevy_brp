use super::constants::DEFAULT_PROFILE;
use super::launch_params::LaunchBevyBinaryParams;
use super::support::App;
use super::support::Example;
use super::support::GenericLaunchHandler;
use super::support::LaunchConfig;

/// Handler for launching Bevy apps
pub type LaunchBevyApp = GenericLaunchHandler<LaunchConfig<App>, LaunchBevyBinaryParams>;

/// Create a `LaunchBevyApp` handler instance
pub const fn create_launch_bevy_app_handler() -> LaunchBevyApp {
    GenericLaunchHandler::new(DEFAULT_PROFILE)
}

/// Handler for launching Bevy examples
pub type LaunchBevyExample = GenericLaunchHandler<LaunchConfig<Example>, LaunchBevyBinaryParams>;

/// Create a `LaunchBevyExample` handler instance
pub const fn create_launch_bevy_example_handler() -> LaunchBevyExample {
    GenericLaunchHandler::new(DEFAULT_PROFILE)
}
