//! Send-keys handler: press/hold/release key sequences via BRP.

use std::str::FromStr;
use std::time::Duration;

use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::WindowEvent;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::DEFAULT_KEY_DURATION_MS;
use super::MAX_KEY_DURATION_MS;
use super::create_keyboard_events;
use super::key_code::KeyCodeWrapper;
use crate::window_event;

/// Component that tracks keys that need to be released after a duration
#[derive(Component)]
pub(crate) struct TimedKeyRelease {
    /// The key code wrappers to release (stores wrapper for text field generation)
    pub keys:  Vec<KeyCodeWrapper>,
    /// Timer tracking the remaining duration
    pub timer: Timer,
}

/// Request structure for `send_keys`
#[derive(Debug, Deserialize)]
pub(crate) struct SendKeysRequest {
    /// Array of key codes to send
    pub keys:        Vec<String>,
    /// Duration in milliseconds to hold the keys before releasing
    #[serde(default = "default_duration")]
    pub duration_ms: u32,
}

const fn default_duration() -> u32 { DEFAULT_KEY_DURATION_MS }

/// Response structure for `send_keys`
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SendKeysResponse {
    /// Whether the operation was successful
    pub success:     bool,
    /// List of keys that were sent
    pub keys_sent:   Vec<String>,
    /// Duration in milliseconds the keys were held
    pub duration_ms: u32,
}

/// Validate key codes and return the parsed key code wrappers
fn validate_keys(keys: &[String]) -> Result<Vec<(String, KeyCodeWrapper)>, BrpError> {
    let mut validated_keys = Vec::new();

    for key_str in keys {
        match KeyCodeWrapper::from_str(key_str) {
            Ok(wrapper) => {
                validated_keys.push((key_str.clone(), wrapper));
            },
            Err(_) => {
                return Err(BrpError {
                    code:    INVALID_PARAMS,
                    message: format!("Invalid key code '{key_str}': Unknown key code"),
                    data:    None,
                });
            },
        }
    }

    Ok(validated_keys)
}

/// Handler for `send_keys` requests
///
/// Simulates keyboard input by sending key press/release events
///
/// # Errors
///
/// Returns `BrpError` if:
/// - Request parameters are missing
/// - Request format is invalid
/// - Any key code is invalid or unknown
pub(crate) fn send_keys_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Parse the request
    let request: SendKeysRequest = if let Some(params) = params {
        serde_json::from_value(params).map_err(|e| BrpError {
            code:    INVALID_PARAMS,
            message: format!("Invalid request format: {e}"),
            data:    None,
        })?
    } else {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "Missing request parameters".to_string(),
            data:    None,
        });
    };

    // Validate key codes
    let validated_keys = validate_keys(&request.keys)?;
    let valid_key_strings: Vec<String> = validated_keys.iter().map(|(s, _)| s.clone()).collect();
    let wrappers: Vec<KeyCodeWrapper> = validated_keys.iter().map(|(_, w)| *w).collect();

    // Validate duration doesn't exceed maximum
    if request.duration_ms > MAX_KEY_DURATION_MS {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: format!(
                "Duration {}ms exceeds maximum allowed duration of {}ms (1 minute)",
                request.duration_ms, MAX_KEY_DURATION_MS
            ),
            data:    None,
        });
    }

    // Always send press events first
    let press_events = create_keyboard_events(&wrappers, ButtonState::Pressed);
    for event in press_events {
        window_event::write_input_event(world, event);
    }

    // Always spawn an entity to handle the timed release
    if !wrappers.is_empty() {
        world.spawn(TimedKeyRelease {
            keys:  wrappers,
            timer: Timer::new(
                Duration::from_millis(u64::from(request.duration_ms)),
                TimerMode::Once,
            ),
        });
    }

    Ok(json!(SendKeysResponse {
        success:     true,
        keys_sent:   valid_key_strings,
        duration_ms: request.duration_ms,
    }))
}

/// System that processes timed key releases
pub(crate) fn process_timed_key_releases(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedKeyRelease)>,
    mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    for (entity, mut timed_release) in &mut query {
        timed_release.timer.tick(time.delta());

        if timed_release.timer.is_finished() {
            // Send release events for all keys (text is None for release events)
            let release_events = create_keyboard_events(&timed_release.keys, ButtonState::Released);
            for event in release_events {
                window_events.write(WindowEvent::from(event.clone()));
                keyboard_events.write(event);
            }

            // Remove the component after releasing
            commands.entity(entity).despawn();
        }
    }
}
