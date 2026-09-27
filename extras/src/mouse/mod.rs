//! Mouse input simulation for Bevy Remote Protocol
//!
//! This module provides comprehensive mouse input simulation including:
//! - Cursor movement (delta and absolute positioning)
//! - Mouse button presses (single click, double click)
//! - Drag operations with interpolation
//! - Scroll wheel events
//! - Trackpad gestures (pinch, rotation, double tap)
//!
//! All operations support multi-window targeting.

mod button;
mod click;
mod constants;
mod cursor;
mod drag;
mod gestures;
mod scroll;
mod support;

use bevy::prelude::*;
use cursor::SimulatedCursorPosition;

pub(crate) use self::button::send_mouse_button_handler;
pub(crate) use self::click::click_mouse_handler;
pub(crate) use self::click::double_click_mouse_handler;
pub(crate) use self::cursor::move_mouse_handler;
pub(crate) use self::drag::drag_mouse_handler;
pub(crate) use self::gestures::double_tap_gesture_handler;
pub(crate) use self::gestures::pinch_gesture_handler;
pub(crate) use self::gestures::rotation_gesture_handler;
pub(crate) use self::scroll::scroll_mouse_handler;

pub(super) struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulatedCursorPosition>();
        app.add_systems(Update, cursor::sync_cursor_position);
        app.add_systems(Update, button::process_timed_button_releases);
        app.add_systems(Update, click::process_scheduled_clicks);
        app.add_systems(Update, drag::process_drag_operations);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::app::App;
    use bevy::app::Update;
    use bevy::ecs::message::MessageCursor;
    use bevy::input::ButtonState;
    use bevy::input::mouse::MouseButton;
    use bevy::input::mouse::MouseButtonInput;
    use bevy::prelude::In;
    use bevy::prelude::Messages;
    use bevy::prelude::MinimalPlugins;
    use bevy::prelude::Time;
    use bevy::prelude::Virtual;
    use bevy::time::TimeUpdateStrategy;
    use bevy::window::PrimaryWindow;
    use bevy::window::Window;
    use bevy::window::WindowEvent;
    use serde_json::json;

    use super::button;
    use super::button::TimedButtonRelease;
    use super::click;
    use super::constants::DEFAULT_DOUBLE_CLICK_DELAY_MS;
    use super::constants::DEFAULT_MOUSE_DURATION_MS;
    use super::double_click_mouse_handler;
    use super::send_mouse_button_handler;

    /// An app with one primary window running `process_timed_button_releases` and
    /// `process_scheduled_clicks`, with `Time<Virtual>` paused and `Time<Real>` advancing only
    /// through [`advance_real_clock`].
    ///
    /// `TimePlugin` still runs `time_system`, which copies the paused `Time<Virtual>` into `Time`
    /// every frame, as in an app that has paused its game clock. Because the virtual clock never
    /// advances, `FixedMain` never runs and `signal_message_update_system` never marks the
    /// `MessageRegistry` ready, so `Messages` buffers are not swapped: every message written
    /// during a test stays readable.
    fn app_with_paused_virtual_clock() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<MouseButtonInput>()
            .add_message::<WindowEvent>()
            .add_systems(Update, button::process_timed_button_releases)
            .add_systems(Update, click::process_scheduled_clicks)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        // The first `time_system` run records the start instant without advancing `Time<Real>`.
        app.update();
        app
    }

    /// Advances `Time<Real>` by `ms` milliseconds and runs one frame.
    fn advance_real_clock(app: &mut App, ms: u32) {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            u64::from(ms),
        )));
        app.update();
    }

    fn button_events(app: &App, state: ButtonState) -> Vec<MouseButtonInput> {
        let messages = app.world().resource::<Messages<MouseButtonInput>>();
        MessageCursor::default()
            .read(messages)
            .filter(|event| event.state == state)
            .copied()
            .collect()
    }

    /// The hold is measured on the wall clock: a button pressed while the app
    /// has paused its virtual clock still comes back up after its duration.
    #[test]
    fn button_releases_while_the_virtual_clock_is_paused() {
        let mut app = app_with_paused_virtual_clock();

        let result =
            send_mouse_button_handler(In(Some(json!({ "button": "Left" }))), app.world_mut());
        assert!(result.is_ok());

        advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS);

        assert_eq!(app.world().resource::<Time>().delta(), Duration::ZERO);
        let releases = button_events(&app, ButtonState::Released);
        assert_eq!(
            releases.len(),
            1,
            "the button must be released on the real clock"
        );
        assert_eq!(releases[0].button, MouseButton::Left);
    }

    /// The second click of a double click waits on the wall clock too, so a
    /// paused virtual clock cannot leave it half-done.
    #[test]
    fn double_click_completes_while_the_virtual_clock_is_paused() {
        let mut app = app_with_paused_virtual_clock();

        let result =
            double_click_mouse_handler(In(Some(json!({ "button": "Left" }))), app.world_mut());
        assert!(result.is_ok());

        // `double_click_mouse_handler` writes the first press and release at once; the second
        // press waits on `ScheduledClick::delay_timer`, and its release on
        // `TimedButtonRelease::timer`. The counts below are running totals.
        advance_real_clock(&mut app, DEFAULT_DOUBLE_CLICK_DELAY_MS);
        assert_eq!(
            button_events(&app, ButtonState::Pressed).len(),
            2,
            "the second press fires on the real clock"
        );
        assert_eq!(button_events(&app, ButtonState::Released).len(), 1);

        advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS);
        assert_eq!(
            button_events(&app, ButtonState::Released).len(),
            2,
            "the second click's release fires on the real clock"
        );
        let pending = app
            .world_mut()
            .query::<&TimedButtonRelease>()
            .iter(app.world())
            .count();
        assert_eq!(pending, 0);
    }
}
