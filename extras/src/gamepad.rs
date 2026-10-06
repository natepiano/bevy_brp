//! Simulated gamepads for Bevy Remote Protocol
//!
//! Bevy's gamepad layer is fed by messages: `bevy_gilrs` writes a [`GamepadConnectionEvent`]
//! and [`RawGamepadEvent`]s, and `gamepad_event_processing_system` turns them into [`Gamepad`]
//! state. Writing the same messages for an entity of our own gives a pad that the app treats
//! as real: its input managers, menus and join screens all see it. No gilrs, no hardware.

use std::time::Duration;

use bevy::ecs::message::Message;
use bevy::input::InputSystems;
use bevy::input::gamepad::GamepadConnection;
use bevy::input::gamepad::GamepadConnectionEvent;
use bevy::input::gamepad::RawGamepadAxisChangedEvent;
use bevy::input::gamepad::RawGamepadButtonChangedEvent;
use bevy::input::gamepad::RawGamepadEvent;
use bevy::prelude::*;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::RemoteMethodSystemId;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_request;
use crate::brp_request::EmptyParamsPolicy;
use crate::constants::EXTRAS_COMMAND_PREFIX;
use crate::constants::METHOD_CONNECT_GAMEPAD;
use crate::constants::METHOD_DISCONNECT_GAMEPAD;
use crate::constants::METHOD_SEND_GAMEPAD_BUTTON;
use crate::constants::METHOD_SET_GAMEPAD_AXIS;
use crate::constants::METHOD_SET_GAMEPAD_BUTTON;

/// Default hold for a timed button press
const DEFAULT_GAMEPAD_DURATION_MS: u32 = 100;

/// Maximum hold for a timed button press
const MAX_GAMEPAD_DURATION_MS: u32 = 60_000;

/// Name given to every simulated gamepad, as the OS name of a real one
const SIMULATED_GAMEPAD_NAME: &str = "Simulated gamepad (BRP)";

// ============================================================================
// Types
// ============================================================================

/// Marks a gamepad entity spawned by `brp_extras/connect_gamepad`
///
/// The other gamepad methods only accept entities carrying it: injecting into a real pad
/// would fight its driver for the pad's state.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct SimulatedGamepad;

/// Request structure for `connect_gamepad`
#[derive(Deserialize)]
struct ConnectGamepadRequest {
    /// Name reported for the pad (default: "Simulated gamepad (BRP)")
    #[serde(default)]
    name: Option<String>,
}

/// Request structure for `send_gamepad_button`
#[derive(Deserialize)]
struct SendGamepadButtonRequest {
    /// Simulated gamepad entity
    gamepad:     u64,
    /// Button to tap
    button:      GamepadButton,
    /// Release after this many milliseconds (default: 100, max: 60000)
    #[serde(default)]
    duration_ms: Option<u32>,
}

/// Request structure for `set_gamepad_button`
#[derive(Deserialize)]
struct SetGamepadButtonRequest {
    /// Simulated gamepad entity
    gamepad: u64,
    /// Button to set
    button:  GamepadButton,
    /// Analog value in `[0.0, 1.0]`
    value:   f32,
}

/// Request structure for `set_gamepad_axis`
#[derive(Deserialize)]
struct SetGamepadAxisRequest {
    /// Simulated gamepad entity
    gamepad: u64,
    /// Axis to set
    axis:    GamepadAxis,
    /// Value in `[-1.0, 1.0]`
    value:   f32,
}

/// Request structure for `disconnect_gamepad`
#[derive(Deserialize)]
struct DisconnectGamepadRequest {
    /// Simulated gamepad entity
    gamepad: u64,
}

/// Response structure for `connect_gamepad` and `disconnect_gamepad`
#[derive(Serialize)]
struct GamepadResponse {
    /// The gamepad entity
    gamepad: u64,
}

/// Response structure for `send_gamepad_button`
#[derive(Serialize)]
struct SendGamepadButtonResponse {
    /// The gamepad entity
    gamepad:     u64,
    /// Button that was tapped
    button:      GamepadButton,
    /// Hold before the release
    duration_ms: u32,
}

/// Response structure for `set_gamepad_button`
#[derive(Serialize)]
struct SetGamepadButtonResponse {
    /// The gamepad entity
    gamepad: u64,
    /// Button that was set
    button:  GamepadButton,
    /// Value it was set to
    value:   f32,
}

/// Response structure for `set_gamepad_axis`
#[derive(Serialize)]
struct SetGamepadAxisResponse {
    /// The gamepad entity
    gamepad: u64,
    /// Axis that was set
    axis:    GamepadAxis,
    /// Value it was set to
    value:   f32,
}

/// Component for timed gamepad button releases
#[derive(Component)]
struct TimedGamepadButtonRelease {
    /// Gamepad the button belongs to
    gamepad: Entity,
    /// Button to release
    button:  GamepadButton,
    /// Timer tracking remaining duration
    timer:   Timer,
}

// ============================================================================
// Plugin
// ============================================================================

pub(super) struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            process_timed_gamepad_button_releases.before(InputSystems),
        );
    }
}

/// The gamepad BRP methods, for `register_extras_methods`
pub(crate) fn remote_methods(world: &mut World) -> [(String, RemoteMethodSystemId); 5] {
    [
        (
            format!("{EXTRAS_COMMAND_PREFIX}{METHOD_CONNECT_GAMEPAD}"),
            RemoteMethodSystemId::Instant(world.register_system(connect_gamepad_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}{METHOD_DISCONNECT_GAMEPAD}"),
            RemoteMethodSystemId::Instant(world.register_system(disconnect_gamepad_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}{METHOD_SEND_GAMEPAD_BUTTON}"),
            RemoteMethodSystemId::Instant(world.register_system(send_gamepad_button_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}{METHOD_SET_GAMEPAD_BUTTON}"),
            RemoteMethodSystemId::Instant(world.register_system(set_gamepad_button_handler)),
        ),
        (
            format!("{EXTRAS_COMMAND_PREFIX}{METHOD_SET_GAMEPAD_AXIS}"),
            RemoteMethodSystemId::Instant(world.register_system(set_gamepad_axis_handler)),
        ),
    ]
}

// ============================================================================
// Handlers
// ============================================================================

/// Handler for `connect_gamepad` BRP method
///
/// Spawns a simulated gamepad and announces it the way `bevy_gilrs` announces a real one.
/// It is a [`Gamepad`] from the next `PreUpdate` on.
fn connect_gamepad_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: ConnectGamepadRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Allow)?;
    let name = request
        .name
        .unwrap_or_else(|| SIMULATED_GAMEPAD_NAME.to_string());

    let gamepad = world
        .spawn((SimulatedGamepad, Name::new(name.clone())))
        .id();
    let event = GamepadConnectionEvent::new(
        gamepad,
        GamepadConnection::Connected {
            name,
            vendor_id: None,
            product_id: None,
        },
    );
    write_raw_gamepad_event(world, event);

    brp_request::serialize_response(
        GamepadResponse {
            gamepad: gamepad.to_bits(),
        },
        METHOD_CONNECT_GAMEPAD,
    )
}

/// Handler for `send_gamepad_button` BRP method
///
/// Taps a button for `duration_ms` on the real clock, defaulting to 100 ms.
fn send_gamepad_button_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: SendGamepadButtonRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;
    let duration_ms = request.duration_ms.unwrap_or(DEFAULT_GAMEPAD_DURATION_MS);
    if duration_ms > MAX_GAMEPAD_DURATION_MS {
        return Err(brp_request::invalid_params(format!(
            "Duration exceeds maximum: {duration_ms}ms > {MAX_GAMEPAD_DURATION_MS}ms"
        )));
    }

    cancel_pending_button_releases(world, gamepad, ReleaseCancellation::Button(request.button));
    world.spawn(TimedGamepadButtonRelease {
        gamepad,
        button: request.button,
        timer: Timer::new(
            Duration::from_millis(u64::from(duration_ms)),
            TimerMode::Once,
        ),
    });

    write_raw_gamepad_event(
        world,
        RawGamepadButtonChangedEvent::new(gamepad, request.button, 1.0),
    );

    brp_request::serialize_response(
        SendGamepadButtonResponse {
            gamepad: request.gamepad,
            button: request.button,
            duration_ms,
        },
        METHOD_SEND_GAMEPAD_BUTTON,
    )
}

/// Handler for `set_gamepad_button` BRP method
///
/// Sets a button's analog value until another call changes it.
fn set_gamepad_button_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: SetGamepadButtonRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;
    if !(0.0..=1.0).contains(&request.value) {
        return Err(brp_request::invalid_params(format!(
            "Button value {} is outside [0.0, 1.0]",
            request.value
        )));
    }

    cancel_pending_button_releases(world, gamepad, ReleaseCancellation::Button(request.button));
    write_raw_gamepad_event(
        world,
        RawGamepadButtonChangedEvent::new(gamepad, request.button, request.value),
    );

    brp_request::serialize_response(
        SetGamepadButtonResponse {
            gamepad: request.gamepad,
            button:  request.button,
            value:   request.value,
        },
        METHOD_SET_GAMEPAD_BUTTON,
    )
}

/// Handler for `set_gamepad_axis` BRP method
///
/// Sets an axis value. Axes stay where they are put.
fn set_gamepad_axis_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: SetGamepadAxisRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;
    if !(-1.0..=1.0).contains(&request.value) {
        return Err(brp_request::invalid_params(format!(
            "Axis value {} is outside [-1.0, 1.0]",
            request.value
        )));
    }

    write_raw_gamepad_event(
        world,
        RawGamepadAxisChangedEvent::new(gamepad, request.axis, request.value),
    );

    brp_request::serialize_response(
        SetGamepadAxisResponse {
            gamepad: request.gamepad,
            axis:    request.axis,
            value:   request.value,
        },
        METHOD_SET_GAMEPAD_AXIS,
    )
}

/// Handler for `disconnect_gamepad` BRP method
///
/// Bevy removes the [`Gamepad`] component and leaves the entity, as it does for a real pad.
fn disconnect_gamepad_handler(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: DisconnectGamepadRequest =
        brp_request::parse_request(params, EmptyParamsPolicy::Reject)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;

    cancel_pending_button_releases(world, gamepad, ReleaseCancellation::AllButtons);

    world.entity_mut(gamepad).remove::<SimulatedGamepad>();
    let event = GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected);
    write_raw_gamepad_event(world, event);

    brp_request::serialize_response(
        GamepadResponse {
            gamepad: request.gamepad,
        },
        METHOD_DISCONNECT_GAMEPAD,
    )
}

// ============================================================================
// Systems
// ============================================================================

/// System that processes timed gamepad button releases
///
/// The hold runs on `Time<Real>`, like the keyboard and mouse holds, so a paused virtual
/// clock cannot leave the button pressed.
///
/// A release is only written once the [`Gamepad`] has taken the press. A release written in
/// the same frame as its press would be processed in the same run, and nothing downstream
/// would ever see the button down, however long the hold was.
fn process_timed_gamepad_button_releases(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut query: Query<(Entity, &mut TimedGamepadButtonRelease)>,
    gamepads: Query<Option<&Gamepad>, With<SimulatedGamepad>>,
    mut raw_events: MessageWriter<RawGamepadEvent>,
    mut button_events: MessageWriter<RawGamepadButtonChangedEvent>,
) {
    for (entity, mut release) in &mut query {
        release.timer.tick(time.delta());

        let gamepad = match gamepads.get(release.gamepad) {
            Err(_) => {
                commands.entity(entity).despawn();
                continue;
            },
            Ok(None) => continue,
            Ok(Some(gamepad)) => gamepad,
        };
        if release.timer.is_finished() && gamepad.pressed(release.button) {
            let event = RawGamepadButtonChangedEvent::new(release.gamepad, release.button, 0.0);
            raw_events.write(RawGamepadEvent::from(event));
            button_events.write(event);
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn write_raw_gamepad_event<T>(world: &mut World, event: T)
where
    T: Clone + Message,
    RawGamepadEvent: From<T>,
{
    world.write_message(RawGamepadEvent::from(event.clone()));
    world.write_message(event);
}

enum ReleaseCancellation {
    AllButtons,
    Button(GamepadButton),
}

fn cancel_pending_button_releases(
    world: &mut World,
    gamepad: Entity,
    cancellation: ReleaseCancellation,
) {
    let pending: Vec<Entity> = world
        .query::<(Entity, &TimedGamepadButtonRelease)>()
        .iter(world)
        .filter(|(_, release)| {
            release.gamepad == gamepad
                && match cancellation {
                    ReleaseCancellation::AllButtons => true,
                    ReleaseCancellation::Button(button) => release.button == button,
                }
        })
        .map(|(entity, _)| entity)
        .collect();
    for entity in pending {
        world.despawn(entity);
    }
}

/// Resolve the `gamepad` parameter to a simulated gamepad entity
fn simulated_gamepad(world: &World, bits: u64) -> Result<Entity, BrpError> {
    Entity::try_from_bits(bits)
        .filter(|&entity| world.get::<SimulatedGamepad>(entity).is_some())
        .ok_or_else(|| {
            brp_request::invalid_params(format!(
                "Entity {bits} is not a connected simulated gamepad (never connected, or \
                 disconnected); connect one with brp_extras/connect_gamepad"
            ))
        })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use std::time::Duration;

    use bevy::app::App;
    use bevy::ecs::message::MessageCursor;
    use bevy::input::InputPlugin;
    use bevy::input::gamepad::Gamepad;
    use bevy::input::gamepad::GamepadAxis;
    use bevy::input::gamepad::GamepadButton;
    use bevy::input::gamepad::RawGamepadAxisChangedEvent;
    use bevy::input::gamepad::RawGamepadButtonChangedEvent;
    use bevy::input::gamepad::RawGamepadEvent;
    use bevy::prelude::Entity;
    use bevy::prelude::In;
    use bevy::prelude::Messages;
    use bevy::prelude::MinimalPlugins;
    use bevy::prelude::Time;
    use bevy::prelude::Virtual;
    use bevy::prelude::World;
    use bevy::time::TimeUpdateStrategy;
    use bevy_remote::BrpResult;
    use bevy_remote::error_codes::INVALID_PARAMS;
    use serde_json::Value;
    use serde_json::json;

    use super::DEFAULT_GAMEPAD_DURATION_MS;
    use super::GamepadPlugin;
    use super::TimedGamepadButtonRelease;
    use super::connect_gamepad_handler;
    use super::disconnect_gamepad_handler;
    use super::send_gamepad_button_handler;
    use super::set_gamepad_axis_handler;
    use super::set_gamepad_button_handler;

    /// An app running Bevy's gamepad systems and `process_timed_gamepad_button_releases`, with
    /// `Time<Virtual>` paused and `Time<Real>` advancing only through [`advance_real_clock`].
    ///
    /// `TimePlugin` still runs `time_system`, which copies the paused `Time<Virtual>` into `Time`
    /// every frame, as in an app that has paused its game clock.
    fn app_with_paused_virtual_clock() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin, GamepadPlugin))
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        // The first `time_system` run records the start instant without advancing `Time<Real>`.
        app.update();
        app
    }

    /// Advances `Time<Real>` by `ms` milliseconds and runs one frame.
    fn advance_real_clock(app: &mut App, ms: u64) {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            ms,
        )));
        app.update();
    }

    /// Runs one frame without advancing either clock.
    fn next_frame(app: &mut App) { advance_real_clock(app, 0); }

    /// Connects a simulated gamepad and runs the frame that turns it into a `Gamepad`.
    fn connect(app: &mut App) -> (u64, Entity) {
        let response = connect_gamepad_handler(In(None), app.world_mut()).expect("connect");
        let bits = response["gamepad"].as_u64().expect("gamepad id");
        next_frame(app);
        (bits, Entity::from_bits(bits))
    }

    fn call(
        handler: fn(In<Option<Value>>, &mut World) -> BrpResult,
        app: &mut App,
        params: Value,
    ) -> BrpResult {
        handler(In(Some(params)), app.world_mut())
    }

    fn pressed(app: &App, gamepad: Entity, button: GamepadButton) -> bool {
        app.world()
            .get::<Gamepad>(gamepad)
            .is_some_and(|pad| pad.pressed(button))
    }

    #[test]
    fn connect_makes_a_gamepad() {
        let mut app = app_with_paused_virtual_clock();
        let (_, gamepad) = connect(&mut app);
        assert!(app.world().get::<Gamepad>(gamepad).is_some());
    }

    #[test]
    fn set_button_holds_until_set_again() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 1.0 }),
        )
        .expect("press");
        advance_real_clock(&mut app, 5_000);
        assert!(pressed(&app, gamepad, GamepadButton::South));

        call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 0.0 }),
        )
        .expect("release");
        next_frame(&mut app);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
    }

    #[test]
    fn send_button_taps_for_the_default_duration() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        let response = call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South" }),
        )
        .expect("tap");
        assert_eq!(response["duration_ms"], DEFAULT_GAMEPAD_DURATION_MS);
        next_frame(&mut app);
        assert!(pressed(&app, gamepad, GamepadButton::South));
        advance_real_clock(&mut app, 99);
        assert!(pressed(&app, gamepad, GamepadButton::South));
        advance_real_clock(&mut app, 1);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
    }

    #[test]
    fn a_new_value_cancels_a_pending_release() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);
        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South" }),
        )
        .expect("tap");
        call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 1.0 }),
        )
        .expect("hold");
        advance_real_clock(&mut app, 5_000);
        assert!(pressed(&app, gamepad, GamepadButton::South));
    }

    /// The hold is measured on the wall clock: a button pressed while the app has paused its
    /// virtual clock still comes back up after its duration.
    #[test]
    fn timed_button_releases_while_the_virtual_clock_is_paused() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "duration_ms": 100 }),
        )
        .expect("press");
        next_frame(&mut app);
        assert!(pressed(&app, gamepad, GamepadButton::South));

        // `process_timed_gamepad_button_releases` writes the release in `PreUpdate` of the frame
        // the hold runs out, and `gamepad_event_processing_system` takes it in the same frame.
        advance_real_clock(&mut app, 100);
        assert_eq!(app.world().resource::<Time>().delta(), Duration::ZERO);
        assert!(
            !pressed(&app, gamepad, GamepadButton::South),
            "the button must be released on the real clock"
        );
    }

    /// A press whose hold has already run out by the first frame is still seen down for that
    /// frame; the release follows in the next.
    #[test]
    fn timed_button_is_seen_down_even_if_the_hold_ran_out() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "duration_ms": 0 }),
        )
        .expect("press");
        advance_real_clock(&mut app, 50);
        assert!(pressed(&app, gamepad, GamepadButton::South));
        next_frame(&mut app);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
    }

    #[test]
    fn zero_ms_tap_after_partial_set_is_seen_pressed() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 0.5 }),
        )
        .expect("partial set");
        next_frame(&mut app);
        assert!(!pressed(&app, gamepad, GamepadButton::South));

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "duration_ms": 0 }),
        )
        .expect("tap");
        next_frame(&mut app);
        assert!(pressed(&app, gamepad, GamepadButton::South));
        next_frame(&mut app);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
    }

    #[test]
    fn axis_stays_where_it_is_put() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            set_gamepad_axis_handler,
            &mut app,
            json!({ "gamepad": bits, "axis": "LeftStickX", "value": -1.0 }),
        )
        .expect("axis");
        advance_real_clock(&mut app, 5_000);
        let value = app
            .world()
            .get::<Gamepad>(gamepad)
            .and_then(|pad| pad.get(GamepadAxis::LeftStickX));
        assert_eq!(value, Some(-1.0));
    }

    #[test]
    fn disconnect_removes_the_gamepad() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            disconnect_gamepad_handler,
            &mut app,
            json!({ "gamepad": bits }),
        )
        .expect("disconnect");
        next_frame(&mut app);
        assert!(app.world().get::<Gamepad>(gamepad).is_none());
    }

    #[test]
    fn disconnect_drops_pending_releases() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, _) = connect(&mut app);
        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "duration_ms": 5_000 }),
        )
        .expect("tap");
        call(
            disconnect_gamepad_handler,
            &mut app,
            json!({ "gamepad": bits }),
        )
        .expect("disconnect");
        let mut releases = app.world_mut().query::<&TimedGamepadButtonRelease>();
        assert_eq!(releases.iter(app.world()).count(), 0);
    }

    #[test]
    fn disconnected_pad_rejects_input() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, _) = connect(&mut app);
        call(
            disconnect_gamepad_handler,
            &mut app,
            json!({ "gamepad": bits }),
        )
        .expect("disconnect");

        for (handler, params) in [
            (
                send_gamepad_button_handler as fn(In<Option<Value>>, &mut World) -> BrpResult,
                json!({ "gamepad": bits, "button": "South" }),
            ),
            (
                set_gamepad_button_handler,
                json!({ "gamepad": bits, "button": "South", "value": 1.0 }),
            ),
            (
                set_gamepad_axis_handler,
                json!({ "gamepad": bits, "axis": "LeftStickX", "value": 1.0 }),
            ),
            (disconnect_gamepad_handler, json!({ "gamepad": bits })),
        ] {
            let error = call(handler, &mut app, params).expect_err("disconnected pad");
            assert_eq!(error.code, INVALID_PARAMS);
            assert!(
                error.message.contains("not a connected simulated gamepad"),
                "{}",
                error.message
            );
        }
    }

    #[test]
    fn tap_sent_in_the_connect_frame_still_releases() {
        let mut app = app_with_paused_virtual_clock();
        let response = connect_gamepad_handler(In(None), app.world_mut()).expect("connect");
        let bits = response["gamepad"].as_u64().expect("gamepad id");
        let gamepad = Entity::from_bits(bits);
        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "duration_ms": 100 }),
        )
        .expect("tap");
        next_frame(&mut app);
        assert!(pressed(&app, gamepad, GamepadButton::South));
        advance_real_clock(&mut app, 100);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
    }

    #[test]
    fn button_and_axis_changes_reach_both_raw_streams() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);
        call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 1.0 }),
        )
        .expect("set button");
        call(
            set_gamepad_axis_handler,
            &mut app,
            json!({ "gamepad": bits, "axis": "LeftStickX", "value": 0.5 }),
        )
        .expect("set axis");
        let button_press = RawGamepadButtonChangedEvent::new(gamepad, GamepadButton::South, 1.0);
        let axis_change = RawGamepadAxisChangedEvent::new(gamepad, GamepadAxis::LeftStickX, 0.5);
        let button_messages = app
            .world()
            .resource::<Messages<RawGamepadButtonChangedEvent>>();
        assert!(
            MessageCursor::default()
                .read(button_messages)
                .any(|event| *event == button_press)
        );
        let axis_messages = app
            .world()
            .resource::<Messages<RawGamepadAxisChangedEvent>>();
        assert!(
            MessageCursor::default()
                .read(axis_messages)
                .any(|event| *event == axis_change)
        );
        let raw_messages = app.world().resource::<Messages<RawGamepadEvent>>();
        assert!(
            MessageCursor::default()
                .read(raw_messages)
                .any(|event| *event == RawGamepadEvent::Button(button_press))
        );
        assert!(
            MessageCursor::default()
                .read(raw_messages)
                .any(|event| *event == RawGamepadEvent::Axis(axis_change))
        );

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "East", "duration_ms": 0 }),
        )
        .expect("tap");
        next_frame(&mut app);
        next_frame(&mut app);
        let release = RawGamepadButtonChangedEvent::new(gamepad, GamepadButton::East, 0.0);
        let button_messages = app
            .world()
            .resource::<Messages<RawGamepadButtonChangedEvent>>();
        assert!(
            MessageCursor::default()
                .read(button_messages)
                .any(|event| *event == release)
        );
        let raw_messages = app.world().resource::<Messages<RawGamepadEvent>>();
        assert!(
            MessageCursor::default()
                .read(raw_messages)
                .any(|event| *event == RawGamepadEvent::Button(release))
        );
    }

    #[test]
    fn set_button_rejects_out_of_range_values() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, _) = connect(&mut app);
        let error = call(
            set_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 1.5 }),
        )
        .expect_err("out of range");
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(error.message.contains("outside [0.0, 1.0]"));
    }

    #[test]
    fn rejects_an_entity_it_did_not_make() {
        let mut app = app_with_paused_virtual_clock();
        let other = app.world_mut().spawn_empty().id().to_bits();
        let error = call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": other, "button": "South" }),
        )
        .expect_err("not a simulated gamepad");
        assert_eq!(error.code, INVALID_PARAMS);
        assert!(
            error.message.contains(&other.to_string()),
            "{}",
            error.message
        );
    }

    #[test]
    fn rejects_out_of_range_values() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, _) = connect(&mut app);
        let error = call(
            set_gamepad_axis_handler,
            &mut app,
            json!({ "gamepad": bits, "axis": "LeftStickX", "value": 2.0 }),
        )
        .expect_err("out of range");
        assert_eq!(error.code, INVALID_PARAMS);
    }
}
