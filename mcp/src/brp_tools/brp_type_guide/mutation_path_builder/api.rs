//! Support functions for the mutation path builder module
//!
//! This module contains the public API functions that external callers use to interact
//! with the mutation path builder system. These functions hide internal implementation
//! details and provide a clean interface.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;
use serde::ser::SerializeMap;
use serde_json::Map;
use serde_json::Value;

use super::constants::RESPONSE_AGENT_GUIDANCE_FIELD;
use super::constants::RESPONSE_EXAMPLE_FIELD;
use super::constants::RESPONSE_RESOURCE_FIELD;
use super::constants::RESPONSE_SPAWN_FIELD;
use super::ecs_role::EcsRole;
use super::mutability::Mutability;
use super::mutation_path_external::MutationPathExternal;
use super::mutation_path_external::PathInfo;
use super::not_mutable_reason::NotMutableReason;
use super::path_builder;
use super::path_example::Example;
use super::path_example::PathExample;
use super::path_kind::PathKind;
use super::recursion_context::RecursionContext;
use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
use crate::brp_tools::brp_type_guide::constants::IMMUTABLE_ROOT_DESCRIPTION_TEMPLATE;
use crate::brp_tools::brp_type_guide::constants::INSERT_RESOURCE_GUIDANCE;
use crate::brp_tools::brp_type_guide::constants::NO_COMPONENT_EXAMPLE_TEMPLATE;
use crate::brp_tools::brp_type_guide::constants::NO_RESOURCE_EXAMPLE_TEMPLATE;
use crate::brp_tools::brp_type_guide::constants::OPERATION_INSERT;
use crate::brp_tools::brp_type_guide::constants::OPERATION_SPAWN;
use crate::brp_tools::brp_type_guide::constants::SPAWN_COMPONENT_GUIDANCE;
use crate::brp_tools::brp_type_guide::type_kind::TypeKind;
use crate::error::Error;
use crate::error::Result;
use crate::support::JsonObjectAccess;
use crate::support::SchemaField;

/// Whether the ECS lets a component change in place, read from the schema's
/// `componentInfo.mutable`
///
/// A schema without `componentInfo`, or a `componentInfo` without `mutable`, reads as `Mutable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ComponentMutability {
    /// `componentInfo.mutable` is `true` or absent
    Mutable,
    /// `componentInfo.mutable` is `false` (`#[component(immutable)]`)
    Immutable,
}

impl From<&Value> for ComponentMutability {
    fn from(registry_schema: &Value) -> Self {
        let mutable = registry_schema
            .get_field(SchemaField::ComponentInfo)
            .and_then(|component_info| component_info.get_field(SchemaField::Mutable))
            .and_then(Value::as_bool);

        match mutable {
            Some(false) => Self::Immutable,
            Some(true) | None => Self::Mutable,
        }
    }
}

/// Spawn/insert example with educational guidance for AI agents
///
/// Serializes differently based on variant:
/// - `Spawn` → `{"spawn": {"agent_guidance": "...", "example": <value>}}`
/// - `Resource` → `{"resource": {"agent_guidance": "...", "example": <value>}}`
///
/// When `example` is `Example::NotApplicable`, only `agent_guidance` is included.
///
/// Note: Only derives `Debug` and `Clone` (NOT `Deserialize`) because we implement
/// `Deserialize` manually below with a stub that returns an error.
#[derive(Debug, Clone)]
pub(crate) enum SpawnInsertExample {
    Spawn {
        agent_guidance: String,
        example:        Example,
    },
    Resource {
        agent_guidance: String,
        example:        Example,
    },
}

impl Serialize for SpawnInsertExample {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Spawn {
                agent_guidance,
                example,
            } => {
                let payload = spawn_insert_payload(agent_guidance, example);
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(RESPONSE_SPAWN_FIELD, &payload)?;
                map.end()
            },
            Self::Resource {
                agent_guidance,
                example,
            } => {
                let payload = spawn_insert_payload(agent_guidance, example);
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(RESPONSE_RESOURCE_FIELD, &payload)?;
                map.end()
            },
        }
    }
}

/// Stub `Deserialize` implementation for `SpawnInsertExample`
///
/// Required by serde's flatten attribute but never actually used.
impl<'de> Deserialize<'de> for SpawnInsertExample {
    fn deserialize<D>(_: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "SpawnInsertExample deserialization not implemented - this type is write-only",
        ))
    }
}

/// Entry point for building mutation paths from a type name and registry
///
/// This is the public facade that hides internal implementation details (`PathKind`,
/// `RecursionContext`, `MutationPathInternal`) from external callers. It takes simple
/// inputs and returns the final external format ready for use.
pub(in crate::brp_tools::brp_type_guide) fn build_mutation_paths(
    type_name: &BrpTypeName,
    registry: Arc<HashMap<BrpTypeName, Value>>,
) -> Result<Vec<MutationPathExternal>> {
    // Look up schema to determine TypeKind
    let schema = registry
        .get(type_name)
        .ok_or_else(|| Error::General(format!("Type {type_name} not found in registry")))?;

    let type_kind: TypeKind = schema.into();

    // Create internal context (hidden from caller)
    let path_kind = PathKind::new_root_value(type_name.clone());
    let recursion_context = RecursionContext::new(path_kind, Arc::clone(&registry));

    // Dispatch to the recursive builder
    let internal_paths = path_builder::recurse_mutation_paths(type_kind, &recursion_context)?;

    // Convert internal representation to external format before returning
    let external_paths = internal_paths
        .iter()
        .map(|mutation_path_internal| {
            mutation_path_internal
                .clone()
                .into_mutation_path_external(&registry)
        })
        .collect();

    Ok(external_paths)
}

/// Collapse the mutation paths of an immutable component or resource to one `NotMutable` root
///
/// BRP mutates in place through `ReflectComponent::reflect_mut`, which panics the app for a
/// `#[component(immutable)]` type. `ComponentMutability::Mutable` returns `mutation_paths`
/// unchanged. `ComponentMutability::Immutable` keeps only the root path (`""`), rebuilt by
/// `immutable_root_path`; without a root path it returns no paths.
pub(in crate::brp_tools::brp_type_guide) fn collapse_immutable_component_paths(
    type_name: &BrpTypeName,
    registry_schema: &Value,
    mutation_paths: Vec<MutationPathExternal>,
) -> Vec<MutationPathExternal> {
    match ComponentMutability::from(registry_schema) {
        ComponentMutability::Mutable => mutation_paths,
        ComponentMutability::Immutable => {
            let reflect_traits = registry_schema
                .get_field_array(SchemaField::ReflectTypes)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            let ecs_role = EcsRole::from(reflect_traits.as_slice());

            mutation_paths
                .into_iter()
                .find(|mutation_path| mutation_path.path.is_empty())
                .map(|root| immutable_root_path(type_name, ecs_role, root))
                .into_iter()
                .collect()
        },
    }
}

/// Rebuild `root` as the `NotMutable` root of an immutable component or resource
///
/// Keeps `path_kind`, `type_name`, and `type_kind`; the description and
/// `NotMutableReason::ImmutableComponent` name the insert tool that replaces the whole value.
fn immutable_root_path(
    type_name: &BrpTypeName,
    ecs_role: EcsRole,
    root: MutationPathExternal,
) -> MutationPathExternal {
    let type_kind = root.path_info.type_kind;
    let described_type = if matches!(type_kind, TypeKind::Value) {
        type_name.short_name()
    } else {
        format!(
            "{} {}",
            type_name.short_name(),
            type_kind.description_label()
        )
    };
    let description = IMMUTABLE_ROOT_DESCRIPTION_TEMPLATE
        .replace("{role}", ecs_role.noun())
        .replace("{type}", &described_type)
        .replace("{tool}", ecs_role.insert_tool());
    let reason = NotMutableReason::ImmutableComponent {
        type_name: type_name.clone(),
        ecs_role,
    };

    MutationPathExternal::new(
        root.path,
        description,
        PathInfo {
            path_kind: root.path_info.path_kind,
            type_name: root.path_info.type_name,
            type_kind,
            mutability: Mutability::NotMutable,
            mutability_reason: Option::<Value>::from(&reason),
            applicable_variants: None,
            enum_instructions: None,
            root_example: None,
        },
        PathExample::Simple(Example::NotApplicable),
    )
}

/// Extract spawn/insert example with guidance for AI agents
///
/// Returns `None` for `EcsRole::Other` or when `mutation_paths` has no root path.
/// `EcsRole::Resource` yields `SpawnInsertExample::Resource` even though Bevy 0.20 resources
/// also reflect `Component`; `EcsRole::Component` yields `SpawnInsertExample::Spawn`.
/// `Example::NotApplicable` selects guidance explaining the missing example.
pub(in crate::brp_tools::brp_type_guide) fn extract_spawn_insert_example(
    mutation_paths: &[MutationPathExternal],
    reflect_traits: &[String],
) -> Option<SpawnInsertExample> {
    let root_example = || {
        mutation_paths
            .iter()
            .find(|p| (*p.path).is_empty())
            .map(MutationPathExternal::preferred_example)
    };

    match EcsRole::from(reflect_traits) {
        EcsRole::Resource => {
            let example = root_example()?;
            let agent_guidance = if matches!(example, Example::NotApplicable) {
                NO_RESOURCE_EXAMPLE_TEMPLATE.replace("{}", OPERATION_INSERT)
            } else {
                INSERT_RESOURCE_GUIDANCE.to_string()
            };

            Some(SpawnInsertExample::Resource {
                agent_guidance,
                example,
            })
        },
        EcsRole::Component => {
            let example = root_example()?;
            let agent_guidance = if matches!(example, Example::NotApplicable) {
                NO_COMPONENT_EXAMPLE_TEMPLATE.replace("{}", OPERATION_SPAWN)
            } else {
                SPAWN_COMPONENT_GUIDANCE.to_string()
            };

            Some(SpawnInsertExample::Spawn {
                agent_guidance,
                example,
            })
        },
        EcsRole::Other => None,
    }
}

fn spawn_insert_payload(agent_guidance: &str, example: &Example) -> Value {
    let mut payload = Map::new();
    payload.insert_field(RESPONSE_AGENT_GUIDANCE_FIELD, agent_guidance);
    if !example.is_null_equivalent() {
        payload.insert_field(RESPONSE_EXAMPLE_FIELD, example.to_value());
    }
    Value::Object(payload)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::Map;
    use serde_json::Value;
    use serde_json::json;

    use super::SpawnInsertExample;
    use super::build_mutation_paths;
    use super::collapse_immutable_component_paths;
    use super::extract_spawn_insert_example;
    use crate::brp_tools::brp_type_guide::brp_type_name::BrpTypeName;
    use crate::brp_tools::brp_type_guide::constants::FIELD_INPUT_FOCUS_RECORDED_CHANGES;
    use crate::brp_tools::brp_type_guide::constants::INSERT_RESOURCE_GUIDANCE;
    use crate::brp_tools::brp_type_guide::constants::NO_COMPONENT_EXAMPLE_TEMPLATE;
    use crate::brp_tools::brp_type_guide::constants::OPERATION_SPAWN;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_COMPONENT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_DEFAULT;
    use crate::brp_tools::brp_type_guide::constants::REFLECT_TRAIT_RESOURCE;
    use crate::brp_tools::brp_type_guide::constants::SPAWN_COMPONENT_GUIDANCE;
    use crate::brp_tools::brp_type_guide::constants::TYPE_BEVY_INPUT_FOCUS;
    use crate::brp_tools::brp_type_guide::constants::TYPE_BOOL;
    use crate::brp_tools::brp_type_guide::constants::TYPE_STR_REF;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::MutationPathExternal;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::NotMutableReason;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::ecs_role::EcsRole;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::mutability::Mutability;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::mutation_path::MutationPath;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::mutation_path_external::PathInfo;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::mutation_path_external::RootExample;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::path_example::Example;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::path_example::PathExample;
    use crate::brp_tools::brp_type_guide::mutation_path_builder::path_kind::PathKind;
    use crate::brp_tools::brp_type_guide::type_kind::TypeKind;
    use crate::constants::SCHEMA_REF_PREFIX;
    use crate::error::Error;
    use crate::error::Result;
    use crate::support::JsonObjectAccess;
    use crate::support::SchemaField;

    const CLEAR_COLOR: &str = "bevy_camera::clear_color::ClearColor";
    const COLOR: &str = "bevy_color::color::Color";
    const FIELD_COMPONENT_NAME: &str = "component_name";
    const FIELD_SPAWNED_FROM_SCENE: &str = "spawned_from_scene";
    const RECORDED_CHANGE: &str = "core::option::Option<(bevy_ecs::entity::Entity, bevy_input_focus::gained_and_lost::FocusCause)>";
    const RECORDED_CHANGES: &str = "alloc::vec::Vec<core::option::Option<(bevy_ecs::entity::Entity, bevy_input_focus::gained_and_lost::FocusCause)>>";
    const SCENE_COMPONENT_INFO: &str = "bevy_scene::scene_component::SceneComponentInfo";
    const OPAQUE: &str = "test::Opaque";
    const ICON_DATA: &str = "test::IconData";
    const ICON: &str = "test::Icon";
    const CURSOR: &str = "test::Cursor";
    const OPTION_CURSOR: &str = "core::option::Option<test::Cursor>";
    const DEFAULT_CURSOR: &str = "test::DefaultCursor";
    const OVERRIDE_CURSOR: &str = "test::OverrideCursor";
    const ICON_WRAPPER: &str = "test::IconWrapper";

    /// `(path, mutability, example, description, mutability_reason)` of one mutation path
    type PathSummary = (String, Mutability, Example, String, Option<Value>);

    /// Mutable root path (`""`) for `ClearColor` carrying a JSON example
    fn clear_color_root_path() -> Vec<MutationPathExternal> {
        let type_name = BrpTypeName::from(CLEAR_COLOR);
        vec![MutationPathExternal::new(
            MutationPath::from(""),
            String::new(),
            PathInfo {
                path_kind: PathKind::new_root_value(type_name.clone()),
                type_name,
                type_kind: TypeKind::TupleStruct,
                mutability: Mutability::Mutable,
                mutability_reason: None,
                applicable_variants: None,
                enum_instructions: None,
                root_example: None,
            },
            PathExample::Simple(Example::Json(json!({ "Srgba": { "red": 1.0 } }))),
        )]
    }

    /// `clear_color_root_path` plus a mutable `.0` element path holding the `Color` enum
    fn clear_color_paths() -> Vec<MutationPathExternal> {
        let clear_color = BrpTypeName::from(CLEAR_COLOR);
        let color = BrpTypeName::from(COLOR);
        let mut paths = clear_color_root_path();
        paths.push(MutationPathExternal::new(
            MutationPath::from(".0"),
            String::new(),
            PathInfo {
                path_kind:           PathKind::new_indexed_element(0, color.clone(), clear_color),
                type_name:           color,
                type_kind:           TypeKind::Enum,
                mutability:          Mutability::Mutable,
                mutability_reason:   None,
                applicable_variants: None,
                enum_instructions:   None,
                root_example:        None,
            },
            PathExample::Simple(Example::Json(json!({ "Srgba": { "red": 1.0 } }))),
        ));
        paths
    }

    fn reflect_traits(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    /// Registry schema carrying `kind` and `reflectTypes`
    fn schema(type_kind: &TypeKind, reflect_traits: &[&str]) -> Value {
        let mut schema = Value::Object(Map::new());
        schema.insert_field(SchemaField::Kind.as_ref(), type_kind.as_ref());
        schema.insert_field(SchemaField::ReflectTypes.as_ref(), reflect_traits.to_vec());
        schema
    }

    /// `ClearColor` tuple struct schema with `componentInfo.mutable` set to `mutable`
    fn schema_with_component_mutable(reflect_traits: &[&str], mutable: bool) -> Value {
        let mut component_info = Map::new();
        component_info.insert_field(SchemaField::Mutable.as_ref(), mutable);
        let mut schema = schema(&TypeKind::TupleStruct, reflect_traits);
        schema.insert_field(SchemaField::ComponentInfo.as_ref(), component_info);
        schema
    }

    /// Field or item reference `{"type": {"$ref": "#/$defs/<type_name>"}}`
    fn type_ref(type_name: &str) -> Value {
        let mut reference = Map::new();
        reference.insert_field(
            SchemaField::Ref.as_ref(),
            format!("{SCHEMA_REF_PREFIX}{type_name}"),
        );
        let mut field = Map::new();
        field.insert_field(SchemaField::Type.as_ref(), reference);
        Value::Object(field)
    }

    fn summarize(paths: &[MutationPathExternal]) -> Vec<PathSummary> {
        paths
            .iter()
            .map(|mutation_path| {
                (
                    mutation_path.path.to_string(),
                    mutation_path.path_info.mutability,
                    mutation_path.preferred_example(),
                    mutation_path.description.clone(),
                    mutation_path.path_info.mutability_reason.clone(),
                )
            })
            .collect()
    }

    fn mutability_of(paths: &[MutationPathExternal], path: &str) -> Option<Mutability> {
        paths
            .iter()
            .find(|mutation_path| mutation_path.path.as_str() == path)
            .map(|mutation_path| mutation_path.path_info.mutability)
    }

    /// `SceneComponentInfo { spawned_from_scene: bool, component_name: &'static str }`
    fn scene_component_info_registry() -> Arc<HashMap<BrpTypeName, Value>> {
        let mut properties = Map::new();
        properties.insert_field(FIELD_SPAWNED_FROM_SCENE, type_ref(TYPE_BOOL));
        properties.insert_field(FIELD_COMPONENT_NAME, type_ref(TYPE_STR_REF));
        let mut scene_component_info = schema(&TypeKind::Struct, &[REFLECT_TRAIT_COMPONENT]);
        scene_component_info.insert_field(SchemaField::Properties.as_ref(), properties);

        Arc::new(HashMap::from([
            (
                BrpTypeName::from(SCENE_COMPONENT_INFO),
                scene_component_info,
            ),
            (BrpTypeName::from(TYPE_BOOL), schema(&TypeKind::Value, &[])),
            (
                BrpTypeName::from(TYPE_STR_REF),
                schema(&TypeKind::Value, &[]),
            ),
        ]))
    }

    /// `InputFocus { recorded_changes: Vec<Option<(Entity, FocusCause)>> }`; the `Option`
    /// element type is absent from the registry
    fn input_focus_registry() -> Arc<HashMap<BrpTypeName, Value>> {
        let mut properties = Map::new();
        properties.insert_field(
            FIELD_INPUT_FOCUS_RECORDED_CHANGES,
            type_ref(RECORDED_CHANGES),
        );
        let mut input_focus = schema(
            &TypeKind::Struct,
            &[
                REFLECT_TRAIT_COMPONENT,
                REFLECT_TRAIT_DEFAULT,
                REFLECT_TRAIT_RESOURCE,
            ],
        );
        input_focus.insert_field(SchemaField::Properties.as_ref(), properties);

        let mut recorded_changes = schema(&TypeKind::List, &[]);
        recorded_changes.insert_field(SchemaField::Items.as_ref(), type_ref(RECORDED_CHANGE));

        Arc::new(HashMap::from([
            (BrpTypeName::from(TYPE_BEVY_INPUT_FOCUS), input_focus),
            (BrpTypeName::from(RECORDED_CHANGES), recorded_changes),
        ]))
    }

    /// Enum schema whose `oneOf` lists `variants`
    fn enum_schema(reflect_traits: &[&str], variants: Vec<Value>) -> Value {
        let mut schema = schema(&TypeKind::Enum, reflect_traits);
        schema.insert_field(SchemaField::OneOf.as_ref(), variants);
        schema
    }

    /// Enum variant `type_path`, a tuple variant over `field_type` when one is given
    fn variant(type_path: &str, field_type: Option<&str>) -> Value {
        let mut variant = Map::new();
        variant.insert_field(SchemaField::TypePath.as_ref(), type_path);
        if let Some(field_type) = field_type {
            variant.insert_field(
                SchemaField::PrefixItems.as_ref(),
                vec![type_ref(field_type)],
            );
        }
        Value::Object(variant)
    }

    /// Single-field tuple struct schema over `field_type`
    fn newtype_schema(reflect_traits: &[&str], field_type: &str) -> Value {
        let mut schema = schema(&TypeKind::TupleStruct, reflect_traits);
        schema.insert_field(
            SchemaField::PrefixItems.as_ref(),
            vec![type_ref(field_type)],
        );
        schema
    }

    /// Cursor types shaped like `bevy_picking::cursor`:
    /// - `ICON_DATA { flag: bool, opaque: Opaque }` reflects `Default`; `Opaque` has no example
    /// - `Icon { Image(IconData), System(bool) }`: `Image` has no complete example
    /// - `Cursor { Custom(Icon), Hidden }`: `Custom` is constructible through `Icon::System`
    /// - `DEFAULT_CURSOR(Cursor)`, `OVERRIDE_CURSOR(Option<Cursor>)`, `ICON_WRAPPER(IconData)`
    fn cursor_registry() -> Arc<HashMap<BrpTypeName, Value>> {
        let mut properties = Map::new();
        properties.insert_field("flag", type_ref(TYPE_BOOL));
        properties.insert_field("opaque", type_ref(OPAQUE));
        let mut icon_data = schema(&TypeKind::Struct, &[REFLECT_TRAIT_DEFAULT]);
        icon_data.insert_field(SchemaField::Properties.as_ref(), properties);
        let resource = [REFLECT_TRAIT_DEFAULT, REFLECT_TRAIT_RESOURCE];

        Arc::new(HashMap::from([
            (BrpTypeName::from(TYPE_BOOL), schema(&TypeKind::Value, &[])),
            (BrpTypeName::from(OPAQUE), schema(&TypeKind::Value, &[])),
            (BrpTypeName::from(ICON_DATA), icon_data),
            (
                BrpTypeName::from(ICON),
                enum_schema(
                    &[],
                    vec![
                        variant("test::Icon::Image", Some(ICON_DATA)),
                        variant("test::Icon::System", Some(TYPE_BOOL)),
                    ],
                ),
            ),
            (
                BrpTypeName::from(CURSOR),
                enum_schema(
                    &[],
                    vec![
                        variant("test::Cursor::Custom", Some(ICON)),
                        variant("test::Cursor::Hidden", None),
                    ],
                ),
            ),
            (
                BrpTypeName::from(OPTION_CURSOR),
                enum_schema(
                    &[],
                    vec![
                        variant("core::option::Option<test::Cursor>::None", None),
                        variant("core::option::Option<test::Cursor>::Some", Some(CURSOR)),
                    ],
                ),
            ),
            (
                BrpTypeName::from(DEFAULT_CURSOR),
                newtype_schema(&resource, CURSOR),
            ),
            (
                BrpTypeName::from(OVERRIDE_CURSOR),
                newtype_schema(&resource, OPTION_CURSOR),
            ),
            (
                BrpTypeName::from(ICON_WRAPPER),
                newtype_schema(&resource, ICON_DATA),
            ),
        ]))
    }

    /// Root `(mutability, preferred example, description)` of `type_name` in `cursor_registry`
    fn cursor_root(type_name: &str) -> Result<(Mutability, Example, String)> {
        let paths = build_mutation_paths(&BrpTypeName::from(type_name), cursor_registry())?;
        let (_, mutability, example, description, _) = summarize(&paths)
            .into_iter()
            .find(|(path, ..)| path.is_empty())
            .ok_or_else(|| Error::General(format!("{type_name} has no root mutation path")))?;
        Ok((mutability, example, description))
    }

    #[test]
    fn newtype_over_partially_mutable_enum_uses_constructible_variant() -> Result<()> {
        let (mutability, example, description) = cursor_root(DEFAULT_CURSOR)?;

        assert_eq!(mutability, Mutability::PartiallyMutable);
        assert_eq!(
            example,
            Example::Json(json!({ "Custom": { "System": true } }))
        );
        assert_eq!(
            description,
            "This tuple struct path is partially mutable due to some of its elements not being mutable."
        );
        Ok(())
    }

    #[test]
    fn option_over_partially_mutable_enum_prefers_some() -> Result<()> {
        let (_, example, description) = cursor_root(OVERRIDE_CURSOR)?;

        assert_eq!(
            example,
            Example::Json(json!({ "Custom": { "System": true } }))
        );
        assert!(!description.contains("{}"));
        Ok(())
    }

    #[test]
    fn variant_holding_constructible_enum_gets_example() -> Result<()> {
        let paths = build_mutation_paths(&BrpTypeName::from(CURSOR), cursor_registry())?;

        let root = paths
            .iter()
            .find(|mutation_path| mutation_path.path.is_empty());
        assert_eq!(
            root.map(MutationPathExternal::preferred_example),
            Some(Example::Json(json!({ "Custom": { "System": true } })))
        );
        // `.0` (the `Icon` in `Custom`) has an available root example
        assert!(paths.iter().any(|mutation_path| {
            mutation_path.path.as_str() == ".0"
                && matches!(
                    mutation_path.path_info.root_example,
                    Some(RootExample::Available { .. })
                )
        }));
        Ok(())
    }

    #[test]
    fn nested_variant_with_incomplete_field_is_unavailable() -> Result<()> {
        // `.0.0.flag` sits in `Icon::Image` inside `Cursor::Custom`: variant chain
        // `[Cursor::Custom, Icon::Image]`. `Icon::Image` is judged by the last chain entry.
        let paths = build_mutation_paths(&BrpTypeName::from(CURSOR), cursor_registry())?;

        let flag = paths
            .iter()
            .find(|mutation_path| mutation_path.path.as_str() == ".0.0.flag");
        assert!(matches!(
            flag.and_then(|mutation_path| mutation_path.path_info.root_example.as_ref()),
            Some(RootExample::Unavailable { unavailable_reason })
                if unavailable_reason.starts_with("Cannot construct Image variant")
        ));
        Ok(())
    }

    #[test]
    fn partially_mutable_tuple_struct_without_complete_example_gets_no_empty_object() -> Result<()>
    {
        let (mutability, example, description) = cursor_root(ICON_WRAPPER)?;

        assert_eq!(mutability, Mutability::PartiallyMutable);
        assert_eq!(example, Example::NotApplicable);
        assert_eq!(
            description,
            "This tuple struct path is partially mutable due to some of its elements not being mutable. No example is provided."
        );
        Ok(())
    }

    #[test]
    fn partially_mutable_named_struct_with_default_gets_empty_object() -> Result<()> {
        let (_, example, description) = cursor_root(ICON_DATA)?;

        assert_eq!(example, Example::Json(json!({})));
        assert!(description.contains("accepts empty object {}"));
        assert!(!description.contains("No example is provided."));
        Ok(())
    }

    #[test]
    fn resource_that_also_reflects_component_gets_insert_guidance() {
        // Bevy 0.20 `#[reflect(Resource)]` also registers `ReflectComponent`.
        let reflect_traits = reflect_traits(&[
            REFLECT_TRAIT_COMPONENT,
            REFLECT_TRAIT_DEFAULT,
            REFLECT_TRAIT_RESOURCE,
        ]);

        let spawn_insert_example =
            extract_spawn_insert_example(&clear_color_root_path(), &reflect_traits);

        assert!(matches!(
            spawn_insert_example,
            Some(SpawnInsertExample::Resource { ref agent_guidance, .. })
                if agent_guidance == INSERT_RESOURCE_GUIDANCE
        ));
    }

    #[test]
    fn component_gets_spawn_guidance() {
        let reflect_traits = reflect_traits(&[REFLECT_TRAIT_COMPONENT, REFLECT_TRAIT_DEFAULT]);

        let spawn_insert_example =
            extract_spawn_insert_example(&clear_color_root_path(), &reflect_traits);

        assert!(matches!(
            spawn_insert_example,
            Some(SpawnInsertExample::Spawn { ref agent_guidance, .. })
                if agent_guidance == SPAWN_COMPONENT_GUIDANCE
        ));
    }

    #[test]
    fn type_without_ecs_traits_gets_no_example() {
        let reflect_traits = reflect_traits(&[REFLECT_TRAIT_DEFAULT]);

        assert!(extract_spawn_insert_example(&clear_color_root_path(), &reflect_traits).is_none());
    }

    #[test]
    fn immutable_component_collapses_to_not_mutable_root() {
        let type_name = BrpTypeName::from(CLEAR_COLOR);
        let schema = schema_with_component_mutable(&[REFLECT_TRAIT_COMPONENT], false);

        let paths = collapse_immutable_component_paths(&type_name, &schema, clear_color_paths());

        let reason = NotMutableReason::ImmutableComponent {
            type_name,
            ecs_role: EcsRole::Component,
        };
        assert_eq!(
            summarize(&paths),
            vec![(
                String::new(),
                Mutability::NotMutable,
                Example::NotApplicable,
                "This immutable component cannot be mutated in place; replace the entire ClearColor tuple struct with 'mcp__brp__world_insert_components'".to_string(),
                Option::<Value>::from(&reason),
            )]
        );
    }

    #[test]
    fn immutable_resource_names_insert_resources() {
        let type_name = BrpTypeName::from(CLEAR_COLOR);
        let schema = schema_with_component_mutable(
            &[
                REFLECT_TRAIT_COMPONENT,
                REFLECT_TRAIT_DEFAULT,
                REFLECT_TRAIT_RESOURCE,
            ],
            false,
        );

        let paths = collapse_immutable_component_paths(&type_name, &schema, clear_color_paths());

        let reason = NotMutableReason::ImmutableComponent {
            type_name,
            ecs_role: EcsRole::Resource,
        };
        assert_eq!(
            summarize(&paths),
            vec![(
                String::new(),
                Mutability::NotMutable,
                Example::NotApplicable,
                "This immutable resource cannot be mutated in place; replace the entire ClearColor tuple struct with 'mcp__brp__world_insert_resources'".to_string(),
                Option::<Value>::from(&reason),
            )]
        );
    }

    #[test]
    fn mutable_component_keeps_paths() {
        let schema = schema_with_component_mutable(&[REFLECT_TRAIT_COMPONENT], true);

        let paths = collapse_immutable_component_paths(
            &BrpTypeName::from(CLEAR_COLOR),
            &schema,
            clear_color_paths(),
        );

        assert_eq!(summarize(&paths), summarize(&clear_color_paths()));
    }

    #[test]
    fn missing_component_info_keeps_paths() {
        let schema = schema(&TypeKind::TupleStruct, &[REFLECT_TRAIT_COMPONENT]);

        let paths = collapse_immutable_component_paths(
            &BrpTypeName::from(CLEAR_COLOR),
            &schema,
            clear_color_paths(),
        );

        assert_eq!(summarize(&paths), summarize(&clear_color_paths()));
    }

    #[test]
    fn str_field_is_not_mutable_and_leaves_no_spawn_example() -> Result<()> {
        let paths = build_mutation_paths(
            &BrpTypeName::from(SCENE_COMPONENT_INFO),
            scene_component_info_registry(),
        )?;

        assert_eq!(
            mutability_of(&paths, ""),
            Some(Mutability::PartiallyMutable)
        );
        assert_eq!(
            mutability_of(&paths, ".spawned_from_scene"),
            Some(Mutability::Mutable)
        );
        assert_eq!(
            mutability_of(&paths, ".component_name"),
            Some(Mutability::NotMutable)
        );
        let reason = NotMutableReason::MissingReflectDeserialize(BrpTypeName::from(TYPE_STR_REF));
        assert!(paths.iter().any(|mutation_path| {
            mutation_path.path.as_str() == ".component_name"
                && mutation_path.path_info.mutability_reason == Option::<Value>::from(&reason)
        }));

        let spawn_insert_example =
            extract_spawn_insert_example(&paths, &reflect_traits(&[REFLECT_TRAIT_COMPONENT]));
        let no_example_guidance = NO_COMPONENT_EXAMPLE_TEMPLATE.replace("{}", OPERATION_SPAWN);
        assert!(matches!(
            spawn_insert_example,
            Some(SpawnInsertExample::Spawn {
                ref agent_guidance,
                example: Example::NotApplicable,
            }) if *agent_guidance == no_example_guidance
        ));
        Ok(())
    }

    #[test]
    fn input_focus_recorded_changes_is_one_mutable_root_value() -> Result<()> {
        let paths = build_mutation_paths(
            &BrpTypeName::from(TYPE_BEVY_INPUT_FOCUS),
            input_focus_registry(),
        )?;

        let path_names: Vec<&str> = paths
            .iter()
            .map(|mutation_path| mutation_path.path.as_str())
            .collect();
        assert_eq!(path_names, ["", ".recorded_changes"]);

        assert!(paths.iter().any(|mutation_path| {
            mutation_path.path.as_str() == ".recorded_changes"
                && mutation_path.path_info.mutability == Mutability::Mutable
                && mutation_path.preferred_example() == Example::Json(json!([]))
        }));

        let root_example = paths
            .iter()
            .find(|mutation_path| mutation_path.path.is_empty())
            .map(|mutation_path| mutation_path.preferred_example().to_value());
        assert_eq!(
            root_example
                .as_ref()
                .and_then(|example| example.get(FIELD_INPUT_FOCUS_RECORDED_CHANGES)),
            Some(&json!([]))
        );
        Ok(())
    }
}
