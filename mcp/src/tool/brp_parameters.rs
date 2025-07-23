/// Parameter structs for BRP tools
///
/// These are parameter structs used by the BRP tools.
/// Each struct corresponds to a tool with the same name (minus the Params suffix).
/// These are used by the proc macro on `ToolName` to automatically construct the tool handler
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::brp_tools::default_port;

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DestroyParams {
    /// The entity ID to destroy
    #[to_metadata]
    pub entity: u64,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct DiscoverFormatParams {
    /// Array of fully-qualified component type names to discover formats for
    #[to_metadata]
    pub types: Vec<String>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:  u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetParams {
    /// The entity ID to get component data from
    #[to_metadata]
    pub entity:     u64,
    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    #[to_result]
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct GetResourceParams {
    /// The fully-qualified type name of the resource to get
    #[to_metadata]
    pub resource: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct InsertParams {
    /// The entity ID to insert components into
    #[to_metadata]
    pub entity:     u64,
    /// Object containing component data to insert. Keys are component types, values are component
    /// data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w],
    /// not objects with named fields.
    #[to_metadata]
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct InsertResourceParams {
    /// The fully-qualified type name of the resource to insert or update
    #[to_metadata]
    pub resource: String,
    /// The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    #[to_metadata]
    pub value:    serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListParams {
    /// Optional entity ID to list components for - to list all types, do not pass entity parameter
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub entity: Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ListResourcesParams {
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct MutateComponentParams {
    /// The entity ID containing the component to mutate
    #[to_metadata]
    pub entity:    u64,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    #[to_metadata]
    pub value:     serde_json::Value,
    /// The fully-qualified type name of the component to mutate
    #[to_metadata]
    pub component: String,
    /// The path to the field within the component (e.g., 'translation.x')
    #[to_metadata]
    pub path:      String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:      u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct MutateResourceParams {
    /// The fully-qualified type name of the resource to mutate
    #[to_metadata]
    pub resource: String,
    /// The path to the field within the resource (e.g., 'settings.volume')
    #[to_metadata]
    pub path:     String,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    #[to_metadata]
    pub value:    serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Properties: components (array), option
    /// (array), has (array)
    #[to_metadata]
    pub data:   serde_json::Value,
    /// Object specifying which entities to query. Properties: with (array), without (array)
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub filter: Option<serde_json::Value>,
    /// If true, returns error on unknown component types (default: false)
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub strict: Option<bool>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RegistrySchemaParams {
    /// Include only types from these crates (e.g., [`bevy_transform`, `my_game`])
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub with_crates:    Option<Vec<String>>,
    /// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub without_crates: Option<Vec<String>>,
    /// Include only types with these reflect traits (e.g., [`Component`, `Resource`])
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub with_types:     Option<Vec<String>>,
    /// Exclude types with these reflect traits (e.g., [`RenderResource`])
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub without_types:  Option<Vec<String>>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:           u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RemoveParams {
    /// The entity ID to remove components from
    #[to_metadata]
    pub entity:     u64,
    /// Array of component type names to remove
    #[to_metadata]
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RemoveResourceParams {
    /// The fully-qualified type name of the resource to remove
    #[to_metadata]
    pub resource: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ReparentParams {
    /// Array of entity IDs to reparent
    #[to_metadata]
    pub entities: Vec<u64>,
    /// The new parent entity ID (omit to remove parent)
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub parent:   Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct ScreenshotParams {
    /// File path where the screenshot should be saved
    #[to_metadata]
    pub path: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SendKeysParams {
    /// Array of key code names to send
    #[to_metadata]
    pub keys:        Vec<String>,
    /// Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1
    /// minute)
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub duration_ms: Option<u32>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:        u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SetDebugModeParams {
    /// Enable or disable debug mode for `bevy_brp_extras` plugin
    #[to_metadata]
    pub enabled: bool,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:    u16,
}

#[derive(Deserialize, Serialize, JsonSchema, bevy_brp_mcp_macros::FieldPlacement)]
pub struct SpawnParams {
    /// Object containing component data to spawn with. Keys are component types, values are
    /// component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat:
    /// [x,y,z,w], not objects with named fields.
    #[serde(default)]
    #[to_metadata(skip_if_none)]
    pub components: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    #[to_call_info]
    pub port:       u16,
}
