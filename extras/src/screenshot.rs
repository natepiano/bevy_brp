//! Screenshot handler for BRP extras

use bevy::prelude::*;
use bevy::remote;
use bevy::remote::error_codes::{INTERNAL_ERROR, INVALID_PARAMS};
use bevy::remote::{BrpError, BrpResult};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::tasks::IoTaskPool;
use serde_json::{Value, json};

/// Handler for screenshot requests
///
/// Takes a screenshot of the primary window and saves it to the specified path.
/// The path parameter must be provided in the request.
/// File I/O is performed asynchronously to avoid blocking the main thread.
pub fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Check if PNG support is available at runtime
    if bevy::image::ImageFormat::from_extension("png").is_none() {
        return Err(BrpError {
            code: remote::error_codes::INTERNAL_ERROR,
            message: "PNG support not available. Enable the 'png' feature in your Bevy dependency"
                .to_string(),
            data: None,
        });
    }
    // Get the path from params
    let path = params
        .as_ref()
        .and_then(|v| v.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| BrpError {
            code: INVALID_PARAMS,
            message: "Missing 'path' parameter".to_string(),
            data: None,
        })?;

    // Convert to absolute path
    let path_buf = std::path::Path::new(path);
    let absolute_path = if path_buf.is_absolute() {
        path_buf.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| BrpError {
                code: INTERNAL_ERROR,
                message: format!("Failed to get current directory: {e}"),
                data: None,
            })?
            .join(path_buf)
    };

    let absolute_path_str = absolute_path.to_string_lossy().to_string();

    // Log the screenshot request
    info!("Screenshot requested for: {}", absolute_path_str);

    // Check if we have a primary window
    let window_exists = world.query::<&Window>().iter(world).any(|w| {
        info!(
            "Found window - resolution: {:?}, visible: {:?}",
            w.resolution, w.visible
        );
        true
    });

    if !window_exists {
        warn!("No windows found in the world!");
    }

    // Spawn a screenshot entity with an observer to handle the capture
    let path_for_observer = absolute_path_str.clone();
    let entity = world
        .spawn((
            Screenshot::primary_window(),
            Name::new(format!("Screenshot_{absolute_path_str}")),
        ))
        .observe(move |screenshot_captured: On<ScreenshotCaptured>| {
            info!("Screenshot captured! Starting async save to: {path_for_observer}");
            let img = screenshot_captured.event().image.clone();
            let path_clone = path_for_observer.clone();

            // Move file I/O to background thread to avoid blocking main thread
            IoTaskPool::get()
                .spawn(async move {
                    match img.try_into_dynamic() {
                        Ok(dyn_img) => {
                            // Create parent directory if needed
                            if let Some(parent) = std::path::Path::new(&path_clone).parent()
                                && let Err(e) = std::fs::create_dir_all(parent)
                            {
                                error!(
                                    "Failed to create directory for screenshot {path_clone}: {e}"
                                );
                                return;
                            }

                            // Save the image
                            match dyn_img.save(&path_clone) {
                                Ok(()) => {
                                    info!("Screenshot successfully saved to: {path_clone}");
                                }
                                Err(e) => {
                                    error!("Failed to save screenshot to {path_clone}: {e}");
                                }
                            }
                        }
                        Err(e) => error!("Failed to convert screenshot to dynamic image: {e}"),
                    }
                })
                .detach();
        })
        .id();

    info!("Screenshot entity spawned with ID: {:?}", entity);

    Ok(json!({
        "success": true,
        "path": absolute_path_str,
        "working_directory": std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("unknown")).to_string_lossy(),
        "note": "Screenshot capture initiated. File I/O will be performed asynchronously on background thread."
    }))
}
