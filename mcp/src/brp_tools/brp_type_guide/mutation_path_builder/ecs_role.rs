//! Classifies a registered type as an ECS resource, component, or neither from its reflect traits

use super::constants::RESPONSE_RESOURCE_FIELD;
use super::constants::RESPONSE_SPAWN_FIELD;
use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_COMPONENT;
use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_RESOURCE;
use crate::brp_tools::brp_type_guide::constants::ROLE_COMPONENT;
use crate::brp_tools::brp_type_guide::constants::ROLE_RESOURCE;
use crate::brp_tools::brp_type_guide::constants::TOOL_WORLD_INSERT_COMPONENTS;
use crate::brp_tools::brp_type_guide::constants::TOOL_WORLD_INSERT_RESOURCES;

/// How a type enters the ECS world, read from the schema's `reflectTypes` array
///
/// Bevy 0.20 stores resources as components (each carries the required
/// `bevy_ecs::resource::IsResource`), and `#[reflect(Resource)]` also registers
/// `ReflectComponent`, so a resource's reflect traits list both `Resource` and `Component`.
/// `Resource` therefore takes precedence: such a type is inserted with
/// `world_insert_resources`, not spawned on an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EcsRole {
    /// Reflects `Resource`, with or without `Component`
    Resource,
    /// Reflects `Component` without `Resource`
    Component,
    /// Reflects neither `Resource` nor `Component`
    Other,
}

// Agent-facing wording for replacing a whole value. `EcsRole::Other` takes the component form,
// matching the spawn guidance in `mutation_path_internal`.
impl EcsRole {
    /// Noun naming the role: `ROLE_RESOURCE` or `ROLE_COMPONENT`
    pub(super) const fn noun(self) -> &'static str {
        match self {
            Self::Resource => ROLE_RESOURCE,
            Self::Component | Self::Other => ROLE_COMPONENT,
        }
    }

    /// Tool that replaces the whole value: `TOOL_WORLD_INSERT_RESOURCES` or
    /// `TOOL_WORLD_INSERT_COMPONENTS`
    pub(super) const fn insert_tool(self) -> &'static str {
        match self {
            Self::Resource => TOOL_WORLD_INSERT_RESOURCES,
            Self::Component | Self::Other => TOOL_WORLD_INSERT_COMPONENTS,
        }
    }

    /// Type guide field holding the whole-value example: `resource` or `spawn`
    pub(super) const fn example_field(self) -> &'static str {
        match self {
            Self::Resource => RESPONSE_RESOURCE_FIELD,
            Self::Component | Self::Other => RESPONSE_SPAWN_FIELD,
        }
    }
}

impl<S: AsRef<str>> From<&[S]> for EcsRole {
    fn from(reflect_traits: &[S]) -> Self {
        let reflects = |trait_name: &str| {
            reflect_traits
                .iter()
                .any(|reflect_trait| reflect_trait.as_ref() == trait_name)
        };

        if reflects(REFLECT_TRAIT_RESOURCE) {
            Self::Resource
        } else if reflects(REFLECT_TRAIT_COMPONENT) {
            Self::Component
        } else {
            Self::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::EcsRole;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_COMPONENT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_DEFAULT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_RESOURCE;

    #[test]
    fn resource_reflecting_component_classifies_as_resource() {
        let reflect_traits = [
            REFLECT_TRAIT_COMPONENT,
            REFLECT_TRAIT_DEFAULT,
            REFLECT_TRAIT_RESOURCE,
        ];
        assert_eq!(EcsRole::from(reflect_traits.as_slice()), EcsRole::Resource);
    }

    #[test]
    fn component_without_resource_classifies_as_component() {
        let reflect_traits = [REFLECT_TRAIT_COMPONENT, REFLECT_TRAIT_DEFAULT];
        assert_eq!(EcsRole::from(reflect_traits.as_slice()), EcsRole::Component);
    }

    #[test]
    fn type_without_ecs_traits_classifies_as_other() {
        let reflect_traits = [REFLECT_TRAIT_DEFAULT];
        assert_eq!(EcsRole::from(reflect_traits.as_slice()), EcsRole::Other);
    }
}
