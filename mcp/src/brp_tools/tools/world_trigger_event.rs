//! `world.trigger_event` tool - Trigger events in the Bevy world

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::brp_tools::Port;

/// Parameters for the `world.trigger_event` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TriggerEventParams {
    /// The full type path of the event to trigger (e.g., "my_game::events::SpawnEnemy")
    pub event: String,

    /// The serialized value of the event payload, if any.
    /// For unit events (no data), omit this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `world.trigger_event` tool
///
/// Note: This follows the `DespawnEntityResult` pattern - the `{event}` placeholder
/// in the message template is resolved from `TriggerEventParams.event` at response-building
/// time, so we don't need an `event` field in this struct.
#[derive(Serialize, ResultStruct)]
#[brp_result]
pub struct TriggerEventResult {
    /// The raw BRP response (null on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[to_result(skip_if_none)]
    pub result: Option<Value>,

    /// Message template for formatting responses
    #[to_message(message_template = "Triggered event {event}")]
    pub message_template: String,
}
