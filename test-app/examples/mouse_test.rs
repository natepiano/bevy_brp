//! Mouse input test application for BRP extras
//!
//! This example demonstrates and tests all mouse input functionality provided by `bevy_brp_extras`.
//! It creates two windows (primary and secondary) to test window-specific targeting.
//!
//! The `MouseStateTracker` resource tracks all mouse interactions and can be queried via BRP
//! to verify correct behavior during integration tests.
//!
//! Each window also contains a pickable 3D cuboid to verify that simulated input events
//! correctly flow through Bevy's picking system via the `WindowEvent` channel.

use bevy::camera::RenderTarget;
use bevy::color::palettes::css;
use bevy::input::gestures::DoubleTapGesture;
use bevy::input::gestures::PinchGesture;
use bevy::input::gestures::RotationGesture;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::mouse::MouseMotion;
use bevy::input::mouse::MouseWheel;
use bevy::math::primitives::Cuboid;
use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy::ui::UiTargetCamera;
use bevy::window::CursorMoved;
use bevy::window::PrimaryWindow;
use bevy::window::WindowMode;
use bevy::window::WindowRef;
use bevy::window::WindowResolution;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

// Mouse test constants
/// Size of the pickable cuboid (used for both mesh and gizmo outline)
const CUBOID_SIZE: f32 = 1.5;

/// Double-click detection threshold in seconds
const DOUBLE_CLICK_THRESHOLD: f32 = 0.4;

/// X offset for the secondary window scene (far enough that cameras don't see each other's cuboids)
const SECONDARY_SCENE_OFFSET: f32 = 100.0;

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
            MeshPickingPlugin,
            BrpExtrasPlugin::new().port_in_title(PortDisplay::Always),
        ))
        .init_resource::<MouseStateTracker>()
        .add_systems(Startup, (setup_windows, setup_scene).chain())
        .add_systems(PostStartup, position_secondary_window)
        .add_systems(Update, minimize_window)
        .add_systems(
            Update,
            (
                track_cursor_position,
                track_mouse_buttons,
                track_click_events,
                update_button_durations,
                track_mouse_wheel,
                track_mouse_motion,
                track_gestures,
                draw_gizmo_outlines,
                update_audit_display,
            ),
        )
        .run();
}

/// Resource tracking all mouse input state for testing purposes
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct MouseStateTracker {
    buttons:       ButtonStates,
    clicks:        WindowClicks,
    cursor:        WindowCursorState,
    cursor_window: Option<Entity>,
    gestures:      WindowGestures,
    motion:        MotionState,
    picking:       WindowPickingState,
    scroll:        WindowScrollState,
}

#[derive(Clone, Copy)]
enum WindowSlot {
    Primary,
    Secondary,
}

#[derive(Default, Reflect)]
struct ButtonState {
    duration:  f32,
    pressed:   bool,
    timestamp: f32,
}

impl ButtonState {
    const fn set_pressed(&mut self, pressed: bool, current_time: f32) {
        self.pressed = pressed;
        self.timestamp = current_time;
        if !pressed {
            self.duration = 0.0;
        }
    }

    fn update_duration(&mut self, current_time: f32) {
        if self.pressed {
            self.duration = current_time - self.timestamp;
        }
    }
}

#[derive(Default, Reflect)]
struct ButtonStates {
    back:    ButtonState,
    forward: ButtonState,
    left:    ButtonState,
    middle:  ButtonState,
    right:   ButtonState,
}

#[derive(Default, Reflect)]
struct ClickState {
    double_position:  Vec2,
    double_timestamp: f32,
    position:         Vec2,
    timestamp:        f32,
}

#[derive(Default, Reflect)]
struct CursorState {
    position:  Vec2,
    timestamp: f32,
}

#[derive(Default, Reflect)]
struct GestureState {
    double_tap_timestamp: f32,
    pinch_timestamp:      f32,
    pinch_total:          f32,
    rotation_timestamp:   f32,
    rotation_total:       f32,
}

#[derive(Default, Reflect)]
struct MotionState {
    delta_total: Vec2,
    timestamp:   f32,
}

#[derive(Default, Reflect)]
struct PickingState {
    clicks:          u32,
    double_clicks:   u32,
    gizmo_active:    bool,
    last_click_time: f32,
}

#[derive(Default, Reflect)]
struct ScrollState {
    timestamp: f32,
    unit:      String,
    x_total:   f32,
    y_total:   f32,
}

#[derive(Default, Reflect)]
struct WindowClicks {
    primary:   ClickState,
    secondary: ClickState,
}

#[derive(Default, Reflect)]
struct WindowCursorState {
    primary:   CursorState,
    secondary: CursorState,
}

#[derive(Default, Reflect)]
struct WindowGestures {
    primary:   GestureState,
    secondary: GestureState,
}

#[derive(Default, Reflect)]
struct WindowPickingState {
    primary:   PickingState,
    secondary: PickingState,
}

#[derive(Default, Reflect)]
struct WindowScrollState {
    primary:   ScrollState,
    secondary: ScrollState,
}

impl MouseStateTracker {
    const fn button_mut(&mut self, button: MouseButton) -> Option<&mut ButtonState> {
        match button {
            MouseButton::Back => Some(&mut self.buttons.back),
            MouseButton::Forward => Some(&mut self.buttons.forward),
            MouseButton::Left => Some(&mut self.buttons.left),
            MouseButton::Middle => Some(&mut self.buttons.middle),
            MouseButton::Right => Some(&mut self.buttons.right),
            MouseButton::Other(_) => None,
        }
    }

    const fn clicks(&self, slot: WindowSlot) -> &ClickState {
        match slot {
            WindowSlot::Primary => &self.clicks.primary,
            WindowSlot::Secondary => &self.clicks.secondary,
        }
    }

    const fn clicks_mut(&mut self, slot: WindowSlot) -> &mut ClickState {
        match slot {
            WindowSlot::Primary => &mut self.clicks.primary,
            WindowSlot::Secondary => &mut self.clicks.secondary,
        }
    }

    const fn cursor(&self, slot: WindowSlot) -> &CursorState {
        match slot {
            WindowSlot::Primary => &self.cursor.primary,
            WindowSlot::Secondary => &self.cursor.secondary,
        }
    }

    const fn cursor_mut(&mut self, slot: WindowSlot) -> &mut CursorState {
        match slot {
            WindowSlot::Primary => &mut self.cursor.primary,
            WindowSlot::Secondary => &mut self.cursor.secondary,
        }
    }

    const fn gestures(&self, slot: WindowSlot) -> &GestureState {
        match slot {
            WindowSlot::Primary => &self.gestures.primary,
            WindowSlot::Secondary => &self.gestures.secondary,
        }
    }

    const fn gestures_mut(&mut self, slot: WindowSlot) -> &mut GestureState {
        match slot {
            WindowSlot::Primary => &mut self.gestures.primary,
            WindowSlot::Secondary => &mut self.gestures.secondary,
        }
    }

    const fn picking(&self, slot: WindowSlot) -> &PickingState {
        match slot {
            WindowSlot::Primary => &self.picking.primary,
            WindowSlot::Secondary => &self.picking.secondary,
        }
    }

    const fn picking_mut(&mut self, slot: WindowSlot) -> &mut PickingState {
        match slot {
            WindowSlot::Primary => &mut self.picking.primary,
            WindowSlot::Secondary => &mut self.picking.secondary,
        }
    }

    const fn scroll(&self, slot: WindowSlot) -> &ScrollState {
        match slot {
            WindowSlot::Primary => &self.scroll.primary,
            WindowSlot::Secondary => &self.scroll.secondary,
        }
    }

    const fn scroll_mut(&mut self, slot: WindowSlot) -> &mut ScrollState {
        match slot {
            WindowSlot::Primary => &mut self.scroll.primary,
            WindowSlot::Secondary => &mut self.scroll.secondary,
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

/// Marker for primary window cuboid
#[derive(Component)]
struct PrimaryCuboid;

/// Marker for secondary window cuboid
#[derive(Component)]
struct SecondaryCuboid;

/// Marker for primary window background (catches click-off events)
#[derive(Component)]
struct PrimaryBackground;

/// Marker for secondary window background
#[derive(Component)]
struct SecondaryBackground;

/// Attached to a cuboid when selected — drives gizmo outline rendering
#[derive(Component)]
struct GizmoOutline {
    color: Color,
}

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

#[allow(
    clippy::too_many_lines,
    reason = "test scene setup with many UI elements"
)]
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    // Shared mesh and material assets
    let cuboid = Cuboid::new(CUBOID_SIZE, CUBOID_SIZE, CUBOID_SIZE);
    let cuboid_mesh = meshes.add(cuboid);
    let cuboid_material = materials.add(StandardMaterial {
        base_color: Color::from(css::CORNFLOWER_BLUE),
        ..default()
    });
    let background_mesh = meshes.add(Cuboid::new(20.0, 20.0, 0.1));
    let background_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.2),
        ..default()
    });

    // === Primary window scene (at origin) ===

    let primary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 0,
                ..default()
            },
            Transform::from_xyz(0.0, 1.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            RenderTarget::Window(WindowRef::Entity(primary_window)),
        ))
        .id();

    // Primary cuboid
    commands
        .spawn((
            Mesh3d(cuboid_mesh.clone()),
            MeshMaterial3d(cuboid_material.clone()),
            Transform::from_xyz(0.0, 0.0, 0.0),
            PrimaryCuboid,
            Pickable::default(),
        ))
        .observe(on_primary_cuboid_click);

    // Primary background (behind cuboid, catches click-off)
    commands
        .spawn((
            Mesh3d(background_mesh.clone()),
            MeshMaterial3d(background_material.clone()),
            Transform::from_xyz(0.0, 0.0, -3.0),
            PrimaryBackground,
            Pickable::default(),
        ))
        .observe(on_primary_background_click);

    // Primary light
    commands.spawn((
        DirectionalLight {
            illuminance: 2000.0,
            ..default()
        },
        Transform::from_xyz(2.0, 4.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // === Secondary window scene (offset by SECONDARY_SCENE_OFFSET on X) ===

    let secondary_camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: 1,
                ..default()
            },
            Transform::from_xyz(SECONDARY_SCENE_OFFSET, 1.5, 5.0)
                .looking_at(Vec3::new(SECONDARY_SCENE_OFFSET, 0.0, 0.0), Vec3::Y),
            RenderTarget::Window(WindowRef::Entity(secondary_window)),
        ))
        .id();

    // Secondary cuboid
    commands
        .spawn((
            Mesh3d(cuboid_mesh),
            MeshMaterial3d(cuboid_material),
            Transform::from_xyz(SECONDARY_SCENE_OFFSET, 0.0, 0.0),
            SecondaryCuboid,
            Pickable::default(),
        ))
        .observe(on_secondary_cuboid_click);

    // Secondary background
    commands
        .spawn((
            Mesh3d(background_mesh),
            MeshMaterial3d(background_material),
            Transform::from_xyz(SECONDARY_SCENE_OFFSET, 0.0, -3.0),
            SecondaryBackground,
            Pickable::default(),
        ))
        .observe(on_secondary_background_click);

    // Secondary light
    commands.spawn((
        DirectionalLight {
            illuminance: 2000.0,
            ..default()
        },
        Transform::from_xyz(SECONDARY_SCENE_OFFSET + 2.0, 4.0, 3.0)
            .looking_at(Vec3::new(SECONDARY_SCENE_OFFSET, 0.0, 0.0), Vec3::Y),
    ));

    // === UI overlays (text displays) ===

    // Primary window UI
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            Pickable::IGNORE,
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
                Pickable::IGNORE,
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
            Pickable::IGNORE,
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
                Pickable::IGNORE,
                SecondaryAuditDisplay,
            ));
        });
}

// ============================================================================
// Picking observers
// ============================================================================

fn on_primary_cuboid_click(
    trigger: On<Pointer<Click>>,
    mut tracker: ResMut<MouseStateTracker>,
    mut commands: Commands,
    cuboids: Query<Entity, With<PrimaryCuboid>>,
    time: Res<Time>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    let current = time.elapsed_secs();
    let Ok(cuboid_entity) = cuboids.single() else {
        return;
    };

    let picking = tracker.picking_mut(WindowSlot::Primary);
    if current - picking.last_click_time < DOUBLE_CLICK_THRESHOLD {
        picking.double_clicks += 1;
        commands.entity(cuboid_entity).insert(GizmoOutline {
            color: Color::from(css::YELLOW),
        });
    } else {
        picking.clicks += 1;
        commands.entity(cuboid_entity).insert(GizmoOutline {
            color: Color::from(css::LIME),
        });
    }
    picking.gizmo_active = true;
    picking.last_click_time = current;
}

fn on_secondary_cuboid_click(
    trigger: On<Pointer<Click>>,
    mut tracker: ResMut<MouseStateTracker>,
    mut commands: Commands,
    cuboids: Query<Entity, With<SecondaryCuboid>>,
    time: Res<Time>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    let current = time.elapsed_secs();
    let Ok(cuboid_entity) = cuboids.single() else {
        return;
    };

    let picking = tracker.picking_mut(WindowSlot::Secondary);
    if current - picking.last_click_time < DOUBLE_CLICK_THRESHOLD {
        picking.double_clicks += 1;
        commands.entity(cuboid_entity).insert(GizmoOutline {
            color: Color::from(css::YELLOW),
        });
    } else {
        picking.clicks += 1;
        commands.entity(cuboid_entity).insert(GizmoOutline {
            color: Color::from(css::LIME),
        });
    }
    picking.gizmo_active = true;
    picking.last_click_time = current;
}

fn on_primary_background_click(
    trigger: On<Pointer<Click>>,
    mut tracker: ResMut<MouseStateTracker>,
    mut commands: Commands,
    cuboids: Query<Entity, With<PrimaryCuboid>>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    tracker.picking_mut(WindowSlot::Primary).gizmo_active = false;
    if let Ok(cuboid_entity) = cuboids.single() {
        commands.entity(cuboid_entity).remove::<GizmoOutline>();
    }
}

fn on_secondary_background_click(
    trigger: On<Pointer<Click>>,
    mut tracker: ResMut<MouseStateTracker>,
    mut commands: Commands,
    cuboids: Query<Entity, With<SecondaryCuboid>>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    tracker.picking_mut(WindowSlot::Secondary).gizmo_active = false;
    if let Ok(cuboid_entity) = cuboids.single() {
        commands.entity(cuboid_entity).remove::<GizmoOutline>();
    }
}

// ============================================================================
// Gizmo rendering
// ============================================================================

fn draw_gizmo_outlines(mut gizmos: Gizmos, query: Query<(&Transform, &GizmoOutline)>) {
    let cuboid = Cuboid::new(CUBOID_SIZE, CUBOID_SIZE, CUBOID_SIZE);
    for (transform, outline) in &query {
        gizmos.primitive_3d(&cuboid, transform.to_isometry(), outline.color);
    }
}

// ============================================================================
// Input tracking systems (unchanged from original)
// ============================================================================

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
        let current_time = time.elapsed_secs();

        if let Some(button) = tracker.button_mut(event.button) {
            button.set_pressed(event.state.is_pressed(), current_time);
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

    for event in button_events.read() {
        if event.button != MouseButton::Left || event.state.is_pressed() {
            continue;
        }

        let slot = if event.window == primary_window {
            WindowSlot::Primary
        } else {
            WindowSlot::Secondary
        };
        let click_position = tracker.cursor(slot).position;
        let click = tracker.clicks_mut(slot);

        if current_time - click.timestamp < DOUBLE_CLICK_THRESHOLD {
            click.double_timestamp = current_time;
            click.double_position = click_position;
        }

        click.timestamp = current_time;
        click.position = click_position;
    }
}

// Update button durations for pressed buttons
fn update_button_durations(mut tracker: ResMut<MouseStateTracker>, time: Res<Time>) {
    let current_time = time.elapsed_secs();

    tracker.buttons.back.update_duration(current_time);
    tracker.buttons.forward.update_duration(current_time);
    tracker.buttons.left.update_duration(current_time);
    tracker.buttons.middle.update_duration(current_time);
    tracker.buttons.right.update_duration(current_time);
}

fn track_cursor_position(
    mut cursor_events: MessageReader<CursorMoved>,
    mut tracker: ResMut<MouseStateTracker>,
    primary_query: Query<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    for event in cursor_events.read() {
        let current_time = time.elapsed_secs();

        if let Ok(primary_entity) = primary_query.single() {
            let slot = if event.window == primary_entity {
                WindowSlot::Primary
            } else {
                WindowSlot::Secondary
            };
            let cursor = tracker.cursor_mut(slot);
            cursor.position = event.position;
            cursor.timestamp = current_time;
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
        let slot = if event.window == primary_window {
            Some(WindowSlot::Primary)
        } else if event.window == secondary_window {
            Some(WindowSlot::Secondary)
        } else {
            None
        };

        if let Some(slot) = slot {
            let scroll = tracker.scroll_mut(slot);
            scroll.x_total += event.x;
            scroll.y_total += event.y;
            scroll.unit = format!("{:?}", event.unit);
            scroll.timestamp = current_time;
        }
    }
}

fn track_mouse_motion(
    mut motion_events: MessageReader<MouseMotion>,
    mut tracker: ResMut<MouseStateTracker>,
    time: Res<Time>,
) {
    for event in motion_events.read() {
        tracker.motion.delta_total += event.delta;
        tracker.motion.timestamp = time.elapsed_secs();
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

    let Ok(primary_window) = primary_windows.single() else {
        return;
    };

    let slot = if tracker
        .cursor_window
        .is_none_or(|window| window == primary_window)
    {
        WindowSlot::Primary
    } else {
        WindowSlot::Secondary
    };

    for event in pinch_events.read() {
        let gestures = tracker.gestures_mut(slot);
        gestures.pinch_total += event.0;
        gestures.pinch_timestamp = current_time;
    }

    for event in rotation_events.read() {
        let gestures = tracker.gestures_mut(slot);
        gestures.rotation_total += event.0;
        gestures.rotation_timestamp = current_time;
    }

    for _ in double_tap_events.read() {
        tracker.gestures_mut(slot).double_tap_timestamp = current_time;
    }
}

// ============================================================================
// Display formatting
// ============================================================================

fn format_timestamp(current_time: f32, timestamp: f32) -> String {
    if timestamp > 0.0 && (current_time - timestamp) < 0.5 {
        "[NOW]".to_string()
    } else {
        String::new()
    }
}

fn format_button(current_time: f32, button: &ButtonState) -> String {
    if button.pressed {
        if button.duration > 0.0 {
            format!(
                "PRESSED {} [{:.1}s]",
                format_timestamp(current_time, button.timestamp),
                button.duration
            )
        } else {
            format!(
                "PRESSED {}",
                format_timestamp(current_time, button.timestamp)
            )
        }
    } else {
        format!(
            "released {}",
            format_timestamp(current_time, button.timestamp)
        )
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

fn format_picking(picking: &PickingState) -> String {
    format!(
        "Clicks: {}  DblClk: {}  Gizmo: {}",
        picking.clicks,
        picking.double_clicks,
        if picking.gizmo_active { "ON" } else { "off" }
    )
}

fn format_primary_display(tracker: &MouseStateTracker, current_time: f32) -> String {
    let primary_cursor = tracker.cursor(WindowSlot::Primary);
    let primary_clicks = tracker.clicks(WindowSlot::Primary);
    let primary_picking = tracker.picking(WindowSlot::Primary);
    let primary_scroll = tracker.scroll(WindowSlot::Primary);
    let primary_gestures = tracker.gestures(WindowSlot::Primary);

    format!(
        "=== PRIMARY WINDOW ===\n\
        Cursor: ({:.1}, {:.1}) {}\n\n\
        CLICKS:\n\
        Click: {}          DoubleClick: {}\n\n\
        PICKING:\n\
        {}\n\n\
        SCROLL:\n\
        X: {:.1}  Y: {:.1}  [{}] {}\n\n\
        GESTURES:\n\
        Pinch: {:.2} {}     Rotation: {:.2} {}     DoubleTap: {}\n\n\
        === SHARED STATE ===\n\
        BUTTONS:\n\
        Left:    {}      Middle: {}\n\
        Right:   {}      Back:   {}\n\
        Forward: {}",
        primary_cursor.position.x,
        primary_cursor.position.y,
        format_timestamp(current_time, primary_cursor.timestamp),
        format_click(
            current_time,
            primary_clicks.timestamp,
            primary_clicks.position
        ),
        format_click(
            current_time,
            primary_clicks.double_timestamp,
            primary_clicks.double_position
        ),
        format_picking(primary_picking),
        primary_scroll.x_total,
        primary_scroll.y_total,
        primary_scroll.unit,
        format_timestamp(current_time, primary_scroll.timestamp),
        primary_gestures.pinch_total,
        format_timestamp(current_time, primary_gestures.pinch_timestamp),
        primary_gestures.rotation_total,
        format_timestamp(current_time, primary_gestures.rotation_timestamp),
        format_timestamp(current_time, primary_gestures.double_tap_timestamp),
        format_button(current_time, &tracker.buttons.left),
        format_button(current_time, &tracker.buttons.middle),
        format_button(current_time, &tracker.buttons.right),
        format_button(current_time, &tracker.buttons.back),
        format_button(current_time, &tracker.buttons.forward),
    )
}

fn format_secondary_display(tracker: &MouseStateTracker, current_time: f32) -> String {
    let secondary_cursor = tracker.cursor(WindowSlot::Secondary);
    let secondary_clicks = tracker.clicks(WindowSlot::Secondary);
    let secondary_picking = tracker.picking(WindowSlot::Secondary);
    let secondary_scroll = tracker.scroll(WindowSlot::Secondary);
    let secondary_gestures = tracker.gestures(WindowSlot::Secondary);

    format!(
        "=== SECONDARY WINDOW ===\n\
        Cursor: ({:.1}, {:.1}) {}\n\n\
        CLICKS:\n\
        Click: {}          DoubleClick: {}\n\n\
        PICKING:\n\
        {}\n\n\
        SCROLL:\n\
        X: {:.1}  Y: {:.1}  [{}] {}\n\n\
        GESTURES:\n\
        Pinch: {:.2} {}     Rotation: {:.2} {}     DoubleTap: {}",
        secondary_cursor.position.x,
        secondary_cursor.position.y,
        format_timestamp(current_time, secondary_cursor.timestamp),
        format_click(
            current_time,
            secondary_clicks.timestamp,
            secondary_clicks.position
        ),
        format_click(
            current_time,
            secondary_clicks.double_timestamp,
            secondary_clicks.double_position
        ),
        format_picking(secondary_picking),
        secondary_scroll.x_total,
        secondary_scroll.y_total,
        secondary_scroll.unit,
        format_timestamp(current_time, secondary_scroll.timestamp),
        secondary_gestures.pinch_total,
        format_timestamp(current_time, secondary_gestures.pinch_timestamp),
        secondary_gestures.rotation_total,
        format_timestamp(current_time, secondary_gestures.rotation_timestamp),
        format_timestamp(current_time, secondary_gestures.double_tap_timestamp),
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
