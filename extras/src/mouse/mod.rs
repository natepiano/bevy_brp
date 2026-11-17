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
    use std::error::Error;
    use std::time::Duration;

    use bevy::app::App;
    use bevy::app::Update;
    use bevy::ecs::message::Message;
    use bevy::ecs::message::MessageCursor;
    use bevy::input::ButtonState;
    use bevy::input::mouse::MouseButton;
    use bevy::input::mouse::MouseButtonInput;
    use bevy::input::mouse::MouseMotion;
    use bevy::math::Vec2;
    use bevy::prelude::Entity;
    use bevy::prelude::In;
    use bevy::prelude::Messages;
    use bevy::prelude::MinimalPlugins;
    use bevy::prelude::Time;
    use bevy::prelude::Virtual;
    use bevy::prelude::With;
    use bevy::time::TimeUpdateStrategy;
    use bevy::window::CursorMoved;
    use bevy::window::PrimaryWindow;
    use bevy::window::Window;
    use bevy::window::WindowEvent;
    use serde_json::json;

    use super::button;
    use super::button::TimedButtonRelease;
    use super::click;
    use super::constants::DEFAULT_DOUBLE_CLICK_DELAY_MS;
    use super::constants::DEFAULT_MOUSE_DURATION_MS;
    use super::cursor::SimulatedCursorPosition;
    use super::double_click_mouse_handler;
    use super::drag;
    use super::drag_mouse_handler;
    use super::move_mouse_handler;
    use super::send_mouse_button_handler;

    const DRAG_START: Vec2 = Vec2::new(100.0, 100.0);
    const DRAG_END: Vec2 = Vec2::new(200.0, 150.0);
    const DRAG_FRAMES: u32 = 3;
    const INITIAL_POSITIONS: [Vec2; 2] = [Vec2::new(900.0, 700.0), Vec2::new(10.0, 20.0)];

    /// Create an app with one primary window and the mouse operation systems.
    ///
    /// Pause `Time<Virtual>` and advance `Time<Real>` only through [`advance_real_clock`].
    /// The paused clock prevents message buffer swaps, so messages remain readable across frames.
    fn app_with_paused_virtual_clock() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<MouseButtonInput>()
            .add_message::<MouseMotion>()
            .add_message::<CursorMoved>()
            .add_message::<WindowEvent>()
            .init_resource::<SimulatedCursorPosition>()
            .add_systems(Update, button::process_timed_button_releases)
            .add_systems(Update, click::process_scheduled_clicks)
            .add_systems(Update, drag::process_drag_operations)
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

    fn messages_since<M: Message + Clone>(app: &App, cursor: &mut MessageCursor<M>) -> Vec<M> {
        cursor
            .read(app.world().resource::<Messages<M>>())
            .cloned()
            .collect()
    }

    fn app_with_pending_drag(initial: Vec2) -> Result<(App, Entity), Box<dyn Error>> {
        let mut app = app_with_paused_virtual_clock();
        move_mouse_handler(
            In(Some(json!({"position": [initial.x, initial.y]}))),
            app.world_mut(),
        )
        .map_err(|error| format!("{error:?}"))?;
        app.update();
        let window = app
            .world_mut()
            .query_filtered::<Entity, With<PrimaryWindow>>()
            .single(app.world())?;
        drag_mouse_handler(
            In(Some(json!({
                "button": "Left",
                "start": [DRAG_START.x, DRAG_START.y],
                "end": [DRAG_END.x, DRAG_END.y],
                "frames": DRAG_FRAMES
            }))),
            app.world_mut(),
        )
        .map_err(|error| format!("{error:?}"))?;
        Ok((app, window))
    }

    #[test]
    fn drag_moves_to_start_on_a_frame_without_button_events() -> Result<(), Box<dyn Error>> {
        for initial in INITIAL_POSITIONS {
            let (mut app, window) = app_with_pending_drag(initial)?;
            let mut motion_cursor = app
                .world()
                .resource::<Messages<MouseMotion>>()
                .get_cursor_current();
            let mut cursor_cursor = app
                .world()
                .resource::<Messages<CursorMoved>>()
                .get_cursor_current();
            app.update();

            let motions = messages_since(&app, &mut motion_cursor);
            assert_eq!(motions.len(), 1);
            assert_eq!(motions[0].delta, DRAG_START - initial);
            let cursors = messages_since(&app, &mut cursor_cursor);
            assert_eq!(cursors.len(), 1);
            assert_eq!(cursors[0].window, window);
            assert_eq!(cursors[0].position, DRAG_START);
            assert_eq!(cursors[0].delta, Some(DRAG_START - initial));
            assert_eq!(
                app.world()
                    .get::<Window>(window)
                    .ok_or("Missing window")?
                    .cursor_position(),
                Some(DRAG_START)
            );
            assert_eq!(button_events(&app, ButtonState::Pressed).len(), 0);
            assert_eq!(button_events(&app, ButtonState::Released).len(), 0);
        }
        Ok(())
    }

    #[test]
    fn drag_press_frame_has_zero_motion_delta() -> Result<(), Box<dyn Error>> {
        for initial in INITIAL_POSITIONS {
            let (mut app, window) = app_with_pending_drag(initial)?;
            let mut button_cursor = MessageCursor::<MouseButtonInput>::default();
            let mut motion_cursor = app
                .world()
                .resource::<Messages<MouseMotion>>()
                .get_cursor_current();
            let mut cursor_cursor = app
                .world()
                .resource::<Messages<CursorMoved>>()
                .get_cursor_current();
            let mut press_frames = 0;
            for _ in 0..2 {
                app.update();
                let buttons = messages_since(&app, &mut button_cursor);
                let motions = messages_since(&app, &mut motion_cursor);
                let cursors = messages_since(&app, &mut cursor_cursor);
                if buttons
                    .iter()
                    .any(|event| event.state == ButtonState::Pressed)
                {
                    press_frames += 1;
                    assert_eq!(buttons.len(), 1);
                    assert_eq!(buttons[0].button, MouseButton::Left);
                    assert_eq!(buttons[0].window, window);
                    assert!(motions.iter().all(|event| event.delta == Vec2::ZERO));
                    assert!(cursors.iter().all(|event| event.delta == Some(Vec2::ZERO)));
                    assert_eq!(
                        app.world()
                            .get::<Window>(window)
                            .ok_or("Missing window")?
                            .cursor_position(),
                        Some(DRAG_START)
                    );
                    assert_eq!(
                        app.world()
                            .resource::<SimulatedCursorPosition>()
                            .get_position(window),
                        DRAG_START
                    );
                }
            }
            assert_eq!(press_frames, 1);
        }
        Ok(())
    }

    #[test]
    fn drag_reaches_end_and_releases_once() -> Result<(), Box<dyn Error>> {
        let (mut app, window) = app_with_pending_drag(INITIAL_POSITIONS[0])?;
        for _ in 0..DRAG_FRAMES + 4 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<Window>(window)
                .ok_or("Missing window")?
                .cursor_position(),
            Some(DRAG_END)
        );
        assert_eq!(
            app.world()
                .resource::<SimulatedCursorPosition>()
                .get_position(window),
            DRAG_END
        );
        assert_eq!(button_events(&app, ButtonState::Pressed).len(), 1);
        let releases = button_events(&app, ButtonState::Released);
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].button, MouseButton::Left);
        assert_eq!(releases[0].window, window);
        Ok(())
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
