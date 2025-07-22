/// Parameter structs for BRP tools
///
/// These are strongly-typed parameter structs used by the BRP tools.
/// Each struct corresponds to a tool with the same name (minus the Params suffix).
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::constants::default_port;

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct DestroyParams {
    /// The entity ID to destroy
    pub entity: u64,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct DiscoverFormatParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:  u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GetParams {
    /// The entity ID to get component data from
    pub entity:     u64,
    /// Array of component types to retrieve. Each component must be a fully-qualified type name
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct GetResourceParams {
    /// The fully-qualified type name of the resource to get
    pub resource: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct InsertParams {
    /// The entity ID to insert components into
    pub entity:     u64,
    /// Object containing component data to insert. Keys are component types, values are component
    /// data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat: [x,y,z,w],
    /// not objects with named fields.
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct InsertResourceParams {
    /// The fully-qualified type name of the resource to insert or update
    pub resource: String,
    /// The resource value to insert. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value:    serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ListParams {
    /// Optional entity ID to list components for - to list all types, do not pass entity parameter
    #[serde(default)]
    pub entity: Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ListResourcesParams {
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct MutateComponentParams {
    /// The entity ID containing the component to mutate
    pub entity:    u64,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value:     serde_json::Value,
    /// The fully-qualified type name of the component to mutate
    pub component: String,
    /// The path to the field within the component (e.g., 'translation.x')
    pub path:      String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:      u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct MutateResourceParams {
    /// The fully-qualified type name of the resource to mutate
    pub resource: String,
    /// The path to the field within the resource (e.g., 'settings.volume')
    pub path:     String,
    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value:    serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Properties: components (array), option
    /// (array), has (array)
    pub data:   serde_json::Value,
    /// Object specifying which entities to query. Properties: with (array), without (array)
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
    /// If true, returns error on unknown component types (default: false)
    #[serde(default)]
    pub strict: Option<bool>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:   u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RegistrySchemaParams {
    /// Include only types from these crates (e.g., [`bevy_transform`, `my_game`])
    #[serde(default)]
    pub with_crates:    Option<Vec<String>>,
    /// Exclude types from these crates (e.g., [`bevy_render`, `bevy_pbr`])
    #[serde(default)]
    pub without_crates: Option<Vec<String>>,
    /// Include only types with these reflect traits (e.g., [`Component`, `Resource`])
    #[serde(default)]
    pub with_types:     Option<Vec<String>>,
    /// Exclude types with these reflect traits (e.g., [`RenderResource`])
    #[serde(default)]
    pub without_types:  Option<Vec<String>>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:           u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RemoveParams {
    /// The entity ID to remove components from
    pub entity:     u64,
    /// Array of component type names to remove
    pub components: serde_json::Value,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:       u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RemoveResourceParams {
    /// The fully-qualified type name of the resource to remove
    pub resource: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ReparentParams {
    /// Array of entity IDs to reparent
    pub entities: Vec<u64>,
    /// The new parent entity ID (omit to remove parent)
    #[serde(default)]
    pub parent:   Option<u64>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:     u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct RpcDiscoverParams {
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct ScreenshotParams {
    /// File path where the screenshot should be saved
    pub path: String,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port: u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SendKeysParams {
    /// Array of key code names to send
    pub keys:        Vec<String>,
    /// Duration in milliseconds to hold the keys before releasing (default: 100ms, max: 60000ms/1
    /// minute)
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:        u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SetDebugModeParams {
    /// Enable or disable debug mode for `bevy_brp_extras` plugin
    pub enabled: bool,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:    u16,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct SpawnParams {
    /// Object containing component data to spawn with. Keys are component types, values are
    /// component data. Note: Math types use array format - Vec2: [x,y], Vec3: [x,y,z], Vec4/Quat:
    /// [x,y,z,w], not objects with named fields.
    #[serde(default)]
    pub components: Option<serde_json::Value>,
    /// The BRP port (default: 15702)
    #[serde(
        default = "default_port",
        deserialize_with = "crate::tool::deserialize_port"
    )]
    pub port:       u16,
}
