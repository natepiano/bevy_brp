//! Registers every reflected component with the `World` at startup
//!
//! Bevy registers a component with the `World` the first time the `World` sees it: a spawn, an
//! insert, or a query. `registry.schema` reports `componentInfo`, which carries `mutable`, only for
//! registered components. Without this pass, a component nothing has spawned yet has no
//! `componentInfo`, so BRP clients cannot tell that a `#[component(immutable)]` component is
//! immutable, and `world.mutate_components` on it panics the app.

use bevy::prelude::*;

/// Registers each type in the `AppTypeRegistry` that reflects `Component` with the `World`
///
/// Components the `World` already knows cost one lookup each. Registration assigns a
/// `ComponentId` and records `ComponentInfo`; it spawns nothing and runs no hooks.
pub(crate) fn register_reflected_components(world: &mut World) {
    let reflect_components: Vec<ReflectComponent> = world
        .resource::<AppTypeRegistry>()
        .read()
        .iter()
        .filter_map(|registration| registration.data::<ReflectComponent>())
        .cloned()
        .collect();

    for reflect_component in reflect_components {
        reflect_component.register_component(world);
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::component::ComponentInfo;

    use super::*;

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    #[component(immutable)]
    struct ImmutableProbe;

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct MutableProbe;

    /// `ComponentInfo::mutable` for `C`, or `None` while the `World` has not registered `C`
    fn mutability<C: Component>(world: &World) -> Option<bool> {
        let component_id = world.component_id::<C>()?;
        world
            .components()
            .get_info(component_id)
            .map(ComponentInfo::mutable)
    }

    #[test]
    fn registers_unspawned_components_with_their_mutability() {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        {
            let mut type_registry = world.resource::<AppTypeRegistry>().write();
            type_registry.register::<ImmutableProbe>();
            type_registry.register::<MutableProbe>();
        }
        assert_eq!(mutability::<ImmutableProbe>(&world), None);

        register_reflected_components(&mut world);

        assert_eq!(mutability::<ImmutableProbe>(&world), Some(false));
        assert_eq!(mutability::<MutableProbe>(&world), Some(true));
    }
}
