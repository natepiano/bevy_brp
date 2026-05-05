//! Keyboard event creation utilities

use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::keyboard::NativeKey;
use bevy::prelude::Entity;

use super::key_code::KeyCodeWrapper;

/// Create keyboard events from validated key code wrappers.
///
/// Populates `logical_key` and `text` fields for printable characters,
/// enabling text input simulation that works with Bevy's text input systems.
pub(super) fn create_keyboard_events(
    wrappers: &[KeyCodeWrapper],
    state: ButtonState,
) -> Vec<KeyboardInput> {
    create_keyboard_events_with_text(wrappers, state, None)
}

/// Create keyboard events with an optional target character override.
///
/// When `target_char` is provided, it will be used as the `text` field
/// for the final non-modifier key in the sequence. This is essential for
/// shifted characters (e.g., `!` requires Shift+1, but text should be `!`).
pub(super) fn create_keyboard_events_with_text(
    wrappers: &[KeyCodeWrapper],
    state: ButtonState,
    target_char: Option<char>,
) -> Vec<KeyboardInput> {
    // Find the last non-modifier key index (that's where we set the text)
    let last_non_modifier_idx = wrappers.iter().rposition(|w| {
        !matches!(
            w,
            KeyCodeWrapper::ShiftLeft
                | KeyCodeWrapper::ShiftRight
                | KeyCodeWrapper::ControlLeft
                | KeyCodeWrapper::ControlRight
                | KeyCodeWrapper::AltLeft
                | KeyCodeWrapper::AltRight
                | KeyCodeWrapper::SuperLeft
                | KeyCodeWrapper::SuperRight
        )
    });

    wrappers
        .iter()
        .enumerate()
        .map(|(idx, &wrapper)| {
            let key_code = wrapper.to_key_code();
            let is_target_key = Some(idx) == last_non_modifier_idx;

            // Use target_char for the final non-modifier key, otherwise use to_char()
            let char_opt = if is_target_key && target_char.is_some() {
                target_char
            } else {
                wrapper.to_char()
            };

            // Build logical_key and text based on whether this is a printable character.
            let (logical_key, text) =
                char_opt.map_or((Key::Unidentified(NativeKey::Unidentified), None), |c| {
                    let s: String = c.to_string();
                    // Only populate text on press events, not release, and only for the target key
                    let text = if state == ButtonState::Pressed && is_target_key {
                        Some(s.clone().into())
                    } else {
                        None
                    };
                    (Key::Character(s.into()), text)
                });

            KeyboardInput {
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
