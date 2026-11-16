//! Keyboard modifiers held during mouse operations.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::WindowEvent;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error;

use crate::keyboard;
use crate::keyboard::KeyCodeWrapper;
use crate::window_event;

/// A keyboard modifier held for a whole click or drag.
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Modifier {
    /// The left Shift key.
    Shift,
    /// The left Control key.
    Control,
    /// The left Alt key.
    Alt,
    /// The left Super key.
    Super,
}

impl Modifier {
    /// The left key that the app receives for this modifier.
    pub(super) const fn key(self) -> KeyCodeWrapper {
        match self {
            Self::Shift => KeyCodeWrapper::ShiftLeft,
            Self::Control => KeyCodeWrapper::ControlLeft,
            Self::Alt => KeyCodeWrapper::AltLeft,
            Self::Super => KeyCodeWrapper::SuperLeft,
        }
    }
}

/// Read unique modifiers in request order and name the field in parse errors.
pub(super) fn deserialize_modifiers<'de, D>(deserializer: D) -> Result<Vec<Modifier>, D::Error>
where
    D: Deserializer<'de>,
{
    let requested = Vec::<Modifier>::deserialize(deserializer)
        .map_err(|error| D::Error::custom(format!("Invalid modifiers: {error}")))?;
    let mut modifiers = Vec::with_capacity(requested.len());
    for modifier in requested {
        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }
    Ok(modifiers)
}

/// Modifier keys that must remain held until the next Update.
#[derive(Component)]
pub(super) struct PendingModifierRelease {
    /// The modifier keys to release.
    pub modifiers: Vec<Modifier>,
    /// The window that receives the release events.
    pub window:    Entity,
}

/// The number of clicks and drags that hold each modifier across all windows.
#[derive(Resource, Default)]
pub(super) struct HeldModifiers(HashMap<Modifier, u32>);

impl HeldModifiers {
    /// Update holder counts and create events for keys that change state.
    fn update(
        &mut self,
        modifiers: &[Modifier],
        state: ButtonState,
        window: Entity,
    ) -> Vec<KeyboardInput> {
        let mut keys = Vec::with_capacity(modifiers.len());
        for &modifier in modifiers {
            match state {
                ButtonState::Pressed => {
                    let holders = self.0.entry(modifier).or_default();
                    *holders += 1;
                    if *holders == 1 {
                        keys.push(modifier.key());
                    }
                },
                ButtonState::Released => {
                    if let Entry::Occupied(mut entry) = self.0.entry(modifier) {
                        let holders = entry.get_mut();
                        *holders -= 1;
                        if *holders == 0 {
                            entry.remove();
                            keys.push(modifier.key());
                        }
                    }
                },
            }
        }
        keyboard::create_keyboard_events(&keys, state, window)
    }
}

/// Modifier ownership and the channels that receive modifier events.
#[derive(SystemParam)]
pub(super) struct ModifierEvents<'w> {
    held:                     ResMut<'w, HeldModifiers>,
    keyboard_events:          MessageWriter<'w, KeyboardInput>,
    pub(super) window_events: MessageWriter<'w, WindowEvent>,
}

impl ModifierEvents<'_> {
    /// Hold modifiers and press each key when its first holder starts.
    pub(super) fn press(&mut self, modifiers: &[Modifier], window: Entity) {
        self.send(modifiers, ButtonState::Pressed, window);
    }

    /// Release each key when its last holder finishes.
    pub(super) fn release(&mut self, modifiers: &[Modifier], window: Entity) {
        self.send(modifiers, ButtonState::Released, window);
    }

    fn send(&mut self, modifiers: &[Modifier], state: ButtonState, window: Entity) {
        for event in self.held.update(modifiers, state, window) {
            self.window_events.write(WindowEvent::from(event.clone()));
            self.keyboard_events.write(event);
        }
    }
}

/// Update modifier ownership and send events through the world.
pub(super) fn send_modifier_events(
    world: &mut World,
    modifiers: &[Modifier],
    state: ButtonState,
    window: Entity,
) {
    let events = world
        .resource_mut::<HeldModifiers>()
        .update(modifiers, state, window);
    for event in events {
        window_event::write_input_event(world, event);
    }
}

/// Release keys before timed button releases can create new pending releases.
pub(super) fn process_pending_modifier_releases(
    mut commands: Commands,
    query: Query<(Entity, &PendingModifierRelease)>,
    mut modifier_events: ModifierEvents,
) {
    for (entity, pending) in &query {
        modifier_events.release(&pending.modifiers, pending.window);
        commands.entity(entity).despawn();
    }
}
