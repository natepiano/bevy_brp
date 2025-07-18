use super::constants::DEFAULT_PROFILE;
use super::support::{App, GenericLaunchHandler, LaunchConfig};

/// Handler for launching Bevy apps
pub type LaunchBevyApp = GenericLaunchHandler<LaunchConfig<App>>;

/// Create a new `LaunchBevyApp` handler instance
pub const fn create_launch_bevy_app_handler() -> LaunchBevyApp {
    GenericLaunchHandler::new("app_name", "app name", DEFAULT_PROFILE)
}
