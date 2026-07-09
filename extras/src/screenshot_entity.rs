//! Screenshot-entity handler for BRP extras
//!
//! PNG crop of a single UI node's on-screen rect. Reuses the same
//! window-screenshot + async-save path as the whole-window `screenshot`
//! handler.

use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy::tasks::IoTaskPool;
use bevy::ui::ComputedNode;
use bevy::ui::UiGlobalTransform;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::COMPONENT_NOT_PRESENT;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;

use crate::constants::IMAGE_EXTENSION_PNG;
use crate::constants::PARAM_PATH;
use crate::constants::RESPONSE_RECT_FIELD;
use crate::constants::RESPONSE_SUCCESS_FIELD;

#[derive(Deserialize)]
struct ScreenshotEntityParams {
    entity: Entity,
    path:   String,
}

/// Handler for entity-screenshot requests
///
/// Crops the primary window screenshot down to the given entity's laid-out
/// UI rect and saves it to the specified path. File I/O is performed
/// asynchronously to avoid blocking the main thread.
pub(crate) fn handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    if bevy::image::ImageFormat::from_extension(IMAGE_EXTENSION_PNG).is_none() {
        return Err(BrpError {
            code:    INTERNAL_ERROR,
            message: "PNG support not available. Enable the 'png' feature in your Bevy dependency"
                .to_string(),
            data:    None,
        });
    }

    let params: ScreenshotEntityParams = params
        .ok_or_else(|| BrpError {
            code:    INVALID_PARAMS,
            message: "Missing 'entity'/'path' parameters".to_string(),
            data:    None,
        })
        .and_then(|value| {
            serde_json::from_value(value).map_err(|err| BrpError {
                code:    INVALID_PARAMS,
                message: format!("Invalid params: {err}"),
                data:    None,
            })
        })?;

    let mut query = world.query::<(&ComputedNode, &UiGlobalTransform)>();
    let (computed, transform) = query.get(world, params.entity).map_err(|_| BrpError {
        code:    COMPONENT_NOT_PRESENT,
        message: "Entity has no ComputedNode/UiGlobalTransform (not a laid-out UI node)"
            .to_string(),
        data:    None,
    })?;

    let size = computed.size;
    let center = transform.translation;
    let x = (center.x - size.x / 2.0).max(0.0);
    let y = (center.y - size.y / 2.0).max(0.0);
    let w = size.x.max(1.0);
    let h = size.y.max(1.0);

    let path_buf = std::path::Path::new(&params.path);
    let absolute_path = if path_buf.is_absolute() {
        path_buf.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| BrpError {
                code:    INTERNAL_ERROR,
                message: format!("Failed to get current directory: {e}"),
                data:    None,
            })?
            .join(path_buf)
    };
    let absolute_path_str = absolute_path.to_string_lossy().to_string();
    let path_for_observer = absolute_path_str.clone();

    world
        .spawn((
            Screenshot::primary_window(),
            Name::new(format!("EntityScreenshot_{}", params.entity)),
        ))
        .observe(move |ev: On<ScreenshotCaptured>| {
            let img = ev.event().image.clone();
            let path_clone = path_for_observer.clone();
            IoTaskPool::get()
                .spawn(async move {
                    match img.try_into_dynamic() {
                        Ok(dynamic_image) => {
                            if let Some(parent) = std::path::Path::new(&path_clone).parent()
                                && let Err(e) = std::fs::create_dir_all(parent)
                            {
                                error!("Failed to create directory for {path_clone}: {e}");
                                return;
                            }
                            let cropped =
                                dynamic_image.crop_imm(x as u32, y as u32, w as u32, h as u32);
                            if let Err(e) = cropped.save(&path_clone) {
                                error!("Failed to save entity screenshot to {path_clone}: {e}");
                            }
                        },
                        Err(e) => error!("Failed to convert screenshot to dynamic image: {e}"),
                    }
                })
                .detach();
        });

    Ok(json!({
        RESPONSE_SUCCESS_FIELD: true,
        PARAM_PATH: absolute_path_str,
        RESPONSE_RECT_FIELD: { "x": x, "y": y, "w": w, "h": h },
    }))
}
