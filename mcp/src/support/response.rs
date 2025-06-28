use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app_tools::support::scanning::extract_workspace_name;
use crate::brp_tools::brp_set_debug_mode::is_debug_enabled;
use crate::error::{Error, Result};

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub status:                ResponseStatus,
    pub message:               String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:                  Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brp_mcp_debug_info:    Option<Value>,
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
}

/// Builder for constructing JSON responses
pub struct ResponseBuilder {
    status:                ResponseStatus,
    message:               String,
    data:                  Option<Value>,
    brp_mcp_debug_info:    Option<Value>,
    brp_extras_debug_info: Option<Value>,
}

impl ResponseBuilder {
    pub const fn success() -> Self {
        Self {
            status:                ResponseStatus::Success,
            message:               String::new(),
            data:                  None,
            brp_mcp_debug_info:    None,
            brp_extras_debug_info: None,
        }
    }

    pub const fn error() -> Self {
        Self {
            status:                ResponseStatus::Error,
            message:               String::new(),
            data:                  None,
            brp_mcp_debug_info:    None,
            brp_extras_debug_info: None,
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn data(mut self, data: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        self.data = Some(serde_json::to_value(data).change_context(Error::General(
            "Failed to serialize response data".to_string(),
        ))?);
        Ok(self)
    }

    /// Add a field to the data object. Creates a new object if data is None.
    pub fn add_field(mut self, key: &str, value: impl Serialize) -> Result<Self> {
        use error_stack::ResultExt;

        let value_json = serde_json::to_value(value)
            .change_context(Error::General(format!("Failed to serialize field '{key}'")))?;

        if let Some(Value::Object(map)) = &mut self.data {
            map.insert(key.to_string(), value_json);
        } else {
            let mut map = serde_json::Map::new();
            map.insert(key.to_string(), value_json);
            self.data = Some(Value::Object(map));
        }

        Ok(self)
    }

    /// Auto-inject debug info if debug mode is enabled
    /// This should be called before `build()` to ensure debug info is included when appropriate
    pub fn auto_inject_debug_info(
        mut self,
        brp_mcp_debug: Option<impl Serialize>,
        brp_extras_debug: Option<impl Serialize>,
    ) -> Self {
        // Only inject BRP MCP debug info if MCP debug mode is enabled and debug info is provided
        if is_debug_enabled() {
            if let Some(debug_info) = brp_mcp_debug {
                if let Ok(serialized) = serde_json::to_value(&debug_info) {
                    self.brp_mcp_debug_info = Some(serialized);
                }
            }
        }

        // Always inject BRP extras debug info if it's provided (means extras debug is enabled)
        if let Some(debug_info) = brp_extras_debug {
            if let Ok(serialized) = serde_json::to_value(&debug_info) {
                self.brp_extras_debug_info = Some(serialized);
            }
        }

        self
    }

    pub fn build(self) -> JsonResponse {
        JsonResponse {
            status:                self.status,
            message:               self.message,
            data:                  self.data,
            brp_mcp_debug_info:    self.brp_mcp_debug_info,
            brp_extras_debug_info: self.brp_extras_debug_info,
        }
    }
}

/// Add workspace information to response data if available
pub fn add_workspace_info_to_response(
    response_data: &mut serde_json::Value,
    workspace_root: Option<&PathBuf>,
) {
    if let Some(root) = workspace_root {
        if let Some(workspace_name) = extract_workspace_name(root) {
            response_data["workspace"] = serde_json::Value::String(workspace_name);
        }
    }
}
