use rmcp::model::{CallToolResult, Content};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::response::FieldPlacement;

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub status:                ResponseStatus,
    pub message:               String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata:              Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result:                Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brp_extras_debug_info: Option<Value>,
}

/// Response status types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Success,
    Error,
}

impl JsonResponse {
    /// Convert to JSON string with error-stack context
    pub fn to_json(&self) -> Result<String> {
        use error_stack::ResultExt;

        serde_json::to_string_pretty(self).change_context(Error::General(
            "Failed to serialize JSON response".to_string(),
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

/// Builder for constructing JSON responses
pub struct ResponseBuilder {
    status:                ResponseStatus,
    message:               String,
    metadata:              Option<Value>,
    result:                Option<Value>,
    brp_extras_debug_info: Option<Value>,
}

impl ResponseBuilder {
    pub const fn success() -> Self {
        Self {
            status:                ResponseStatus::Success,
            message:               String::new(),
            metadata:              None,
            result:                None,
            brp_extras_debug_info: None,
        }
    }

    pub const fn error() -> Self {
        Self {
            status:                ResponseStatus::Error,
            message:               String::new(),
            metadata:              None,
            result:                None,
            brp_extras_debug_info: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Add a field to the metadata object. Creates a new object if metadata is None.
    pub fn add_field(mut self, key: &str, value: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        let value_json = serde_json::to_value(value)
            .change_context(Error::General(format!("Failed to serialize field '{key}'")))?;

        // Skip fields marked for nullable skipping
        if let Value::String(s) = &value_json {
            if s == "__SKIP_NULL_FIELD__" {
                return Ok(self);
            }
        }

        if let Some(Value::Object(map)) = &mut self.metadata {
            map.insert(key.to_string(), value_json);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value_json);
            self.metadata = Some(Value::Object(map));
        }

        Ok(self)
    }

    /// Set the result field directly for Raw responses
    pub fn with_result(mut self, result: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        let result_json = serde_json::to_value(result)
            .change_context(Error::General("Failed to serialize result".to_string()))?;

        self.result = Some(result_json);
        Ok(self)
    }

    /// Add a field to the metadata object (alias for clarity)
    pub fn add_metadata_field(self, key: &str, value: impl Serialize) -> Result<Self> {
        self.add_field(key, value)
    }

    /// Add a field to the specified location (metadata or result object)
    pub fn add_field_to(
        mut self,
        key: &str,
        value: impl Serialize,
        placement: FieldPlacement,
    ) -> Result<Self> {
        use error_stack::ResultExt;

        let value_json = serde_json::to_value(value)
            .change_context(Error::General(format!("Failed to serialize field '{key}'")))?;

        // Skip fields marked for nullable skipping
        if let Value::String(s) = &value_json {
            if s == "__SKIP_NULL_FIELD__" {
                return Ok(self);
            }
        }

        match placement {
            FieldPlacement::Metadata => {
                // For metadata, use field name as key in object
                if let Some(Value::Object(map)) = &mut self.metadata {
                    map.insert(key.to_string(), value_json);
                } else {
                    let mut map = serde_json::Map::new();
                    map.insert(key.to_string(), value_json);
                    self.metadata = Some(Value::Object(map));
                }
            }
            FieldPlacement::Result => {
                // For result, set the entire result field to the value
                // Field name is ignored to match raw BRP behavior
                self.result = Some(value_json);
            }
        }

        Ok(self)
    }

    /// Auto-inject debug info if debug mode is enabled
    /// This should be called before `build()` to ensure debug info is included when appropriate
    pub fn auto_inject_debug_info(mut self, brp_extras_debug: Option<impl Serialize>) -> Self {
        // Always inject BRP extras debug info if it's provided (means extras debug is enabled)
        if let Some(debug_info) = brp_extras_debug {
            if let Ok(serialized) = serde_json::to_value(&debug_info) {
                self.brp_extras_debug_info = Some(serialized);
            }
        }

        self
    }

    pub fn build(self) -> JsonResponse {
        let response = JsonResponse {
            status:                self.status,
            message:               self.message,
            metadata:              self.metadata,
            result:                self.result,
            brp_extras_debug_info: self.brp_extras_debug_info,
        };
        tracing::debug!(
            "ResponseBuilder::build - result field: {:?}",
            response.result
        );
        response
    }
}
