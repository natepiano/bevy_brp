use std::borrow::Cow;

use rmcp::model::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::tool_name::CallInfo;
use crate::error::Error;
use crate::error::Result;

/// Wrapper for Value that produces an empty object schema `{}` instead of `true` or specific types.
/// This ensures compatibility with strict JSON Schema validators (like Gemini's).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AnySchemaValue(pub Value);

impl JsonSchema for AnySchemaValue {
    fn schema_name() -> Cow<'static, str> {
        "AnySchemaValue".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        serde_json::from_value(json!({})).unwrap()
    }
}

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolCallJsonResponse {
    pub status:                ResponseStatus,
    pub message:               String,
    pub call_info:             CallInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata:              Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters:            Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:                Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_info:            Option<AnySchemaValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brp_extras_debug_info: Option<AnySchemaValue>,
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
        // Convert self to Value
        let value = serde_json::to_value(self).unwrap_or_else(|e| {
            serde_json::json!({
                "status": "error",
                "message": format!("Failed to serialize response: {}", e),
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
pub enum ResponseStatus {
    Success,
    Error,
}
