use rmcp::model::{CallToolResult, Content};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::tool::FieldPlacement;

/// Standard JSON response structure for all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub status:                ResponseStatus,
    pub message:               String,
    pub call_info:             CallInfo,
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

/// Call information for tracking tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CallInfo {
    /// Local tool execution (no BRP involved, no port)
    Local {
        /// The MCP tool name (e.g., "`brp_status`")
        mcp_tool: String,
    },
    /// Local tool with port (no BRP involved, but uses port)
    LocalWithPort {
        /// The MCP tool name (e.g., "`brp_launch_bevy_app`")
        mcp_tool: String,
        /// The port number
        port:     u16,
    },
    /// BRP tool execution (calls Bevy Remote Protocol)
    Brp {
        /// The MCP tool name (e.g., "`bevy_spawn`")
        mcp_tool:   String,
        /// The BRP method name (e.g., "bevy/spawn")
        brp_method: String,
        /// The BRP port number
        port:       u16,
    },
}

impl CallInfo {
    /// Create `CallInfo` for a local tool
    pub const fn local(mcp_tool: String) -> Self {
        Self::Local { mcp_tool }
    }

    /// Create `CallInfo` for a local tool with port
    pub const fn local_with_port(mcp_tool: String, port: u16) -> Self {
        Self::LocalWithPort { mcp_tool, port }
    }

    /// Create `CallInfo` for a BRP tool
    pub const fn brp(mcp_tool: String, brp_method: String, port: u16) -> Self {
        Self::Brp {
            mcp_tool,
            brp_method,
            port,
        }
    }
}

/// Trait for types that can provide `CallInfo` data
pub trait CallInfoProvider: Send + Sync {
    /// Convert this provider into a `CallInfo` instance
    fn to_call_info(&self, tool_name: String) -> CallInfo;
}

/// Marker type for local tools without port
pub struct LocalCallInfo;

/// Marker type for local tools with port
pub struct LocalWithPortCallInfo {
    pub port: u16,
}

/// Marker type for BRP tools
pub struct BrpCallInfo {
    pub method: &'static str,
    pub port:   u16,
}

impl CallInfoProvider for LocalCallInfo {
    fn to_call_info(&self, tool_name: String) -> CallInfo {
        CallInfo::local(tool_name)
    }
}

impl CallInfoProvider for LocalWithPortCallInfo {
    fn to_call_info(&self, tool_name: String) -> CallInfo {
        CallInfo::local_with_port(tool_name, self.port)
    }
}

impl CallInfoProvider for BrpCallInfo {
    fn to_call_info(&self, tool_name: String) -> CallInfo {
        CallInfo::brp(tool_name, self.method.to_string(), self.port)
    }
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
#[derive(Clone)]
pub struct ResponseBuilder {
    status:                ResponseStatus,
    message:               String,
    call_info:             CallInfo,
    metadata:              Option<Value>,
    result:                Option<Value>,
    brp_extras_debug_info: Option<Value>,
}

impl ResponseBuilder {
    /// Create a success response with call info pre-populated
    pub const fn success(call_info: CallInfo) -> Self {
        Self {
            status: ResponseStatus::Success,
            message: String::new(),
            call_info,
            metadata: None,
            result: None,
            brp_extras_debug_info: None,
        }
    }

    /// Create an error response with call info pre-populated
    pub const fn error(call_info: CallInfo) -> Self {
        Self {
            status: ResponseStatus::Error,
            message: String::new(),
            call_info,
            metadata: None,
            result: None,
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

    /// Add multiple fields from an optional JSON object to metadata
    /// Useful for adding error details or other optional metadata
    pub fn add_optional_details(self, details: Option<&serde_json::Value>) -> Self {
        match details {
            Some(Value::Object(map)) => {
                map.iter()
                    .filter(|(_, v)| !v.is_null())
                    .fold(self, |builder, (key, value)| {
                        builder.clone().add_field(key, value).unwrap_or_else(|_| {
                            tracing::warn!("Failed to add detail field '{}'", key);
                            builder // Keep the original builder if add_field fails
                        })
                    })
            }
            _ => self,
        }
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

    pub fn build(self) -> JsonResponse {
        let response = JsonResponse {
            status:                self.status,
            message:               self.message,
            call_info:             self.call_info,
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

    /// Get metadata for template substitution
    pub const fn metadata(&self) -> Option<&Value> {
        self.metadata.as_ref()
    }

    /// Get result for template substitution
    pub const fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }
}
