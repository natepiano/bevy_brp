//! Type-text handler: sequential character-by-character typing via BRP.

use std::str::FromStr;

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

use super::create_keyboard_events;
use super::create_keyboard_events_with_text;
use super::key_code::KeyCodeWrapper;

/// Phase of the text typing state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypingPhase {
    /// Ready to press the next character's keys
    PressNext,
    /// Need to release the currently held keys before pressing the next character
    ReleaseCurrentKeys,
}

/// Component for sequential text typing (one character per frame).
/// Used by `type_text` RPC to simulate realistic typing.
#[derive(Component)]
pub(crate) struct TextTypingQueue {
    /// Characters remaining to type
    pub chars:        std::collections::VecDeque<char>,
    /// Currently pressed keys (waiting for release next frame)
    pub current_keys: Vec<KeyCodeWrapper>,
    /// The character we're currently typing (for proper text field on shifted chars)
    pub current_char: Option<char>,
    /// Current phase of the typing state machine
    pub phase:        TypingPhase,
}

/// Request structure for `type_text`
#[derive(Debug, Deserialize)]
pub(crate) struct TypeTextRequest {
    /// Text to type (supports letters, numbers, symbols, newlines, tabs)
    pub text: String,
}

/// Response structure for `type_text`
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TypeTextResponse {
    /// Whether the operation was initiated successfully
    pub success:      bool,
    /// Number of characters queued for typing
    pub chars_queued: usize,
    /// Characters that couldn't be mapped to keys (skipped)
    pub skipped:      Vec<char>,
}

/// Convert a character to the key(s) needed to type it.
/// Returns None for unmappable characters.
fn char_to_keys(c: char) -> Option<Vec<KeyCodeWrapper>> {
    match c {
        // Lowercase letters
        'a'..='z' => {
            let key_name = format!("Key{}", c.to_ascii_uppercase());
            KeyCodeWrapper::from_str(&key_name).ok().map(|k| vec![k])
        },
        // Uppercase letters (need Shift)
        'A'..='Z' => {
            let key_name = format!("Key{c}");
            KeyCodeWrapper::from_str(&key_name)
                .ok()
                .map(|k| vec![KeyCodeWrapper::ShiftLeft, k])
        },
        // Numbers
        '0'..='9' => {
            let key_name = format!("Digit{c}");
            KeyCodeWrapper::from_str(&key_name).ok().map(|k| vec![k])
        },
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
        '}' => Some(vec![
            KeyCodeWrapper::ShiftLeft,
            KeyCodeWrapper::BracketRight,
        ]),
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
pub(crate) fn type_text_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
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
            success:      true,
            chars_queued: 0,
            skipped:      vec![],
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
            current_char: None,
            phase: TypingPhase::PressNext,
        });
    }

    Ok(json!(TypeTextResponse {
        success: true,
        chars_queued,
        skipped,
    }))
}

/// System that processes text typing queues (one character per frame).
pub(crate) fn process_text_typing(
    mut commands: Commands,
    mut query: Query<(Entity, &mut TextTypingQueue)>,
    mut keyboard_events: MessageWriter<bevy::input::keyboard::KeyboardInput>,
    mut window_events: MessageWriter<WindowEvent>,
) {
    for (entity, mut queue) in &mut query {
        match queue.phase {
            TypingPhase::ReleaseCurrentKeys => {
                // Release the current keys
                if !queue.current_keys.is_empty() {
                    let release_events =
                        create_keyboard_events(&queue.current_keys, ButtonState::Released);
                    for event in release_events {
                        window_events.write(WindowEvent::from(event.clone()));
                        keyboard_events.write(event);
                    }
                    queue.current_keys.clear();
                    queue.current_char = None;
                }
                queue.phase = TypingPhase::PressNext;
            },
            TypingPhase::PressNext => {
                // Press the next character's keys
                if let Some(c) = queue.chars.pop_front() {
                    if let Some(keys) = char_to_keys(c) {
                        // Pass the actual character so shifted chars get correct text field
                        let press_events =
                            create_keyboard_events_with_text(&keys, ButtonState::Pressed, Some(c));
                        for event in press_events {
                            window_events.write(WindowEvent::from(event.clone()));
                            keyboard_events.write(event);
                        }
                        queue.current_keys = keys;
                        queue.current_char = Some(c);
                        queue.phase = TypingPhase::ReleaseCurrentKeys;
                    }
                } else {
                    // All done, despawn
                    commands.entity(entity).despawn();
                }
            },
        }
    }
}
