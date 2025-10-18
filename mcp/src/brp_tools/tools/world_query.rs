//! `world.query` tool - Query entities by components

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Selector for optional components in a query (mirrors Bevy's `ComponentSelector`)
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComponentSelector {
    /// Select all components present on the entity
    All,
    /// Select specific components by their full type paths
    #[serde(untagged)]
    Paths(Vec<String>),
}

impl<'de> Deserialize<'de> for ComponentSelector {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::String(ref s) if s == "all" => Ok(Self::All),
            serde_json::Value::Array(arr) => {
                let paths = arr
                    .into_iter()
                    .map(|v| {
                        v.as_str()
                            .ok_or_else(|| {
                                Error::custom(
                                    "option array must contain only strings (component type paths)",
                                )
                            })
                            .map(String::from)
                    })
                    .collect::<core::result::Result<Vec<_>, _>>()?;
                Ok(Self::Paths(paths))
            }
            _ => Err(Error::custom(
                "option field must be either the string \"all\" or an array of component type \
                 paths like [\"bevy_transform::components::transform::Transform\"]",
            )),
        }
    }
}

impl Default for ComponentSelector {
    fn default() -> Self {
        Self::Paths(vec![])
    }
}

/// Query data specification - what component data to retrieve
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQuery {
    /// Required components - entities must have all of these
    #[serde(default)]
    pub components: Vec<String>,

    /// Optional components - retrieve if present. Can be "all" or array of paths
    #[serde(default)]
    pub option: ComponentSelector,

    /// Components to check for presence (returns boolean, not data)
    #[serde(default)]
    pub has: Vec<String>,
}

/// Query filter specification - which entities to include
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct BrpQueryFilter {
    /// Entities must have all of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,

    /// Entities must NOT have any of these components
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
}

/// Parameters for the `world.query` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct QueryParams {
    /// Object specifying what component data to retrieve. Required.
    /// Structure: {components: string[], option: "all" | string[], has: string[]}.
    /// Use {} to get entity IDs only without component data.
    pub data: BrpQuery,

    /// Object specifying which entities to query. Optional. Structure: {with: string[],
    /// without: string[]}. Defaults to {} (no filter) if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<BrpQueryFilter>,

    /// If true, returns error on unknown component types (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.query` tool
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct QueryResult {
    /// The raw BRP response - array of entities with their components
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Count of entities returned
    #[to_metadata(result_operation = "count")]
    pub entity_count: usize,

    /// Total count of components across all entities
    #[to_metadata(result_operation = "count_query_components")]
    pub component_count: usize,

    /// Message template for formatting responses
    #[to_message(message_template = "Found {entity_count} entities")]
    pub message_template: String,
}
