//! Screenshot request handling for BRP extras.

#[cfg(not(target_arch = "wasm32"))]
mod capture;
#[cfg(not(target_arch = "wasm32"))]
mod request;

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use bevy::ecs::system::In;
use bevy::prelude::App;
use bevy::prelude::Plugin;
use bevy::prelude::World;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::screenshot::Screenshot;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::PrimaryWindow;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use self::capture::CapturePlugin;
#[cfg(not(target_arch = "wasm32"))]
use self::capture::PendingScreenshotCaptures;
#[cfg(not(target_arch = "wasm32"))]
use self::request::ScreenshotRequest;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::IMAGE_EXTENSION_PNG;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::PARAM_PATH;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_NOTE_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_STATUS_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_SUCCESS_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::RESPONSE_WORKING_DIRECTORY_FIELD;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_CAPTURE_NOTE;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::SCREENSHOT_STATUS_COMPLETED;
#[cfg(not(target_arch = "wasm32"))]
use crate::constants::UNKNOWN_WORKING_DIRECTORY;

pub(super) struct ScreenshotPlugin;

impl Plugin for ScreenshotPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(CapturePlugin);
        #[cfg(target_arch = "wasm32")]
        let _ = app;
    }
}

/// Handles the terminal `brp_extras/screenshot` watching request.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    ensure_png_support()?;

    let request = ScreenshotRequest::from_params(params)?;
    if let Some(response) = capture::read_existing(
        &mut world.resource_mut::<PendingScreenshotCaptures>(),
        &request,
    ) {
        return response;
    }

    let primary_window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .ok_or_else(no_primary_window_error)?;
    let target = Screenshot::primary_window()
        .0
        .normalize(Some(primary_window))
        .ok_or_else(no_primary_window_error)?;

    let (response, spawn_target) = capture::handle(
        &mut world.resource_mut::<PendingScreenshotCaptures>(),
        request,
        target,
    )?;

    if let Some(target) = spawn_target {
        capture::spawn_primary_window_batch(world, target);
    }

    Ok(response)
}

/// Returns an actionable error on targets without filesystem publication.
#[cfg(target_arch = "wasm32")]
pub(crate) fn handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult<Option<Value>> {
    drop(params);
    let _ = world;
    Err(BrpError {
        code:    INTERNAL_ERROR,
        message: "Screenshot PNG publication is unsupported on WASM; use a native target with filesystem access"
            .to_string(),
        data:    None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn completed_response(path: &Path) -> Value {
    json!({
        RESPONSE_SUCCESS_FIELD: true,
        PARAM_PATH: path.to_string_lossy(),
        RESPONSE_WORKING_DIRECTORY_FIELD: std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from(UNKNOWN_WORKING_DIRECTORY))
            .to_string_lossy(),
        RESPONSE_NOTE_FIELD: SCREENSHOT_CAPTURE_NOTE,
        RESPONSE_STATUS_FIELD: SCREENSHOT_STATUS_COMPLETED,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_png_support() -> BrpResult<()> {
    if bevy::image::ImageFormat::from_extension(IMAGE_EXTENSION_PNG).is_some() {
        return Ok(());
    }

    Err(BrpError {
        code:    INTERNAL_ERROR,
        message: "PNG support not available. Enable the 'png' feature in your Bevy dependency"
            .to_string(),
        data:    None,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn no_primary_window_error() -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: "Screenshot capture requires a primary window".to_string(),
        data:    None,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn completed_response_preserves_existing_fields_and_adds_terminal_status() {
        let response = completed_response(Path::new("/tmp/screenshot.png"));

        assert_eq!(
            response.get(RESPONSE_SUCCESS_FIELD),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            response.get(PARAM_PATH).and_then(Value::as_str),
            Some("/tmp/screenshot.png")
        );
        assert!(response.get(RESPONSE_WORKING_DIRECTORY_FIELD).is_some());
        assert_eq!(
            response.get(RESPONSE_NOTE_FIELD).and_then(Value::as_str),
            Some(SCREENSHOT_CAPTURE_NOTE)
        );
        assert_eq!(
            response.get(RESPONSE_STATUS_FIELD).and_then(Value::as_str),
            Some(SCREENSHOT_STATUS_COMPLETED)
        );
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::error::Error;
    use std::io;

    use bevy::prelude::*;
    use serde_json::json;

    use super::*;

    #[test]
    fn unsupported_publication_returns_before_resource_or_job_creation()
    -> Result<(), Box<dyn Error>> {
        let mut app = App::new();
        app.add_plugins(ScreenshotPlugin);
        let initial_entities = app.world().entities().len();
        let system_id = app.world_mut().register_system(handler);

        let result = app
            .world_mut()
            .run_system_with(system_id, Some(json!({ "path": 42 })))
            .map_err(|error| io::Error::other(error.to_string()))?;

        assert!(matches!(
            result,
            Err(error) if error.message.contains("unsupported on WASM")
        ));
        assert_eq!(app.world().entities().len(), initial_entities);
        Ok(())
    }
}
