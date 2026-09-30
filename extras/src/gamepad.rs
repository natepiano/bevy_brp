//! Simulated gamepads for Bevy Remote Protocol
//!
//! Bevy's gamepad layer is fed by messages: `bevy_gilrs` writes a [`GamepadConnectionEvent`]
//! and [`RawGamepadEvent`]s, and `gamepad_event_processing_system` turns them into [`Gamepad`]
//! state. Writing the same messages for an entity of our own gives a pad that the app treats
//! as real: its input managers, menus and join screens all see it. No gilrs, no hardware.

use std::time::Duration;

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
use bevy_remote::error_codes::INVALID_PARAMS;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::constants::EXTRAS_COMMAND_PREFIX;
use crate::constants::METHOD_CONNECT_GAMEPAD;
use crate::constants::METHOD_DISCONNECT_GAMEPAD;
use crate::constants::METHOD_SEND_GAMEPAD_BUTTON;
use crate::constants::METHOD_SET_GAMEPAD_AXIS;
use crate::constants::MISSING_REQUEST_PARAMETERS_MESSAGE;

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
pub(crate) struct SimulatedGamepad;

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
    /// Button to set
    button:      GamepadButton,
    /// Analog value in `[0.0, 1.0]` (default: 1.0, pressed)
    #[serde(default)]
    value:       Option<f32>,
    /// Release after this many milliseconds (default: hold until set again, max: 60000)
    #[serde(default)]
    duration_ms: Option<u32>,
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
    /// Button that was set
    button:      GamepadButton,
    /// Value it was set to
    value:       f32,
    /// Hold before the release, if timed
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_ms: Option<u32>,
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
pub(crate) fn remote_methods(world: &mut World) -> [(String, RemoteMethodSystemId); 4] {
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
pub(crate) fn connect_gamepad_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: ConnectGamepadRequest = parse_request(Some(
        params.unwrap_or_else(|| Value::Object(Map::default())),
    ))?;
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
    world.write_message(event.clone());
    world.write_message(RawGamepadEvent::Connection(event));

    to_value(GamepadResponse {
        gamepad: gamepad.to_bits(),
    })
}

/// Handler for `send_gamepad_button` BRP method
///
/// Sets a button's analog value. With `duration_ms` the button is released after that much
/// real time; without it, it stays until set again.
pub(crate) fn send_gamepad_button_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: SendGamepadButtonRequest = parse_request(params)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;
    let value = request.value.unwrap_or(1.0);
    if !(0.0..=1.0).contains(&value) {
        return Err(invalid_params(format!(
            "Button value {value} is outside [0.0, 1.0]"
        )));
    }
    if let Some(duration_ms) = request.duration_ms
        && duration_ms > MAX_GAMEPAD_DURATION_MS
    {
        return Err(invalid_params(format!(
            "Duration exceeds maximum: {duration_ms}ms > {MAX_GAMEPAD_DURATION_MS}ms"
        )));
    }

    // A new value for a button replaces any release still pending for it.
    let pending: Vec<Entity> = world
        .query::<(Entity, &TimedGamepadButtonRelease)>()
        .iter(world)
        .filter(|(_, release)| release.gamepad == gamepad && release.button == request.button)
        .map(|(entity, _)| entity)
        .collect();
    for entity in pending {
        world.despawn(entity);
    }
    if let Some(duration_ms) = request.duration_ms
        && value != 0.0
    {
        world.spawn(TimedGamepadButtonRelease {
            gamepad,
            button: request.button,
            timer: Timer::new(
                Duration::from_millis(u64::from(duration_ms)),
                TimerMode::Once,
            ),
        });
    }

    world.write_message(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
        gamepad,
        request.button,
        value,
    )));

    to_value(SendGamepadButtonResponse {
        gamepad: request.gamepad,
        button: request.button,
        value,
        duration_ms: request.duration_ms,
    })
}

/// Handler for `set_gamepad_axis` BRP method
///
/// Sets an axis value. Axes stay where they are put.
pub(crate) fn set_gamepad_axis_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: SetGamepadAxisRequest = parse_request(params)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;
    if !(-1.0..=1.0).contains(&request.value) {
        return Err(invalid_params(format!(
            "Axis value {} is outside [-1.0, 1.0]",
            request.value
        )));
    }

    world.write_message(RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
        gamepad,
        request.axis,
        request.value,
    )));

    to_value(SetGamepadAxisResponse {
        gamepad: request.gamepad,
        axis:    request.axis,
        value:   request.value,
    })
}

/// Handler for `disconnect_gamepad` BRP method
///
/// Bevy removes the [`Gamepad`] component and leaves the entity, as it does for a real pad.
pub(crate) fn disconnect_gamepad_handler(
    In(params): In<Option<Value>>,
    world: &mut World,
) -> BrpResult {
    let request: DisconnectGamepadRequest = parse_request(params)?;
    let gamepad = simulated_gamepad(world, request.gamepad)?;

    let pending: Vec<Entity> = world
        .query::<(Entity, &TimedGamepadButtonRelease)>()
        .iter(world)
        .filter(|(_, release)| release.gamepad == gamepad)
        .map(|(entity, _)| entity)
        .collect();
    for entity in pending {
        world.despawn(entity);
    }

    let event = GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected);
    world.write_message(event.clone());
    world.write_message(RawGamepadEvent::Connection(event));

    to_value(GamepadResponse {
        gamepad: request.gamepad,
    })
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
    gamepads: Query<&Gamepad>,
    mut raw_events: MessageWriter<RawGamepadEvent>,
) {
    for (entity, mut release) in &mut query {
        release.timer.tick(time.delta());

        let Ok(gamepad) = gamepads.get(release.gamepad) else {
            commands.entity(entity).despawn();
            continue;
        };
        let taken = gamepad
            .get(release.button)
            .is_some_and(|value| value != 0.0);
        if release.timer.is_finished() && taken {
            raw_events.write(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                release.gamepad,
                release.button,
                0.0,
            )));
            commands.entity(entity).despawn();
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

const fn invalid_params(message: String) -> BrpError {
    BrpError {
        code: INVALID_PARAMS,
        message,
        data: None,
    }
}

fn parse_request<T: serde::de::DeserializeOwned>(params: Option<Value>) -> Result<T, BrpError> {
    let params =
        params.ok_or_else(|| invalid_params(MISSING_REQUEST_PARAMETERS_MESSAGE.to_string()))?;
    serde_json::from_value(params)
        .map_err(|e| invalid_params(format!("Failed to parse parameters: {e}")))
}

fn to_value<T: Serialize>(response: T) -> BrpResult {
    serde_json::to_value(response).map_err(|e| BrpError {
        code:    bevy_remote::error_codes::INTERNAL_ERROR,
        message: format!("Failed to serialize response: {e}"),
        data:    None,
    })
}

/// Resolve the `gamepad` parameter to a simulated gamepad entity
fn simulated_gamepad(world: &World, bits: u64) -> Result<Entity, BrpError> {
    Entity::try_from_bits(bits)
        .filter(|&entity| world.get::<SimulatedGamepad>(entity).is_some())
        .ok_or_else(|| {
            invalid_params(format!(
                "Entity {bits} is not a simulated gamepad; create one with \
                 brp_extras/connect_gamepad"
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
    use bevy::input::InputPlugin;
    use bevy::input::gamepad::Gamepad;
    use bevy::input::gamepad::GamepadAxis;
    use bevy::input::gamepad::GamepadButton;
    use bevy::prelude::Entity;
    use bevy::prelude::In;
    use bevy::prelude::MinimalPlugins;
    use bevy::prelude::Time;
    use bevy::prelude::Virtual;
    use bevy::prelude::World;
    use bevy::time::TimeUpdateStrategy;
    use bevy_remote::BrpResult;
    use bevy_remote::error_codes::INVALID_PARAMS;
    use serde_json::Value;
    use serde_json::json;

    use super::GamepadPlugin;
    use super::connect_gamepad_handler;
    use super::disconnect_gamepad_handler;
    use super::send_gamepad_button_handler;
    use super::set_gamepad_axis_handler;

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
    fn button_holds_until_set_again() {
        let mut app = app_with_paused_virtual_clock();
        let (bits, gamepad) = connect(&mut app);

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South" }),
        )
        .expect("press");
        advance_real_clock(&mut app, 5_000);
        assert!(pressed(&app, gamepad, GamepadButton::South));

        call(
            send_gamepad_button_handler,
            &mut app,
            json!({ "gamepad": bits, "button": "South", "value": 0.0 }),
        )
        .expect("release");
        next_frame(&mut app);
        assert!(!pressed(&app, gamepad, GamepadButton::South));
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
            json!({ "gamepad": bits, "button": "South", "duration_ms": 1 }),
        )
        .expect("press");
        advance_real_clock(&mut app, 50);
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
