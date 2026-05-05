use std::borrow::Cow;

use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use schemars::Schema;
use schemars::SchemaGenerator;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::tool_name::CallInfo;

/// Wrapper for Value that produces an empty object schema `{}` instead of `true` or specific types.
/// This ensures compatibility with strict JSON Schema validators (like Gemini's).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct AnySchemaValue(pub(super) Value);

impl JsonSchema for AnySchemaValue {
    fn schema_name() -> Cow<'static, str> { "AnySchemaValue".into() }

    #[allow(
        clippy::expect_used,
        reason = "empty JSON object deserialization is infallible"
    )]
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        serde_json::from_value(json!({}))
            .expect("Serializing empty JSON object to Schema should always succeed")
    }
}

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(super) struct ToolCallJsonResponse {
    pub(super) status:                ResponseStatus,
    pub(super) message:               String,
    pub(super) call_info:             CallInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) metadata:              Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parameters:            Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) result:                Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error_info:            Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) brp_extras_debug_info: Option<AnySchemaValue>,
}

impl ToolCallJsonResponse {
    /// Creates a `CallToolResult` from this `JsonResponse`
    pub(super) fn to_call_tool_result(&self) -> CallToolResult {
        // Convert self to Value
        let value = serde_json::to_value(self).unwrap_or_else(|e| {
            serde_json::json!({
                "status": "error",
                "message": format!("Failed to serialize response: {e}"),
                "call_info": self.call_info
            })
        });

        match self.status {
            ResponseStatus::Success => CallToolResult::structured(value),
            ResponseStatus::Error => CallToolResult::structured_error(value),
        }
    }
}

/// Response status types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(super) enum ResponseStatus {
    Success,
    Error,
}
