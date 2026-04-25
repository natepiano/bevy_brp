//! Minimal BRP event test example
//!
//! Tests `world.trigger_event` BRP method with triggerable events.

use bevy::ecs::observer::On;
use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy_brp_extras::BrpExtrasPlugin;
use bevy_brp_extras::PortDisplay;

/// Test event with no payload
#[derive(Event, Reflect, Clone)]
#[reflect(Event)]
struct TestUnitEvent;

/// Test event with payload
#[derive(Event, Reflect, Clone, Default)]
#[reflect(Event)]
struct TestPayloadEvent {
    pub message: String,
    pub value:   i32,
}

/// Resource to verify events were triggered
#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
struct EventTriggerTracker {
    pub unit_event_count:     u32,
    pub last_payload_message: String,
    pub last_payload_value:   i32,
    pub payload_event_count:  u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Event Test".to_string(),
                resolution: (400, 300).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(BrpExtrasPlugin::new().port_in_title(PortDisplay::Always))
        .init_resource::<EventTriggerTracker>()
        .add_observer(on_unit_event)
        .add_observer(on_payload_event)
        .add_systems(Startup, minimize_window)
        .run();
}

fn on_unit_event(_unit_event: On<TestUnitEvent>, mut tracker: ResMut<EventTriggerTracker>) {
    tracker.unit_event_count += 1;
}

fn on_payload_event(on: On<TestPayloadEvent>, mut tracker: ResMut<EventTriggerTracker>) {
    tracker.last_payload_message.clone_from(&on.event().message);
    tracker.last_payload_value = on.event().value;
    tracker.payload_event_count += 1;
}

fn minimize_window(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        window.set_minimized(true);
    }
}
