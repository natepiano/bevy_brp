use std::time::Duration;

use bevy::app::App;
use bevy::app::PreUpdate;
use bevy::app::Update;
use bevy::ecs::message::MessageCursor;
use bevy::ecs::schedule::Schedule;
use bevy::input::ButtonInput;
use bevy::input::ButtonState;
use bevy::input::InputPlugin;
use bevy::input::InputSystems;
use bevy::input::keyboard::KeyCode;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButton;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::math::Vec2;
use bevy::prelude::In;
use bevy::prelude::IntoScheduleConfigs;
use bevy::prelude::MessageReader;
use bevy::prelude::Messages;
use bevy::prelude::MinimalPlugins;
use bevy::prelude::Res;
use bevy::prelude::ResMut;
use bevy::prelude::Resource;
use bevy::prelude::Time;
use bevy::prelude::Virtual;
use bevy::time::TimeUpdateStrategy;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::Window;
use bevy::window::WindowEvent;
use serde_json::json;

use super::MousePlugin;
use super::button::TimedButtonRelease;
use super::click_mouse_handler;
use super::constants::DEFAULT_DOUBLE_CLICK_DELAY_MS;
use super::constants::DEFAULT_MOUSE_DURATION_MS;
use super::cursor;
use super::double_click_mouse_handler;
use super::drag;
use super::drag_mouse_handler;
use super::send_mouse_button_handler;

/// Frames for the press, the initial position, both releases, and input delivery.
const DRAG_COMPLETION_OVERHEAD_FRAMES: u32 = 5;

/// An app with one primary window and the input and mouse plugins.
/// The virtual clock stays paused. [`advance_real_clock`] advances the real clock.
///
/// `TimePlugin` copies the paused `Time<Virtual>` into `Time` each frame.
/// The paused clock prevents `FixedMain` from marking the message registry ready.
/// The message buffers retain every message for the test.
fn app_with_paused_virtual_clock() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin, MousePlugin))
        .add_message::<CursorMoved>()
        .add_message::<WindowEvent>()
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

    let result = send_mouse_button_handler(In(Some(json!({ "button": "Left" }))), app.world_mut());
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

    let result = double_click_mouse_handler(In(Some(json!({ "button": "Left" }))), app.world_mut());
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

/// Input visible to the app after Bevy processes the previous frame's messages.
struct InputFrame {
    keys:          ButtonInput<KeyCode>,
    keyboard:      Vec<KeyboardInput>,
    buttons:       Vec<MouseButtonInput>,
    cursors:       Vec<CursorMoved>,
    motion:        Vec<MouseMotion>,
    window_events: Vec<WindowEvent>,
}

/// Input observations in frame order.
#[derive(Default, Resource)]
struct InputFrames(Vec<InputFrame>);

fn record_input_frame(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard: MessageReader<KeyboardInput>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut cursors: MessageReader<CursorMoved>,
    mut motion: MessageReader<MouseMotion>,
    mut window_events: MessageReader<WindowEvent>,
    mut frames: ResMut<InputFrames>,
) {
    frames.0.push(InputFrame {
        keys:          (*keys).clone(),
        keyboard:      keyboard.read().cloned().collect(),
        buttons:       buttons.read().copied().collect(),
        cursors:       cursors.read().cloned().collect(),
        motion:        motion.read().copied().collect(),
        window_events: window_events.read().cloned().collect(),
    });
}

fn observe_input(app: &mut App) {
    app.init_resource::<InputFrames>()
        .add_systems(PreUpdate, record_input_frame.after(InputSystems));
}

fn click_release_frame(frames: &[InputFrame]) -> Result<usize, String> {
    frames
        .iter()
        .position(|frame| {
            frame.buttons.iter().any(|event| {
                event.button == MouseButton::Left && event.state == ButtonState::Released
            })
        })
        .ok_or_else(|| String::from("the app must receive the mouse button release"))
}

#[test]
fn click_with_shift_holds_shift_across_the_click() -> Result<(), String> {
    let mut app = app_with_paused_virtual_clock();
    observe_input(&mut app);
    let response = click_mouse_handler(
        In(Some(json!({ "button": "Left", "modifiers": ["Shift"] }))),
        app.world_mut(),
    )
    .map_err(|error| format!("the click must succeed: {error:?}"))?;

    advance_real_clock(&mut app, 0);
    advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS);
    advance_real_clock(&mut app, 0);
    advance_real_clock(&mut app, 0);

    let frames = &app.world().resource::<InputFrames>().0;
    let press_frame = frames
        .iter()
        .position(|frame| {
            frame
                .buttons
                .iter()
                .any(|event| event.state == ButtonState::Pressed)
        })
        .ok_or_else(|| String::from("the app must receive the mouse button press"))?;
    let events = &frames[press_frame].window_events;
    let key_press = events
        .iter()
        .position(|event| {
            matches!(event, WindowEvent::KeyboardInput(input)
            if input.key_code == KeyCode::ShiftLeft && input.state == ButtonState::Pressed)
        })
        .ok_or_else(|| String::from("Shift must be pressed with the mouse button"))?;
    let button_press = events
        .iter()
        .position(|event| {
            matches!(event, WindowEvent::MouseButtonInput(input)
            if input.state == ButtonState::Pressed)
        })
        .ok_or_else(|| String::from("the window must receive the mouse button press"))?;
    assert!(key_press < button_press);

    let release_frame = click_release_frame(frames)?;
    assert!(
        frames[press_frame..=release_frame]
            .iter()
            .all(|frame| frame.keys.pressed(KeyCode::ShiftLeft))
    );
    assert!(frames[release_frame].keyboard.iter().all(|event| {
        event.key_code != KeyCode::ShiftLeft || event.state != ButtonState::Released
    }));
    assert!(frames[release_frame + 1].keyboard.iter().any(|event| {
        event.key_code == KeyCode::ShiftLeft && event.state == ButtonState::Released
    }));
    assert!(!frames[release_frame + 1].keys.pressed(KeyCode::ShiftLeft));
    assert_eq!(response["modifiers"], json!(["Shift"]));
    Ok(())
}

fn assert_overlapping_click_shares_control(
    frames: &[InputFrame],
    release_frame: usize,
) -> Result<(), String> {
    let click_release = frames
        .iter()
        .position(|frame| {
            frame.buttons.iter().any(|event| {
                event.button == MouseButton::Right && event.state == ButtonState::Released
            })
        })
        .ok_or_else(|| String::from("the overlapping click must release its button"))?;
    assert!(click_release < release_frame);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let changes = frames
            .iter()
            .flat_map(|frame| &frame.keyboard)
            .filter(|event| event.key_code == KeyCode::ControlLeft && event.state == state)
            .count();
        assert_eq!(
            changes, 1,
            "overlapping operations must share one Control hold"
        );
    }
    Ok(())
}

#[test]
fn drag_with_control_holds_control_for_every_drag_frame() -> Result<(), String> {
    const DRAG_FRAME_COUNT: u32 = 3;
    const OBSERVATION_FRAME_COUNT: u32 = DRAG_FRAME_COUNT + DRAG_COMPLETION_OVERHEAD_FRAMES;

    for overlapping_click in [false, true] {
        let mut app = app_with_paused_virtual_clock();
        observe_input(&mut app);
        let response = drag_mouse_handler(
            In(Some(json!({
                "button": "Left", "start": [10.0, 20.0], "end": [40.0, 50.0],
                "frames": DRAG_FRAME_COUNT, "modifiers": ["Control"]
            }))),
            app.world_mut(),
        )
        .map_err(|error| format!("the drag must succeed: {error:?}"))?;

        advance_real_clock(&mut app, 0);
        if overlapping_click {
            click_mouse_handler(
                In(Some(json!({ "button": "Right", "modifiers": ["Control"] }))),
                app.world_mut(),
            )
            .map_err(|error| format!("the overlapping click must succeed: {error:?}"))?;
        }
        for frame in 1..OBSERVATION_FRAME_COUNT {
            let elapsed_ms = if frame == 1 {
                DEFAULT_MOUSE_DURATION_MS
            } else {
                0
            };
            advance_real_clock(&mut app, elapsed_ms);
        }

        let frames = &app.world().resource::<InputFrames>().0;
        let motion_frames: Vec<_> = frames
            .iter()
            .filter(|frame| !frame.cursors.is_empty())
            .collect();
        assert!(!motion_frames.is_empty(), "the drag must move the cursor");
        for frame in motion_frames {
            assert!(frame.keys.pressed(KeyCode::ControlLeft));
            assert!(frame.keyboard.iter().all(|event| {
                event.key_code != KeyCode::ControlLeft || event.state != ButtonState::Released
            }));
        }
        let release_frame = click_release_frame(frames)?;
        assert!(frames[release_frame].keys.pressed(KeyCode::ControlLeft));
        assert!(frames[release_frame + 1].keyboard.iter().any(|event| {
            event.key_code == KeyCode::ControlLeft && event.state == ButtonState::Released
        }));
        assert!(!frames[release_frame + 1].keys.pressed(KeyCode::ControlLeft));
        let press_frame = frames
            .iter()
            .find(|frame| {
                frame
                    .buttons
                    .iter()
                    .any(|event| event.state == ButtonState::Pressed)
            })
            .ok_or_else(|| String::from("the drag must press the mouse button"))?;
        assert!(press_frame.keys.pressed(KeyCode::ControlLeft));
        let events = &press_frame.window_events;
        let key_press = events
            .iter()
            .position(|event| {
                matches!(
                    event, WindowEvent::KeyboardInput(input)
                    if input.key_code == KeyCode::ControlLeft && input.state == ButtonState::Pressed
                )
            })
            .ok_or_else(|| String::from("Control must be pressed before the drag button"))?;
        let button_press = events
            .iter()
            .position(|event| {
                matches!(
                    event, WindowEvent::MouseButtonInput(input) if input.state == ButtonState::Pressed
                )
            })
            .ok_or_else(|| String::from("the drag must press the mouse button"))?;
        assert!(key_press < button_press);
        assert_eq!(response["modifiers"], json!(["Control"]));
        if overlapping_click {
            assert_overlapping_click_shares_control(frames, release_frame)?;
        }
    }
    Ok(())
}

#[test]
fn click_without_modifiers_sends_no_keyboard_input() -> Result<(), String> {
    let mut app = app_with_paused_virtual_clock();
    observe_input(&mut app);
    let response = click_mouse_handler(In(Some(json!({ "button": "Left" }))), app.world_mut())
        .map_err(|error| format!("the old click request must succeed: {error:?}"))?;
    advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS);
    advance_real_clock(&mut app, 0);
    advance_real_clock(&mut app, 0);

    assert!(app.world().resource::<Messages<KeyboardInput>>().is_empty());
    let frames = &app.world().resource::<InputFrames>().0;
    assert!(frames.iter().all(|frame| frame.keyboard.is_empty()));
    assert!(frames.iter().all(|frame| {
        frame
            .window_events
            .iter()
            .all(|event| !matches!(event, WindowEvent::KeyboardInput(_)))
    }));
    assert_eq!(button_events(&app, ButtonState::Pressed).len(), 1);
    assert_eq!(button_events(&app, ButtonState::Released).len(), 1);
    assert!(response.get("modifiers").is_none());
    Ok(())
}

#[test]
fn duplicate_modifiers_press_once() -> Result<(), String> {
    for (requested, expected, keys) in [
        (
            json!(["Shift", "Shift"]),
            json!(["Shift"]),
            vec![KeyCode::ShiftLeft],
        ),
        (
            json!(["Super", "Shift", "Alt", "Shift", "Super"]),
            json!(["Super", "Shift", "Alt"]),
            vec![KeyCode::SuperLeft, KeyCode::ShiftLeft, KeyCode::AltLeft],
        ),
    ] {
        let mut app = app_with_paused_virtual_clock();
        observe_input(&mut app);
        let response = click_mouse_handler(
            In(Some(json!({ "button": "Left", "modifiers": requested }))),
            app.world_mut(),
        )
        .map_err(|error| format!("the duplicate modifier request must succeed: {error:?}"))?;
        advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS);
        advance_real_clock(&mut app, 0);
        advance_real_clock(&mut app, 0);

        let frames = &app.world().resource::<InputFrames>().0;
        for state in [ButtonState::Pressed, ButtonState::Released] {
            let actual: Vec<_> = frames
                .iter()
                .flat_map(|frame| &frame.keyboard)
                .filter(|event| event.state == state)
                .map(|event| event.key_code)
                .collect();
            assert_eq!(
                actual, keys,
                "each modifier must change state once in request order"
            );
        }
        assert_eq!(response["modifiers"], expected);
    }
    Ok(())
}

#[test]
fn unknown_modifier_is_rejected() -> Result<(), String> {
    let mut app = app_with_paused_virtual_clock();
    for result in [
        click_mouse_handler(
            In(Some(json!({ "button": "Left", "modifiers": ["Hyper"] }))),
            app.world_mut(),
        ),
        drag_mouse_handler(
            In(Some(json!({
                "button": "Left", "start": [10.0, 20.0], "end": [40.0, 50.0],
                "frames": 3, "modifiers": ["Hyper"]
            }))),
            app.world_mut(),
        ),
    ] {
        let error = result
            .err()
            .ok_or_else(|| String::from("Hyper must be rejected"))?;
        assert_eq!(error.code, bevy_remote::error_codes::INVALID_PARAMS);
        assert!(
            error.message.contains("modifiers"),
            "the error must name modifiers: {}",
            error.message
        );
        assert!(
            error.message.contains("Hyper"),
            "the error must name Hyper: {}",
            error.message
        );
    }
    Ok(())
}

#[test]
fn modifier_holds_while_the_virtual_clock_is_paused() -> Result<(), String> {
    let mut app = app_with_paused_virtual_clock();
    observe_input(&mut app);
    let result = click_mouse_handler(
        In(Some(json!({ "button": "Left", "modifiers": ["Shift"] }))),
        app.world_mut(),
    );
    assert!(result.is_ok());

    advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS / 2);
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ShiftLeft)
    );
    assert_eq!(
        button_events(&app, ButtonState::Released),
        Vec::<MouseButtonInput>::new()
    );
    advance_real_clock(&mut app, DEFAULT_MOUSE_DURATION_MS / 2);
    advance_real_clock(&mut app, 0);
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ShiftLeft)
    );
    assert_eq!(button_events(&app, ButtonState::Released).len(), 1);
    advance_real_clock(&mut app, 0);
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ShiftLeft)
    );
    assert_eq!(app.world().resource::<Time>().delta(), Duration::ZERO);
    assert_eq!(
        app.world().resource::<Time<Virtual>>().elapsed(),
        Duration::ZERO
    );
    let frames = &app.world().resource::<InputFrames>().0;
    let release_frame = click_release_frame(frames)?;
    assert!(frames[release_frame + 1].keyboard.iter().any(|event| {
        event.key_code == KeyCode::ShiftLeft && event.state == ButtonState::Released
    }));
    Ok(())
}

#[test]
fn drag_motion_deltas_match_cursor_positions_in_both_system_orders() -> Result<(), String> {
    const DRAG_FRAME_COUNT: u32 = 3;
    const OBSERVATION_FRAME_COUNT: u32 = DRAG_FRAME_COUNT + DRAG_COMPLETION_OVERHEAD_FRAMES;

    for sync_after_drag in [false, true] {
        let mut app = app_with_paused_virtual_clock();
        // Keep the plugin's resources and run only the two systems under test in Update.
        app.add_schedule(Schedule::new(Update));
        if sync_after_drag {
            app.add_systems(
                Update,
                (drag::process_drag_operations, cursor::sync_cursor_position)
                    .chain_ignore_deferred(),
            );
        } else {
            app.add_systems(
                Update,
                (cursor::sync_cursor_position, drag::process_drag_operations)
                    .chain_ignore_deferred(),
            );
        }
        observe_input(&mut app);
        drag_mouse_handler(
            In(Some(json!({
                "button": "Left", "start": [0.0, 0.0], "end": [30.0, 0.0],
                "frames": DRAG_FRAME_COUNT
            }))),
            app.world_mut(),
        )
        .map_err(|error| format!("the drag must succeed: {error:?}"))?;
        for _ in 0..OBSERVATION_FRAME_COUNT {
            advance_real_clock(&mut app, 0);
        }

        let frames = &app.world().resource::<InputFrames>().0;
        let positions: Vec<_> = frames
            .iter()
            .flat_map(|frame| &frame.cursors)
            .map(|event| event.position)
            .collect();
        assert_eq!(
            positions,
            [
                Vec2::ZERO,
                Vec2::ZERO,
                Vec2::new(10.0, 0.0),
                Vec2::new(20.0, 0.0),
                Vec2::new(30.0, 0.0),
            ],
            "the cursor must follow the requested path; sync_after_drag={sync_after_drag}"
        );
        let motion: Vec2 = frames
            .iter()
            .flat_map(|frame| &frame.motion)
            .map(|event| event.delta)
            .sum();
        let start = Vec2::ZERO;
        let end = Vec2::new(30.0, 0.0);
        assert_eq!(
            motion,
            end - start,
            "motion must equal the requested displacement; sync_after_drag={sync_after_drag}"
        );
    }
    Ok(())
}
