//! Keyboard input simulation for BRP extras

use std::str::FromStr;
use std::time::Duration;

use bevy::input::ButtonState;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;
use bevy::remote::BrpError;
use bevy::remote::BrpResult;
use bevy::remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;
use strum_macros::Display;
use strum_macros::EnumIter;
use strum_macros::EnumString;

/// Maximum duration for holding keys in milliseconds (1 minute)
const MAX_KEY_DURATION_MS: u32 = 60_000;

/// Default duration for holding keys in milliseconds
const DEFAULT_KEY_DURATION_MS: u32 = 100;

/// Component that tracks keys that need to be released after a duration
#[derive(Component)]
pub struct TimedKeyRelease {
    /// The key code wrappers to release (stores wrapper for text field generation)
    pub keys:  Vec<KeyCodeWrapper>,
    /// Timer tracking the remaining duration
    pub timer: Timer,
}

/// Component for sequential text typing (one character per frame).
/// Used by `type_text` RPC to simulate realistic typing.
#[derive(Component)]
pub struct TextTypingQueue {
    /// Characters remaining to type
    pub chars: std::collections::VecDeque<char>,
    /// Currently pressed keys (waiting for release next frame)
    pub current_keys: Vec<KeyCodeWrapper>,
    /// Phase: true = need to release current keys, false = ready to press next
    pub releasing: bool,
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
    /// Convert the wrapper to a character for text input (lowercase, unshifted).
    ///
    /// Returns `None` for non-printable keys (modifiers, function keys, etc.)
    #[must_use]
    pub const fn to_char(self) -> Option<char> {
        match self {
            // Letters (lowercase)
            Self::KeyA => Some('a'),
            Self::KeyB => Some('b'),
            Self::KeyC => Some('c'),
            Self::KeyD => Some('d'),
            Self::KeyE => Some('e'),
            Self::KeyF => Some('f'),
            Self::KeyG => Some('g'),
            Self::KeyH => Some('h'),
            Self::KeyI => Some('i'),
            Self::KeyJ => Some('j'),
            Self::KeyK => Some('k'),
            Self::KeyL => Some('l'),
            Self::KeyM => Some('m'),
            Self::KeyN => Some('n'),
            Self::KeyO => Some('o'),
            Self::KeyP => Some('p'),
            Self::KeyQ => Some('q'),
            Self::KeyR => Some('r'),
            Self::KeyS => Some('s'),
            Self::KeyT => Some('t'),
            Self::KeyU => Some('u'),
            Self::KeyV => Some('v'),
            Self::KeyW => Some('w'),
            Self::KeyX => Some('x'),
            Self::KeyY => Some('y'),
            Self::KeyZ => Some('z'),
            // Digits
            Self::Digit0 => Some('0'),
            Self::Digit1 => Some('1'),
            Self::Digit2 => Some('2'),
            Self::Digit3 => Some('3'),
            Self::Digit4 => Some('4'),
            Self::Digit5 => Some('5'),
            Self::Digit6 => Some('6'),
            Self::Digit7 => Some('7'),
            Self::Digit8 => Some('8'),
            Self::Digit9 => Some('9'),
            // Special printable keys
            Self::Space => Some(' '),
            Self::Tab => Some('\t'),
            Self::Enter => Some('\n'),
            // Punctuation (US layout, unshifted)
            Self::Backquote => Some('`'),
            Self::Backslash => Some('\\'),
            Self::BracketLeft => Some('['),
            Self::BracketRight => Some(']'),
            Self::Comma => Some(','),
            Self::Equal => Some('='),
            Self::Minus => Some('-'),
            Self::Period => Some('.'),
            Self::Quote => Some('\''),
            Self::Semicolon => Some(';'),
            Self::Slash => Some('/'),
            // Numpad digits
            Self::Numpad0 => Some('0'),
            Self::Numpad1 => Some('1'),
            Self::Numpad2 => Some('2'),
            Self::Numpad3 => Some('3'),
            Self::Numpad4 => Some('4'),
            Self::Numpad5 => Some('5'),
            Self::Numpad6 => Some('6'),
            Self::Numpad7 => Some('7'),
            Self::Numpad8 => Some('8'),
            Self::Numpad9 => Some('9'),
            Self::NumpadAdd => Some('+'),
            Self::NumpadSubtract => Some('-'),
            Self::NumpadMultiply => Some('*'),
            Self::NumpadDivide => Some('/'),
            Self::NumpadDecimal => Some('.'),
            Self::NumpadEnter => Some('\n'),
            // Non-printable keys
            _ => None,
        }
    }

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
            },

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

const fn default_duration() -> u32 { DEFAULT_KEY_DURATION_MS }

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

/// Create keyboard events from validated key code wrappers.
///
/// Populates `logical_key` and `text` fields for printable characters,
/// enabling text input simulation that works with Bevy's text input systems.
fn create_keyboard_events(
    wrappers: &[KeyCodeWrapper],
    press: bool,
) -> Vec<bevy::input::keyboard::KeyboardInput> {
    use bevy::input::keyboard::{Key, NativeKey};
    use smol_str::SmolStr;

    let state = if press {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    };

    wrappers
        .iter()
        .map(|&wrapper| {
            let key_code = wrapper.to_key_code();
            let char_opt = wrapper.to_char();

            // Build logical_key and text based on whether this is a printable character
            let (logical_key, text) = match char_opt {
                Some(c) => {
                    let s = SmolStr::new_inline(&c.to_string());
                    // Only populate text on press events, not release
                    let text = if press { Some(s.clone()) } else { None };
                    (Key::Character(s), text)
                },
                None => (Key::Unidentified(NativeKey::Unidentified), None),
            };

            bevy::input::keyboard::KeyboardInput {
                state,
                key_code,
                logical_key,
                window: Entity::PLACEHOLDER,
                repeat: false,
                text,
            }
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
    let press_events = create_keyboard_events(&wrappers, true);
    for event in press_events {
        world.write_message(event);
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

/// Information about a key code
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyCodeInfo {
    /// The name of the key code (e.g., "`KeyA`", "`Space`")
    pub name:     String,
    /// The category of the key (e.g., "Letters", "Modifiers")
    pub category: String,
}

/// System that processes timed key releases
pub fn process_timed_key_releases(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedKeyRelease)>,
    mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
) {
    for (entity, mut timed_release) in &mut query {
        timed_release.timer.tick(time.delta());

        if timed_release.timer.is_finished() {
            // Send release events for all keys (text is None for release events)
            let release_events = create_keyboard_events(&timed_release.keys, false);
            for event in release_events {
                keyboard_events.write(event);
            }

            // Remove the component after releasing
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// TYPE TEXT - Sequential character typing
// ============================================================================

/// Request structure for `type_text`
#[derive(Debug, Deserialize)]
pub struct TypeTextRequest {
    /// Text to type (supports letters, numbers, symbols, newlines, tabs)
    pub text: String,
}

/// Response structure for `type_text`
#[derive(Debug, Serialize, Deserialize)]
pub struct TypeTextResponse {
    /// Whether the operation was initiated successfully
    pub success: bool,
    /// Number of characters queued for typing
    pub chars_queued: usize,
    /// Characters that couldn't be mapped to keys (skipped)
    pub skipped: Vec<char>,
}

/// Convert a character to the key(s) needed to type it.
/// Returns None for unmappable characters.
fn char_to_keys(c: char) -> Option<Vec<KeyCodeWrapper>> {
    match c {
        // Lowercase letters
        'a'..='z' => {
            let key_name = format!("Key{}", c.to_ascii_uppercase());
            KeyCodeWrapper::from_str(&key_name).ok().map(|k| vec![k])
        }
        // Uppercase letters (need Shift)
        'A'..='Z' => {
            let key_name = format!("Key{}", c);
            KeyCodeWrapper::from_str(&key_name).ok().map(|k| vec![KeyCodeWrapper::ShiftLeft, k])
        }
        // Numbers
        '0'..='9' => {
            let key_name = format!("Digit{}", c);
            KeyCodeWrapper::from_str(&key_name).ok().map(|k| vec![k])
        }
        // Symbols - unshifted
        ' ' => Some(vec![KeyCodeWrapper::Space]),
        '-' => Some(vec![KeyCodeWrapper::Minus]),
        '=' => Some(vec![KeyCodeWrapper::Equal]),
        '[' => Some(vec![KeyCodeWrapper::BracketLeft]),
        ']' => Some(vec![KeyCodeWrapper::BracketRight]),
        '\\' => Some(vec![KeyCodeWrapper::Backslash]),
        ';' => Some(vec![KeyCodeWrapper::Semicolon]),
        '\'' => Some(vec![KeyCodeWrapper::Quote]),
        '`' => Some(vec![KeyCodeWrapper::Backquote]),
        ',' => Some(vec![KeyCodeWrapper::Comma]),
        '.' => Some(vec![KeyCodeWrapper::Period]),
        '/' => Some(vec![KeyCodeWrapper::Slash]),
        // Symbols - shifted
        '!' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit1]),
        '@' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit2]),
        '#' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit3]),
        '$' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit4]),
        '%' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit5]),
        '^' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit6]),
        '&' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit7]),
        '*' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit8]),
        '(' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit9]),
        ')' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Digit0]),
        '_' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Minus]),
        '+' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Equal]),
        '{' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::BracketLeft]),
        '}' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::BracketRight]),
        '|' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Backslash]),
        ':' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Semicolon]),
        '"' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Quote]),
        '~' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Backquote]),
        '<' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Comma]),
        '>' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Period]),
        '?' => Some(vec![KeyCodeWrapper::ShiftLeft, KeyCodeWrapper::Slash]),
        // Control characters
        '\n' => Some(vec![KeyCodeWrapper::Enter]),
        '\t' => Some(vec![KeyCodeWrapper::Tab]),
        // Unmappable
        _ => None,
    }
}

/// Handler for the `type_text` BRP method.
/// Types text one character per frame, simulating realistic keyboard input.
pub fn type_text_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: TypeTextRequest = if let Some(params) = params {
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

    if request.text.is_empty() {
        return Ok(json!(TypeTextResponse {
            success: true,
            chars_queued: 0,
            skipped: vec![],
        }));
    }

    // Convert text to character queue, tracking unmappable chars
    let mut chars = std::collections::VecDeque::new();
    let mut skipped = Vec::new();

    for c in request.text.chars() {
        if char_to_keys(c).is_some() {
            chars.push_back(c);
        } else {
            skipped.push(c);
        }
    }

    let chars_queued = chars.len();

    // Spawn the typing queue component
    if !chars.is_empty() {
        world.spawn(TextTypingQueue {
            chars,
            current_keys: vec![],
            releasing: false,
        });
    }

    Ok(json!(TypeTextResponse {
        success: true,
        chars_queued,
        skipped,
    }))
}

/// System that processes text typing queues (one character per frame).
pub fn process_text_typing(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TextTypingQueue)>,
    mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
) {
    for (entity, mut queue) in &mut query {
        if queue.releasing {
            // Release the current keys
            if !queue.current_keys.is_empty() {
                let release_events = create_keyboard_events(&queue.current_keys, false);
                for event in release_events {
                    keyboard_events.write(event);
                }
                queue.current_keys.clear();
            }
            queue.releasing = false;
        } else {
            // Press the next character's keys
            if let Some(c) = queue.chars.pop_front() {
                if let Some(keys) = char_to_keys(c) {
                    let press_events = create_keyboard_events(&keys, true);
                    for event in press_events {
                        keyboard_events.write(event);
                    }
                    queue.current_keys = keys;
                    queue.releasing = true;
                }
            } else {
                // All done, despawn
                commands.entity(entity).despawn();
            }
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
        assert_eq!(error.code, INVALID_PARAMS);
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
