use super::constants::DEFAULT_PROFILE;
use super::support::launch_common::{App, GenericLaunchHandler, LaunchConfig};
use crate::constants::PARAM_APP_NAME;

/// Handler for launching Bevy apps
pub type LaunchBevyApp = GenericLaunchHandler<LaunchConfig<App>>;

/// Create a new `LaunchBevyApp` handler instance
pub const fn create_launch_bevy_app_handler() -> LaunchBevyApp {
    GenericLaunchHandler::new(PARAM_APP_NAME, "app name", DEFAULT_PROFILE)
}
