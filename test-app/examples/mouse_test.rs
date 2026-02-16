//! Mouse input test application for BRP extras
//!
//! This example demonstrates and tests all mouse input functionality provided by `bevy_brp_extras`.
//! It creates two windows (primary and secondary) to test window-specific targeting.
//!
//! The `MouseStateTracker` resource tracks all mouse interactions and can be queried via BRP
//! to verify correct behavior during integration tests.

use bevy::camera::RenderTarget;
use bevy::input::gestures::DoubleTapGesture;
use bevy::input::gestures::PinchGesture;
use bevy::input::gestures::RotationGesture;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy::window::WindowRef;
use bevy::window::WindowResolution;
use bevy_brp_extras::BrpExtrasPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Mouse Test - Primary".to_string(),
                    resolution: WindowResolution::new(600, 400),
                    position: WindowPosition::At(IVec2::new(50, 50)),
                    mode: WindowMode::Windowed,
                    ..default()
                }),
                ..default()
            }),
            BrpExtrasPlugin::new(),
        ))
        .init_resource::<MouseStateTracker>()
        .add_systems(Startup, (setup_windows, setup_ui).chain())
        .add_systems(PostStartup, position_secondary_window)
        .add_systems(Update, minimize_window)
        .add_systems(
            Update,
            (
                track_mouse_buttons,
                track_click_events,
                update_button_durations,
                track_cursor_position,
                track_mouse_wheel,
                track_mouse_motion,
                track_gestures,
                update_audit_display,
            ),
        )
        .run();
}

/// Resource tracking all mouse input state for testing purposes
#[derive(Resource, Reflect)]
#[reflect(Resource)]
#[allow(clippy::struct_excessive_bools)]
struct MouseStateTracker {
    // Cursor positions per window with timestamps
    primary_window_position:    Vec2,
    primary_cursor_timestamp:   f32,
    secondary_window_position:  Vec2,
    secondary_cursor_timestamp: f32,

    // Motion tracking with timestamp
    motion_delta_total: Vec2,
    motion_timestamp:   f32,

    // Button states with timestamps and durations
    left_pressed:      bool,
    left_timestamp:    f32,
    left_duration:     f32,
    right_pressed:     bool,
    right_timestamp:   f32,
    right_duration:    f32,
    middle_pressed:    bool,
    middle_timestamp:  f32,
    middle_duration:   f32,
    back_pressed:      bool,
    back_timestamp:    f32,
    back_duration:     f32,
    forward_pressed:   bool,
    forward_timestamp: f32,
    forward_duration:  f32,

    // Scroll per window with timestamps
    primary_scroll_x_total:     f32,
    primary_scroll_y_total:     f32,
    primary_scroll_unit:        String,
    primary_scroll_timestamp:   f32,
    secondary_scroll_x_total:   f32,
    secondary_scroll_y_total:   f32,
    secondary_scroll_unit:      String,
    secondary_scroll_timestamp: f32,

    // Gestures per window with timestamps
    primary_pinch_total:            f32,
    primary_pinch_timestamp:        f32,
    primary_rotation_total:         f32,
    primary_rotation_timestamp:     f32,
    primary_double_tap_timestamp:   f32,
    secondary_pinch_total:          f32,
    secondary_pinch_timestamp:      f32,
    secondary_rotation_total:       f32,
    secondary_rotation_timestamp:   f32,
    secondary_double_tap_timestamp: f32,

    // Click tracking per window with timestamps and positions
    primary_click_timestamp:         f32,
    primary_click_position:          Vec2,
    primary_doubleclick_timestamp:   f32,
    primary_doubleclick_position:    Vec2,
    secondary_click_timestamp:       f32,
    secondary_click_position:        Vec2,
    secondary_doubleclick_timestamp: f32,
    secondary_doubleclick_position:  Vec2,

    // Track which window cursor is currently in
    cursor_window: Option<Entity>,
}

impl Default for MouseStateTracker {
    fn default() -> Self {
        Self {
            primary_window_position:         Vec2::ZERO,
            primary_cursor_timestamp:        0.0,
            secondary_window_position:       Vec2::ZERO,
            secondary_cursor_timestamp:      0.0,
            motion_delta_total:              Vec2::ZERO,
            motion_timestamp:                0.0,
            left_pressed:                    false,
            left_timestamp:                  0.0,
            left_duration:                   0.0,
            right_pressed:                   false,
            right_timestamp:                 0.0,
            right_duration:                  0.0,
            middle_pressed:                  false,
            middle_timestamp:                0.0,
            middle_duration:                 0.0,
            back_pressed:                    false,
            back_timestamp:                  0.0,
            back_duration:                   0.0,
            forward_pressed:                 false,
            forward_timestamp:               0.0,
            forward_duration:                0.0,
            primary_scroll_x_total:          0.0,
            primary_scroll_y_total:          0.0,
            primary_scroll_unit:             String::new(),
            primary_scroll_timestamp:        0.0,
            secondary_scroll_x_total:        0.0,
            secondary_scroll_y_total:        0.0,
            secondary_scroll_unit:           String::new(),
            secondary_scroll_timestamp:      0.0,
            primary_pinch_total:             0.0,
            primary_pinch_timestamp:         0.0,
            primary_rotation_total:          0.0,
            primary_rotation_timestamp:      0.0,
            primary_double_tap_timestamp:    0.0,
            secondary_pinch_total:           0.0,
            secondary_pinch_timestamp:       0.0,
            secondary_rotation_total:        0.0,
            secondary_rotation_timestamp:    0.0,
            secondary_double_tap_timestamp:  0.0,
            primary_click_timestamp:         0.0,
            primary_click_position:          Vec2::ZERO,
            primary_doubleclick_timestamp:   0.0,
            primary_doubleclick_position:    Vec2::ZERO,
            secondary_click_timestamp:       0.0,
            secondary_click_position:        Vec2::ZERO,
            secondary_doubleclick_timestamp: 0.0,
            secondary_doubleclick_position:  Vec2::ZERO,
            cursor_window:                   None,
        }
    }
}

/// Marker component for secondary window
#[derive(Component, Reflect)]
#[reflect(Component)]
struct SecondaryWindow;

/// Marker component for primary window audit display
#[derive(Component)]
struct PrimaryAuditDisplay;

/// Marker component for secondary window audit display
#[derive(Component)]
struct SecondaryAuditDisplay;

fn setup_windows(mut commands: Commands) {
    // Spawn secondary window - will be repositioned in PostStartup
    commands.spawn((
        Window {
            title: "Mouse Test - Secondary".to_string(),
            resolution: WindowResolution::new(600, 400),
            position: WindowPosition::Automatic,
            mode: WindowMode::Windowed,
            ..default()
        },
        SecondaryWindow,
    ));
}

type PrimaryWindowQuery<'w, 's> = Query<'w, 's, &'static Window, With<PrimaryWindow>>;
type SecondaryWindowQuery<'w, 's> = Query<'w, 's, &'static mut Window, With<SecondaryWindow>>;

fn position_secondary_window(mut windows: ParamSet<(PrimaryWindowQuery, SecondaryWindowQuery)>) {
    // First, get primary window info
    let (primary_pos, primary_width) = {
        let primary_query = windows.p0();
        let Ok(primary) = primary_query.single() else {
            return;
        };
        let WindowPosition::At(pos) = primary.position else {
            return;
        };
        (pos, primary.resolution.physical_width())
    };

    // Then, update secondary window
    let mut secondary_query = windows.p1();
    let Ok(mut secondary) = secondary_query.single_mut() else {
        return;
    };

    let gap = 20; // 20px gap between windows
    let secondary_x = primary_pos.x + primary_width.cast_signed() + gap;

    info!(
        "Positioning windows - Primary: x={}, width={}, ends at {}. Secondary: x={}",
        primary_pos.x,
        primary_width,
        primary_pos.x + primary_width.cast_signed(),
        secondary_x
    );

    secondary.position = WindowPosition::At(IVec2::new(secondary_x, primary_pos.y));
}

fn setup_ui(
    mut commands: Commands,
    windows: Query<Entity, With<PrimaryWindow>>,
    secondary_windows: Query<Entity, With<SecondaryWindow>>,
) {
    let Ok(primary_window) = windows.single() else {
        warn!("No primary window found");
        return;
    };
    let Ok(secondary_window) = secondary_windows.single() else {
        warn!("No secondary window found");
        return;
    };

    // Spawn camera for primary window
    let primary_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 0,
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(primary_window)),
        ))
        .id();

    // Spawn camera for secondary window
    let secondary_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(secondary_window)),
        ))
        .id();

    // Primary window UI
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            UiTargetCamera(primary_camera),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PRIMARY WINDOW - Waiting for input..."),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                PrimaryAuditDisplay,
            ));
        });

    // Secondary window UI
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            UiTargetCamera(secondary_camera),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SECONDARY WINDOW - Waiting for input..."),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                SecondaryAuditDisplay,
            ));
        });
}

fn minimize_window(
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut minimized: Local<bool>,
) {
    if *minimized {
        return;
    }

    for mut window in &mut windows {
        window.mode = WindowMode::Windowed;
        *minimized = true;
    }
}

fn track_mouse_buttons(
    mut button_events: MessageReader<MouseButtonInput>,
    mut tracker: ResMut<MouseStateTracker>,
    time: Res<Time>,
) {
    for event in button_events.read() {
        let pressed = event.state.is_pressed();
        let current_time = time.elapsed_secs();

        match event.button {
            MouseButton::Left => {
                tracker.left_pressed = pressed;
                tracker.left_timestamp = current_time;
                if !pressed {
                    tracker.left_duration = 0.0;
                }
            },
            MouseButton::Right => {
                tracker.right_pressed = pressed;
                tracker.right_timestamp = current_time;
                if !pressed {
                    tracker.right_duration = 0.0;
                }
            },
            MouseButton::Middle => {
                tracker.middle_pressed = pressed;
                tracker.middle_timestamp = current_time;
                if !pressed {
                    tracker.middle_duration = 0.0;
                }
            },
            MouseButton::Back => {
                tracker.back_pressed = pressed;
                tracker.back_timestamp = current_time;
                if !pressed {
                    tracker.back_duration = 0.0;
                }
            },
            MouseButton::Forward => {
                tracker.forward_pressed = pressed;
                tracker.forward_timestamp = current_time;
                if !pressed {
                    tracker.forward_duration = 0.0;
                }
            },
            MouseButton::Other(_) => {},
        }
    }
}

/// Detect clicks and double-clicks from button release events
fn track_click_events(
    mut button_events: MessageReader<MouseButtonInput>,
    mut tracker: ResMut<MouseStateTracker>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let Ok(primary_window) = primary_windows.single() else {
        return;
    };
    let current_time = time.elapsed_secs();

    // Track last click times per window for double-click detection
    let mut last_primary_click = tracker.primary_click_timestamp;
    let mut last_secondary_click = tracker.secondary_click_timestamp;

    for event in button_events.read() {
        // Only track left button clicks for simplicity
        if event.button == MouseButton::Left && !event.state.is_pressed() {
            let is_primary = event.window == primary_window;

            if is_primary {
                let click_position = tracker.primary_window_position;
                // Detect double-click (two clicks within 400ms)
                if current_time - last_primary_click < 0.4 {
                    tracker.primary_doubleclick_timestamp = current_time;
                    tracker.primary_doubleclick_position = click_position;
                }
                tracker.primary_click_timestamp = current_time;
                tracker.primary_click_position = click_position;
                last_primary_click = current_time;
            } else {
                let click_position = tracker.secondary_window_position;
                // Detect double-click (two clicks within 400ms)
                if current_time - last_secondary_click < 0.4 {
                    tracker.secondary_doubleclick_timestamp = current_time;
                    tracker.secondary_doubleclick_position = click_position;
                }
                tracker.secondary_click_timestamp = current_time;
                tracker.secondary_click_position = click_position;
                last_secondary_click = current_time;
            }
        }
    }
}

// Update button durations for pressed buttons
fn update_button_durations(mut tracker: ResMut<MouseStateTracker>, time: Res<Time>) {
    let current_time = time.elapsed_secs();

    if tracker.left_pressed {
        tracker.left_duration = current_time - tracker.left_timestamp;
    }
    if tracker.right_pressed {
        tracker.right_duration = current_time - tracker.right_timestamp;
    }
    if tracker.middle_pressed {
        tracker.middle_duration = current_time - tracker.middle_timestamp;
    }
    if tracker.back_pressed {
        tracker.back_duration = current_time - tracker.back_timestamp;
    }
    if tracker.forward_pressed {
        tracker.forward_duration = current_time - tracker.forward_timestamp;
    }
}

fn track_cursor_position(
    mut cursor_events: MessageReader<CursorMoved>,
    mut tracker: ResMut<MouseStateTracker>,
    primary_query: Query<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    for event in cursor_events.read() {
        let current_time = time.elapsed_secs();

        // Determine which window received the event and track cursor location
        if let Ok(primary_entity) = primary_query.single() {
            if event.window == primary_entity {
                tracker.primary_window_position = event.position;
                tracker.primary_cursor_timestamp = current_time;
            } else {
                tracker.secondary_window_position = event.position;
                tracker.secondary_cursor_timestamp = current_time;
            }
            tracker.cursor_window = Some(event.window);
        }
    }
}

fn track_mouse_wheel(
    mut wheel_events: MessageReader<MouseWheel>,
    mut tracker: ResMut<MouseStateTracker>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    secondary_windows: Query<Entity, With<SecondaryWindow>>,
    time: Res<Time>,
) {
    let Ok(primary_window) = primary_windows.single() else {
        return;
    };
    let Ok(secondary_window) = secondary_windows.single() else {
        return;
    };

    for event in wheel_events.read() {
        let current_time = time.elapsed_secs();

        if event.window == primary_window {
            tracker.primary_scroll_x_total += event.x;
            tracker.primary_scroll_y_total += event.y;
            tracker.primary_scroll_unit = format!("{:?}", event.unit);
            tracker.primary_scroll_timestamp = current_time;
        } else if event.window == secondary_window {
            tracker.secondary_scroll_x_total += event.x;
            tracker.secondary_scroll_y_total += event.y;
            tracker.secondary_scroll_unit = format!("{:?}", event.unit);
            tracker.secondary_scroll_timestamp = current_time;
        }
    }
}

fn track_mouse_motion(
    mut motion_events: MessageReader<MouseMotion>,
    mut tracker: ResMut<MouseStateTracker>,
    time: Res<Time>,
) {
    for event in motion_events.read() {
        tracker.motion_delta_total += event.delta;
        tracker.motion_timestamp = time.elapsed_secs();
    }
}

fn track_gestures(
    mut pinch_events: MessageReader<PinchGesture>,
    mut rotation_events: MessageReader<RotationGesture>,
    mut double_tap_events: MessageReader<DoubleTapGesture>,
    mut tracker: ResMut<MouseStateTracker>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs();

    // Get primary window entity to determine which window gestures apply to
    let Ok(primary_window) = primary_windows.single() else {
        return;
    };

    // Gestures don't have window fields, so use cursor_window to determine target
    let is_primary = tracker.cursor_window.is_none_or(|w| w == primary_window);

    for event in pinch_events.read() {
        if is_primary {
            tracker.primary_pinch_total += event.0;
            tracker.primary_pinch_timestamp = current_time;
        } else {
            tracker.secondary_pinch_total += event.0;
            tracker.secondary_pinch_timestamp = current_time;
        }
    }

    for event in rotation_events.read() {
        if is_primary {
            tracker.primary_rotation_total += event.0;
            tracker.primary_rotation_timestamp = current_time;
        } else {
            tracker.secondary_rotation_total += event.0;
            tracker.secondary_rotation_timestamp = current_time;
        }
    }

    for _event in double_tap_events.read() {
        if is_primary {
            tracker.primary_double_tap_timestamp = current_time;
        } else {
            tracker.secondary_double_tap_timestamp = current_time;
        }
    }
}

fn format_timestamp(current_time: f32, timestamp: f32) -> String {
    if timestamp > 0.0 && (current_time - timestamp) < 0.5 {
        "[NOW]".to_string()
    } else {
        String::new()
    }
}

fn format_button(current_time: f32, pressed: bool, timestamp: f32, duration: f32) -> String {
    if pressed {
        if duration > 0.0 {
            format!(
                "PRESSED {} [{duration:.1}s]",
                format_timestamp(current_time, timestamp)
            )
        } else {
            format!("PRESSED {}", format_timestamp(current_time, timestamp))
        }
    } else {
        format!("released {}", format_timestamp(current_time, timestamp))
    }
}

fn format_click(current_time: f32, timestamp: f32, position: Vec2) -> String {
    if timestamp > 0.0 && (current_time - timestamp) < 0.5 {
        format!("({:.1}, {:.1}) [NOW]", position.x, position.y)
    } else if timestamp > 0.0 {
        format!("({:.1}, {:.1})", position.x, position.y)
    } else {
        String::new()
    }
}

fn format_primary_display(tracker: &MouseStateTracker, current_time: f32) -> String {
    format!(
        "=== PRIMARY WINDOW ===\n\
        Cursor: ({:.1}, {:.1}) {}\n\n\
        CLICKS:\n\
        Click: {}          DoubleClick: {}\n\n\
        SCROLL:\n\
        X: {:.1}  Y: {:.1}  [{}] {}\n\n\
        GESTURES:\n\
        Pinch: {:.2} {}     Rotation: {:.2} {}     DoubleTap: {}\n\n\
        === SHARED STATE ===\n\
        BUTTONS:\n\
        Left:    {}      Middle: {}\n\
        Right:   {}      Back:   {}\n\
        Forward: {}",
        tracker.primary_window_position.x,
        tracker.primary_window_position.y,
        format_timestamp(current_time, tracker.primary_cursor_timestamp),
        format_click(
            current_time,
            tracker.primary_click_timestamp,
            tracker.primary_click_position
        ),
        format_click(
            current_time,
            tracker.primary_doubleclick_timestamp,
            tracker.primary_doubleclick_position
        ),
        tracker.primary_scroll_x_total,
        tracker.primary_scroll_y_total,
        tracker.primary_scroll_unit,
        format_timestamp(current_time, tracker.primary_scroll_timestamp),
        tracker.primary_pinch_total,
        format_timestamp(current_time, tracker.primary_pinch_timestamp),
        tracker.primary_rotation_total,
        format_timestamp(current_time, tracker.primary_rotation_timestamp),
        format_timestamp(current_time, tracker.primary_double_tap_timestamp),
        format_button(
            current_time,
            tracker.left_pressed,
            tracker.left_timestamp,
            tracker.left_duration
        ),
        format_button(
            current_time,
            tracker.middle_pressed,
            tracker.middle_timestamp,
            tracker.middle_duration
        ),
        format_button(
            current_time,
            tracker.right_pressed,
            tracker.right_timestamp,
            tracker.right_duration
        ),
        format_button(
            current_time,
            tracker.back_pressed,
            tracker.back_timestamp,
            tracker.back_duration
        ),
        format_button(
            current_time,
            tracker.forward_pressed,
            tracker.forward_timestamp,
            tracker.forward_duration
        ),
    )
}

fn format_secondary_display(tracker: &MouseStateTracker, current_time: f32) -> String {
    format!(
        "=== SECONDARY WINDOW ===\n\
        Cursor: ({:.1}, {:.1}) {}\n\n\
        CLICKS:\n\
        Click: {}          DoubleClick: {}\n\n\
        SCROLL:\n\
        X: {:.1}  Y: {:.1}  [{}] {}\n\n\
        GESTURES:\n\
        Pinch: {:.2} {}     Rotation: {:.2} {}     DoubleTap: {}",
        tracker.secondary_window_position.x,
        tracker.secondary_window_position.y,
        format_timestamp(current_time, tracker.secondary_cursor_timestamp),
        format_click(
            current_time,
            tracker.secondary_click_timestamp,
            tracker.secondary_click_position
        ),
        format_click(
            current_time,
            tracker.secondary_doubleclick_timestamp,
            tracker.secondary_doubleclick_position
        ),
        tracker.secondary_scroll_x_total,
        tracker.secondary_scroll_y_total,
        tracker.secondary_scroll_unit,
        format_timestamp(current_time, tracker.secondary_scroll_timestamp),
        tracker.secondary_pinch_total,
        format_timestamp(current_time, tracker.secondary_pinch_timestamp),
        tracker.secondary_rotation_total,
        format_timestamp(current_time, tracker.secondary_rotation_timestamp),
        format_timestamp(current_time, tracker.secondary_double_tap_timestamp),
    )
}

fn update_audit_display(
    tracker: Res<MouseStateTracker>,
    mut primary_text: Query<&mut Text, (With<PrimaryAuditDisplay>, Without<SecondaryAuditDisplay>)>,
    mut secondary_text: Query<
        &mut Text,
        (With<SecondaryAuditDisplay>, Without<PrimaryAuditDisplay>),
    >,
    time: Res<Time>,
) {
    if !tracker.is_changed() {
        return;
    }

    let current_time = time.elapsed_secs();

    // Update primary window display
    if let Ok(mut text) = primary_text.single_mut() {
        **text = format_primary_display(&tracker, current_time);
    }

    // Update secondary window display
    if let Ok(mut text) = secondary_text.single_mut() {
        **text = format_secondary_display(&tracker, current_time);
    }
}
