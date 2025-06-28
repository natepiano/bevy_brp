//! Keyboard input simulation for BRP extras

use std::str::FromStr;
use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::remote::{BrpError, BrpResult, error_codes};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use strum_macros::{Display, EnumIter, EnumString};

/// Maximum duration for holding keys in milliseconds (1 minute)
const MAX_KEY_DURATION_MS: u32 = 60_000;

/// Default duration for holding keys in milliseconds
const DEFAULT_KEY_DURATION_MS: u32 = 100;

/// Component that tracks keys that need to be released after a duration
#[derive(Component)]
pub struct TimedKeyRelease {
    /// The key codes to release
    pub keys:  Vec<KeyCode>,
    /// Timer tracking the remaining duration
    pub timer: Timer,
}

/// Wrapper enum for Bevy's `KeyCode` with strum derives for string conversion
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString, EnumIter, Display)]
#[strum(serialize_all = "PascalCase")]
#[allow(missing_docs)]
pub enum KeyCodeWrapper {
    // Letters
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // Digits
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    // Modifiers
    AltLeft,
    AltRight,
    ControlLeft,
    ControlRight,
    ShiftLeft,
    ShiftRight,
    SuperLeft,
    SuperRight,

    // Navigation
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    End,
    Home,
    PageDown,
    PageUp,

    // Editing
    Backspace,
    Delete,
    Enter,
    Escape,
    Insert,
    Space,
    Tab,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    NumpadDecimal,
    NumpadEnter,

    // Media and special
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    BrowserBack,
    BrowserForward,
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    CapsLock,
    NumLock,
    ScrollLock,
    PrintScreen,
    Pause,
    MediaPlayPause,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,

    // Punctuation and symbols
    Backquote,
    Backslash,
    BracketLeft,
    BracketRight,
    Comma,
    Equal,
    Minus,
    Period,
    Quote,
    Semicolon,
    Slash,
}

impl KeyCodeWrapper {
    /// Convert the wrapper to a Bevy `KeyCode`
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn to_key_code(self) -> KeyCode {
        match self {
            // Letters
            Self::KeyA => KeyCode::KeyA,
            Self::KeyB => KeyCode::KeyB,
            Self::KeyC => KeyCode::KeyC,
            Self::KeyD => KeyCode::KeyD,
            Self::KeyE => KeyCode::KeyE,
            Self::KeyF => KeyCode::KeyF,
            Self::KeyG => KeyCode::KeyG,
            Self::KeyH => KeyCode::KeyH,
            Self::KeyI => KeyCode::KeyI,
            Self::KeyJ => KeyCode::KeyJ,
            Self::KeyK => KeyCode::KeyK,
            Self::KeyL => KeyCode::KeyL,
            Self::KeyM => KeyCode::KeyM,
            Self::KeyN => KeyCode::KeyN,
            Self::KeyO => KeyCode::KeyO,
            Self::KeyP => KeyCode::KeyP,
            Self::KeyQ => KeyCode::KeyQ,
            Self::KeyR => KeyCode::KeyR,
            Self::KeyS => KeyCode::KeyS,
            Self::KeyT => KeyCode::KeyT,
            Self::KeyU => KeyCode::KeyU,
            Self::KeyV => KeyCode::KeyV,
            Self::KeyW => KeyCode::KeyW,
            Self::KeyX => KeyCode::KeyX,
            Self::KeyY => KeyCode::KeyY,
            Self::KeyZ => KeyCode::KeyZ,
            // Digits
            Self::Digit0 => KeyCode::Digit0,
            Self::Digit1 => KeyCode::Digit1,
            Self::Digit2 => KeyCode::Digit2,
            Self::Digit3 => KeyCode::Digit3,
            Self::Digit4 => KeyCode::Digit4,
            Self::Digit5 => KeyCode::Digit5,
            Self::Digit6 => KeyCode::Digit6,
            Self::Digit7 => KeyCode::Digit7,
            Self::Digit8 => KeyCode::Digit8,
            Self::Digit9 => KeyCode::Digit9,
            // Function keys
            Self::F1 => KeyCode::F1,
            Self::F2 => KeyCode::F2,
            Self::F3 => KeyCode::F3,
            Self::F4 => KeyCode::F4,
            Self::F5 => KeyCode::F5,
            Self::F6 => KeyCode::F6,
            Self::F7 => KeyCode::F7,
            Self::F8 => KeyCode::F8,
            Self::F9 => KeyCode::F9,
            Self::F10 => KeyCode::F10,
            Self::F11 => KeyCode::F11,
            Self::F12 => KeyCode::F12,
            Self::F13 => KeyCode::F13,
            Self::F14 => KeyCode::F14,
            Self::F15 => KeyCode::F15,
            Self::F16 => KeyCode::F16,
            Self::F17 => KeyCode::F17,
            Self::F18 => KeyCode::F18,
            Self::F19 => KeyCode::F19,
            Self::F20 => KeyCode::F20,
            Self::F21 => KeyCode::F21,
            Self::F22 => KeyCode::F22,
            Self::F23 => KeyCode::F23,
            Self::F24 => KeyCode::F24,
            // Modifiers
            Self::AltLeft => KeyCode::AltLeft,
            Self::AltRight => KeyCode::AltRight,
            Self::ControlLeft => KeyCode::ControlLeft,
            Self::ControlRight => KeyCode::ControlRight,
            Self::ShiftLeft => KeyCode::ShiftLeft,
            Self::ShiftRight => KeyCode::ShiftRight,
            Self::SuperLeft => KeyCode::SuperLeft,
            Self::SuperRight => KeyCode::SuperRight,
            // Navigation
            Self::ArrowDown => KeyCode::ArrowDown,
            Self::ArrowLeft => KeyCode::ArrowLeft,
            Self::ArrowRight => KeyCode::ArrowRight,
            Self::ArrowUp => KeyCode::ArrowUp,
            Self::End => KeyCode::End,
            Self::Home => KeyCode::Home,
            Self::PageDown => KeyCode::PageDown,
            Self::PageUp => KeyCode::PageUp,
            // Editing
            Self::Backspace => KeyCode::Backspace,
            Self::Delete => KeyCode::Delete,
            Self::Enter => KeyCode::Enter,
            Self::Escape => KeyCode::Escape,
            Self::Insert => KeyCode::Insert,
            Self::Space => KeyCode::Space,
            Self::Tab => KeyCode::Tab,
            // Numpad
            Self::Numpad0 => KeyCode::Numpad0,
            Self::Numpad1 => KeyCode::Numpad1,
            Self::Numpad2 => KeyCode::Numpad2,
            Self::Numpad3 => KeyCode::Numpad3,
            Self::Numpad4 => KeyCode::Numpad4,
            Self::Numpad5 => KeyCode::Numpad5,
            Self::Numpad6 => KeyCode::Numpad6,
            Self::Numpad7 => KeyCode::Numpad7,
            Self::Numpad8 => KeyCode::Numpad8,
            Self::Numpad9 => KeyCode::Numpad9,
            Self::NumpadAdd => KeyCode::NumpadAdd,
            Self::NumpadDivide => KeyCode::NumpadDivide,
            Self::NumpadMultiply => KeyCode::NumpadMultiply,
            Self::NumpadSubtract => KeyCode::NumpadSubtract,
            Self::NumpadDecimal => KeyCode::NumpadDecimal,
            Self::NumpadEnter => KeyCode::NumpadEnter,
            // Media and special
            Self::AudioVolumeDown => KeyCode::AudioVolumeDown,
            Self::AudioVolumeMute => KeyCode::AudioVolumeMute,
            Self::AudioVolumeUp => KeyCode::AudioVolumeUp,
            Self::BrowserBack => KeyCode::BrowserBack,
            Self::BrowserForward => KeyCode::BrowserForward,
            Self::BrowserHome => KeyCode::BrowserHome,
            Self::BrowserRefresh => KeyCode::BrowserRefresh,
            Self::BrowserSearch => KeyCode::BrowserSearch,
            Self::CapsLock => KeyCode::CapsLock,
            Self::NumLock => KeyCode::NumLock,
            Self::ScrollLock => KeyCode::ScrollLock,
            Self::PrintScreen => KeyCode::PrintScreen,
            Self::Pause => KeyCode::Pause,
            Self::MediaPlayPause => KeyCode::MediaPlayPause,
            Self::MediaStop => KeyCode::MediaStop,
            Self::MediaTrackNext => KeyCode::MediaTrackNext,
            Self::MediaTrackPrevious => KeyCode::MediaTrackPrevious,
            // Punctuation and symbols
            Self::Backquote => KeyCode::Backquote,
            Self::Backslash => KeyCode::Backslash,
            Self::BracketLeft => KeyCode::BracketLeft,
            Self::BracketRight => KeyCode::BracketRight,
            Self::Comma => KeyCode::Comma,
            Self::Equal => KeyCode::Equal,
            Self::Minus => KeyCode::Minus,
            Self::Period => KeyCode::Period,
            Self::Quote => KeyCode::Quote,
            Self::Semicolon => KeyCode::Semicolon,
            Self::Slash => KeyCode::Slash,
        }
    }

    /// Get the category for this key code
    #[allow(clippy::enum_glob_use)]
    #[must_use]
    pub const fn category(&self) -> &'static str {
        use KeyCodeWrapper::*;
        match self {
            // Letters
            KeyA | KeyB | KeyC | KeyD | KeyE | KeyF | KeyG | KeyH | KeyI | KeyJ | KeyK | KeyL
            | KeyM | KeyN | KeyO | KeyP | KeyQ | KeyR | KeyS | KeyT | KeyU | KeyV | KeyW | KeyX
            | KeyY | KeyZ => "Letters",

            // Digits
            Digit0 | Digit1 | Digit2 | Digit3 | Digit4 | Digit5 | Digit6 | Digit7 | Digit8
            | Digit9 => "Digits",

            // Function keys
            F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | F9 | F10 | F11 | F12 | F13 | F14 | F15
            | F16 | F17 | F18 | F19 | F20 | F21 | F22 | F23 | F24 => "Function",

            // Modifiers
            AltLeft | AltRight | ControlLeft | ControlRight | ShiftLeft | ShiftRight
            | SuperLeft | SuperRight => "Modifiers",

            // Navigation
            ArrowDown | ArrowLeft | ArrowRight | ArrowUp | End | Home | PageDown | PageUp => {
                "Navigation"
            }

            // Editing
            Backspace | Delete | Enter | Escape | Insert | Space | Tab => "Editing",

            // Numpad
            Numpad0 | Numpad1 | Numpad2 | Numpad3 | Numpad4 | Numpad5 | Numpad6 | Numpad7
            | Numpad8 | Numpad9 | NumpadAdd | NumpadDivide | NumpadMultiply | NumpadSubtract
            | NumpadDecimal | NumpadEnter => "Numpad",

            // Media and special
            AudioVolumeDown | AudioVolumeMute | AudioVolumeUp | BrowserBack | BrowserForward
            | BrowserHome | BrowserRefresh | BrowserSearch | CapsLock | NumLock | ScrollLock
            | PrintScreen | Pause | MediaPlayPause | MediaStop | MediaTrackNext
            | MediaTrackPrevious => "Special",

            // Punctuation and symbols
            Backquote | Backslash | BracketLeft | BracketRight | Comma | Equal | Minus | Period
            | Quote | Semicolon | Slash => "Punctuation",
        }
    }
}

/// Request structure for `send_keys`
#[derive(Debug, Deserialize)]
pub struct SendKeysRequest {
    /// Array of key codes to send
    pub keys:        Vec<String>,
    /// Duration in milliseconds to hold the keys before releasing
    #[serde(default = "default_duration")]
    pub duration_ms: u32,
}

const fn default_duration() -> u32 {
    DEFAULT_KEY_DURATION_MS
}

/// Response structure for `send_keys`
#[derive(Debug, Serialize, Deserialize)]
pub struct SendKeysResponse {
    /// Whether the operation was successful
    pub success:     bool,
    /// List of keys that were sent
    pub keys_sent:   Vec<String>,
    /// Duration in milliseconds the keys were held
    pub duration_ms: u32,
}

/// Validate key codes and return the parsed key codes
fn validate_keys(keys: &[String]) -> Result<Vec<(String, KeyCode)>, BrpError> {
    let mut validated_keys = Vec::new();

    for key_str in keys {
        match parse_key_code(key_str) {
            Ok(key_code) => {
                validated_keys.push((key_str.clone(), key_code));
            }
            Err(e) => {
                return Err(BrpError {
                    code:    error_codes::INVALID_PARAMS,
                    message: format!("Invalid key code '{key_str}': {e}"),
                    data:    None,
                });
            }
        }
    }

    Ok(validated_keys)
}

/// Create keyboard events from validated key codes
fn create_keyboard_events(
    key_codes: &[KeyCode],
    press: bool,
) -> Vec<bevy::input::keyboard::KeyboardInput> {
    let state = if press {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    };

    key_codes
        .iter()
        .map(|&key_code| bevy::input::keyboard::KeyboardInput {
            state,
            key_code,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            window: Entity::PLACEHOLDER,
            repeat: false,
            text: None,
        })
        .collect()
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
pub fn send_keys_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    // Parse the request
    let request: SendKeysRequest = if let Some(params) = params {
        serde_json::from_value(params).map_err(|e| BrpError {
            code:    error_codes::INVALID_PARAMS,
            message: format!("Invalid request format: {e}"),
            data:    None,
        })?
    } else {
        return Err(BrpError {
            code:    error_codes::INVALID_PARAMS,
            message: "Missing request parameters".to_string(),
            data:    None,
        });
    };

    // Validate key codes
    let validated_keys = validate_keys(&request.keys)?;
    let valid_key_strings: Vec<String> = validated_keys.iter().map(|(s, _)| s.clone()).collect();
    let key_codes: Vec<KeyCode> = validated_keys.iter().map(|(_, kc)| *kc).collect();

    // Validate duration doesn't exceed maximum
    if request.duration_ms > MAX_KEY_DURATION_MS {
        return Err(BrpError {
            code:    error_codes::INVALID_PARAMS,
            message: format!(
                "Duration {}ms exceeds maximum allowed duration of {}ms (1 minute)",
                request.duration_ms, MAX_KEY_DURATION_MS
            ),
            data:    None,
        });
    }

    // Always send press events first
    let press_events = create_keyboard_events(&key_codes, true);
    for event in press_events {
        world.send_event(event);
    }

    // Always spawn an entity to handle the timed release
    if !key_codes.is_empty() {
        world.spawn(TimedKeyRelease {
            keys:  key_codes,
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

/// Information about a key code
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyCodeInfo {
    /// The name of the key code (e.g., "`KeyA`", "`Space`")
    pub name:     String,
    /// The category of the key (e.g., "Letters", "Modifiers")
    pub category: String,
}

/// Parse a string into a `KeyCode`
fn parse_key_code(s: &str) -> Result<KeyCode, String> {
    KeyCodeWrapper::from_str(s)
        .map(KeyCodeWrapper::to_key_code)
        .map_err(|_| format!("Unknown key code: {s}"))
}

/// System that processes timed key releases
pub fn process_timed_key_releases(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedKeyRelease)>,
    mut keyboard_events: EventWriter<bevy::input::keyboard::KeyboardInput>,
) {
    for (entity, mut timed_release) in &mut query {
        timed_release.timer.tick(time.delta());

        if timed_release.timer.finished() {
            // Send release events for all keys
            for &key_code in &timed_release.keys {
                let event = bevy::input::keyboard::KeyboardInput {
                    state: ButtonState::Released,
                    key_code,
                    logical_key: bevy::input::keyboard::Key::Unidentified(
                        bevy::input::keyboard::NativeKey::Unidentified,
                    ),
                    window: Entity::PLACEHOLDER,
                    repeat: false,
                    text: None,
                };
                keyboard_events.write(event);
            }

            // Remove the component after releasing
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::app::App;
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    #[allow(clippy::expect_used)]
    fn test_duration_validation_exceeds_maximum() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with duration exceeding the maximum
        let params = json!({
            "keys": ["KeyA"],
            "duration_ms": 70_000  // 70 seconds, exceeds 60 second maximum
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it returns an error
        assert!(result.is_err());

        let error = result.expect_err("Expected an error but got success");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert!(error.message.contains("exceeds maximum allowed duration"));
        assert!(error.message.contains("60000ms"));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_duration_validation_within_maximum() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with duration within the maximum
        let params = json!({
            "keys": ["KeyA"],
            "duration_ms": 30_000  // 30 seconds, within 60 second maximum
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], 30_000);
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_default_duration() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request without specifying duration_ms
        let params = json!({
            "keys": ["KeyA", "KeyB", "Space"]
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], 100); // Should use default of 100ms
        assert_eq!(response["keys_sent"], json!(["KeyA", "KeyB", "Space"]));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn test_zero_duration() {
        // Create a minimal Bevy app
        let mut app = App::new();

        // Create a request with zero duration (should be valid)
        let params = json!({
            "keys": ["Enter"],
            "duration_ms": 0
        });

        // Call the handler
        let result = send_keys_handler(In(Some(params)), app.world_mut());

        // Verify it succeeds
        assert!(result.is_ok());

        let response = result.expect("Expected success but got error");
        assert_eq!(response["success"], true);
        assert_eq!(response["duration_ms"], 0);
    }

    /// Test that all key code variants can be parsed
    #[test]
    #[allow(clippy::expect_used)]
    fn test_parse_all_key_codes() {
        // Test that all keys can be successfully used in a send_keys request
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Iterate over all key code variants
        for key_wrapper in KeyCodeWrapper::iter() {
            let key = key_wrapper.to_string();
            let params = json!({
                "keys": [&key]
            });

            let result = send_keys_handler(In(Some(params)), app.world_mut());

            assert!(result.is_ok(), "Failed to parse key code: {key}");

            if let Ok(response_value) = result {
                let response: SendKeysResponse =
                    serde_json::from_value(response_value).expect("Failed to deserialize response");
                assert!(response.success);
                assert_eq!(response.keys_sent.len(), 1);
                assert_eq!(response.keys_sent[0], key);
                assert_eq!(response.duration_ms, 100); // default duration
            }
        }
    }

    /// Test invalid key codes return appropriate errors
    #[test]
    fn test_invalid_key_codes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let invalid_keys = vec![
            "InvalidKey",
            "Key1",  // Should be Digit1
            "Ctrl",  // Should be ControlLeft or ControlRight
            "Shift", // Should be ShiftLeft or ShiftRight
            "F25",   // Function keys only go up to F24
            "",
            "key a", // lowercase and space
            "KEY_A", // Wrong format
        ];

        for invalid_key in invalid_keys {
            let params = json!({
                "keys": [invalid_key]
            });

            let result = send_keys_handler(In(Some(params)), app.world_mut());

            assert!(
                result.is_err(),
                "Expected error for invalid key: {invalid_key}"
            );
        }
    }

    /// Test press-hold-release cycle with different durations
    #[test]
    #[allow(clippy::expect_used)]
    fn test_press_hold_release_cycle() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with default duration
        let default_params = json!({
            "keys": ["Space", "Enter"]
        });

        let default_result = send_keys_handler(In(Some(default_params)), app.world_mut());

        assert!(default_result.is_ok());
        if let Ok(response_value) = default_result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.duration_ms, 100); // default duration
            assert_eq!(response.keys_sent.len(), 2);
        }

        // Test with custom duration
        let custom_params = json!({
            "keys": ["Space", "Enter"],
            "duration_ms": 500
        });

        let custom_result = send_keys_handler(In(Some(custom_params)), app.world_mut());

        assert!(custom_result.is_ok());
        if let Ok(response_value) = custom_result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.duration_ms, 500);
            assert_eq!(response.keys_sent.len(), 2);
        }
    }

    /// Test missing parameters returns appropriate error
    #[test]
    fn test_missing_parameters() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let result = send_keys_handler(In(None), app.world_mut());

        assert!(result.is_err());
        if let Err(error) = result {
            assert_eq!(error.message, "Missing request parameters");
        }
    }

    /// Test empty key array
    #[test]
    #[allow(clippy::expect_used)]
    fn test_empty_keys() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let params = json!({
            "keys": []
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());
        if let Ok(response_value) = result {
            let response: SendKeysResponse =
                serde_json::from_value(response_value).expect("Failed to deserialize response");
            assert_eq!(response.keys_sent.len(), 0);
        }
    }

    /// Test that `TimedKeyRelease` component is always created
    #[test]
    fn test_timed_release_always_created() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with custom duration
        let params = json!({
            "keys": ["Space", "Enter"],
            "duration_ms": 500
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that a TimedKeyRelease component was created
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1, "Expected one TimedKeyRelease component");

        // Verify the component has the correct keys
        if let Some(timed_release) = query.iter(app.world()).next() {
            assert_eq!(timed_release.keys.len(), 2);
        }
    }

    /// Test default duration creates `TimedKeyRelease` component
    #[test]
    fn test_default_duration_creates_timed_release() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with default duration (no duration_ms specified)
        let params = json!({
            "keys": ["Space"]
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that a TimedKeyRelease component was created with default duration
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 1,
            "Expected one TimedKeyRelease component with default duration"
        );
    }

    /// Test that empty key array does not create `TimedKeyRelease`
    #[test]
    fn test_empty_keys_no_timed_release() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test with empty keys array
        let params = json!({
            "keys": [],
            "duration_ms": 500
        });

        let result = send_keys_handler(In(Some(params)), app.world_mut());

        assert!(result.is_ok());

        // Check that no TimedKeyRelease component was created
        let mut query = app.world_mut().query::<&TimedKeyRelease>();
        let count = query.iter(app.world()).count();
        assert_eq!(
            count, 0,
            "Expected no TimedKeyRelease components when keys array is empty"
        );
    }
}
