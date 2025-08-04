//! `bevy/mutate_component` tool - Mutate component fields

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::brp_tools::{FormatCorrectionStatus, Port};

/// Parameters for the `bevy/mutate_component` tool
#[derive(Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct MutateComponentParams {
    /// The entity ID containing the component to mutate
    pub entity: u64,

    /// The fully-qualified type name of the component to mutate
    pub component: String,

    /// The new value for the field. Note: Math types use array format - Vec2: [x,y], Vec3:
    /// [x,y,z], Vec4/Quat: [x,y,z,w], not objects with named fields.
    pub value: Value,

    /// The path to the field within the component (e.g., 'translation.x')
    #[serde(default)]
    pub path: String,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `bevy/mutate_component` tool
#[derive(Serialize, ResultStruct)]
#[brp_result(format_discovery = true)]
pub struct MutateComponentResult {
    /// The raw BRP response data (empty for mutate)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Format corrections applied during execution
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrections: Option<Vec<serde_json::Value>>,

    /// Whether format discovery was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub format_corrected: Option<FormatCorrectionStatus>,

    /// Warning message when format corrections were applied
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_metadata(skip_if_none)]
    pub warning: Option<String>,

    /// Message template for formatting responses
    #[to_message(message_template = "Mutated {component} for entity {entity}")]
    pub message_template: String,
}
