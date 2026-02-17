//! Shared helper for writing input events to both individual and `WindowEvent` channels.
//!
//! Bevy's picking system reads `MessageReader<WindowEvent>`, while other systems read
//! individual message types like `MessageReader<CursorMoved>`. Bevy's winit integration
//! writes to both channels, so our simulated input must do the same.

use bevy::ecs::message::Message;
use bevy::prelude::*;
use bevy::window::WindowEvent;

/// Write an event to both its individual message channel and the `WindowEvent` channel.
///
/// This ensures systems reading either channel (like `bevy_picking`) see the event.
/// Mirrors the dual-write pattern from `bevy_winit::state::forward_bevy_events()`.
pub fn write_input_event<T>(world: &mut World, event: T)
where
    T: Clone + Message,
    WindowEvent: From<T>,
{
    world.write_message(WindowEvent::from(event.clone()));
    world.write_message(event);
}
