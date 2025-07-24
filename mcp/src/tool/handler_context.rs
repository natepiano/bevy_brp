use std::path::PathBuf;

use rmcp::model::CallToolRequestParam;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::tool::ToolDef;

/// Context passed to all handlers containing service, request, and MCP context
#[derive(Clone)]
pub struct HandlerContext {
    pub(super) tool_def: ToolDef,
    pub request:         CallToolRequestParam,
    pub roots:           Vec<PathBuf>,
}

impl HandlerContext {
    /// Create a new `HandlerContext`
    pub(crate) const fn new(
        tool_def: ToolDef,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            tool_def,
            request,
            roots,
        }
    }

    /// Get tool definition by looking up the request name in the service's tool registry
    ///
    /// # Errors
    /// Returns an error if the tool definition is not found.
    pub const fn tool_def(&self) -> &ToolDef {
        &self.tool_def
    }

    // Common parameter extraction methods (used by both BRP and local handlers)

    /// Get a field value from the request arguments
    pub fn extract_optional_named_field(&self, field_name: &str) -> Option<&Value> {
        self.request.arguments.as_ref()?.get(field_name)
    }

    /// Extract typed parameters from request using serde deserialization
    pub fn extract_parameter_values<T>(&self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Get request arguments as JSON Value
        let args_value = self.request.arguments.as_ref().map_or_else(
            || serde_json::Value::Object(serde_json::Map::new()),
            |args| serde_json::Value::Object(args.clone()),
        );

        // Deserialize into target type
        serde_json::from_value(args_value).map_err(|e| {
            error_stack::Report::new(Error::ParameterExtraction(format!(
                "Failed to extract parameters for type: {}",
                std::any::type_name::<T>()
            )))
            .attach_printable("Parameter validation failed")
            .attach_printable(format!("Expected type: {}", std::any::type_name::<T>()))
        })
    }
}
