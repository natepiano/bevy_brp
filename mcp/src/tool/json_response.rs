//! `JsonResponse` and conversion methods
use rmcp::model::CallToolResult;
use rmcp::model::Content;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::tool_name::CallInfo;
use crate::error::Error;
use crate::error::Result;

use serde_json::json;

fn schema_for_any_object(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    serde_json::from_value(json!({})).unwrap()
}

fn schema_for_any_value(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    serde_json::from_value(json!({})).unwrap()
}

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolCallJsonResponse {
    pub status:                ResponseStatus,
    pub message:               String,
    pub call_info:             CallInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "schema_for_any_object")]
    pub metadata:              Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "schema_for_any_object")]
    pub parameters:            Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "schema_for_any_value")]
    pub result:                Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "schema_for_any_object")]
    pub error_info:            Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(schema_with = "schema_for_any_object")]
    pub brp_extras_debug_info: Option<Value>,
}

impl ToolCallJsonResponse {
    /// Convert to JSON string with error-stack context
    /// Uses `PrettyCompactFormatter` for readable structure with compact arrays
    pub fn to_json(&self) -> Result<String> {
        use error_stack::ResultExt;
        use json_pretty_compact::PrettyCompactFormatter;
        use serde::Serialize;
        use serde_json::Serializer;

        let mut buf = Vec::new();
        let formatter = PrettyCompactFormatter::new();
        let mut ser = Serializer::with_formatter(&mut buf, formatter);

        self.serialize(&mut ser)
            .map_err(|e| Error::General(format!("Failed to serialize JSON response: {e}")))?;

        String::from_utf8(buf).change_context(Error::General(
            "Failed to convert JSON bytes to string".to_string(),
        ))
    }

    /// Convert to JSON string with fallback on error
    pub fn to_json_fallback(&self) -> String {
        self.to_json().unwrap_or_else(|_| {
            r#"{"status":"error","message":"Failed to serialize response"}"#.to_string()
        })
    }

    /// Creates a `CallToolResult` from this `JsonResponse`
    pub fn to_call_tool_result(&self) -> CallToolResult {
        CallToolResult::success(vec![Content::text(self.to_json_fallback())])
    }
}

/// Response status types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Success,
    Error,
}
