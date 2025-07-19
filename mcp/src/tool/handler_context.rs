use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolRequestParam;
use serde_json::{Value, json};

use crate::field_extraction::{
    ExtractedValue, JsonFieldProvider, ParameterFieldType, ParameterName, extract_parameter_field,
};
use crate::response::CallInfo;
use crate::tool::ToolDef;

/// Wrapper for request arguments to implement `JsonFieldProvider`
struct RequestArguments<'a> {
    args: &'a Option<serde_json::Map<String, Value>>,
}

impl JsonFieldProvider for RequestArguments<'_> {
    fn get_root(&self) -> Value {
        self.args
            .as_ref()
            .map_or(Value::Null, |map| Value::Object(map.clone()))
    }
}

/// Capability types that hold data for compile-time access control
/// Capability type indicating no port is available
#[derive(Clone)]
pub struct NoPort;

/// Capability type indicating a port is available with stored data
#[derive(Clone)]
pub struct HasPort {
    pub port: u16,
}

/// Capability type indicating no method is available
#[derive(Clone)]
pub struct NoMethod;

/// Capability type indicating a method is available with stored data
#[derive(Clone)]
pub struct HasMethod {
    pub method: String,
}

/// Trait for `HandlerContext` types that can provide `CallInfo`
pub trait HasCallInfo {
    fn call_info(&self) -> CallInfo;
}

/// Context passed to all handlers containing service, request, and MCP context
#[derive(Clone)]
pub struct HandlerContext<Port = NoPort, Method = NoMethod> {
    pub(super) tool_def: ToolDef,
    pub request:         CallToolRequestParam,
    pub roots:           Vec<PathBuf>,
    // Store capability types directly - they contain the actual data
    port_capability:     Port,
    method_capability:   Method,
}

// Note: HandlerContext now uses capability-based types directly via HandlerContext::with_data()

impl<Port, Method> HandlerContext<Port, Method> {
    /// Create a new `HandlerContext` with specific capabilities
    pub(crate) const fn with_data(
        tool_def: ToolDef,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
        port_capability: Port,
        method_capability: Method,
    ) -> Self {
        Self {
            tool_def,
            request,
            roots,
            port_capability,
            method_capability,
        }
    }
}

// Common methods available for all HandlerContext types
impl<Port, Method> HandlerContext<Port, Method> {
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

    /// Extract a required string parameter with error handling
    pub fn extract_required_string(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<&str, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        self.extract_optional_named_field(field_name)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                        "Missing {field_description} parameter"
                    )))
                    .attach_printable(format!("Field name: {field_name}"))
                    .attach_printable("Expected: string value"),
                )
            })
    }

    /// Extract an optional path parameter
    /// Returns None if not provided or empty string
    pub fn extract_optional_path(&self) -> Option<String> {
        let path = self
            .extract_with_default(ParameterName::Path, "")
            .into_string()
            .unwrap_or_default();
        if path.is_empty() { None } else { Some(path) }
    }

    // Note: extract_method_param() and extract_port() now available on all HandlerContext types

    // ============================================================================
    // UNIFIED EXTRACTION API
    // ============================================================================

    /// Extract a parameter using its built-in type information
    pub fn extract(&self, name: ParameterName) -> Option<ExtractedValue> {
        // Create provider wrapper for request arguments
        let provider = RequestArguments {
            args: &self.request.arguments,
        };

        // Use the new unified extraction system
        extract_parameter_field(&provider, name)
    }

    /// Extract a required parameter, returning error if missing or invalid
    pub fn extract_required(&self, name: ParameterName) -> Result<ExtractedValue, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        let field_name: &str = name.into();

        self.extract(name).ok_or_else(|| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                    "Missing required parameter '{field_name}'"
                )))
                .attach_printable(format!("Parameter name: {field_name}"))
                .attach_printable("Required parameter not provided"),
            )
        })
    }

    /// Extract a parameter with a default value if not present
    pub fn extract_with_default<T: Into<ExtractedValue>>(
        &self,
        name: ParameterName,
        default: T,
    ) -> ExtractedValue {
        self.extract(name).unwrap_or_else(|| default.into())
    }
}

// Capability-based method access - Port access only available when Port = HasPort
impl<Method> HandlerContext<HasPort, Method> {
    /// Get the port number - only available when port capability is present
    pub const fn port(&self) -> u16 {
        self.port_capability.port // Direct access to data in HasPort
    }
}

// Capability-based method access - Method access only available when Method = HasMethod (i.e., BRP
// method calls)
impl<Port> HandlerContext<Port, HasMethod> {
    /// Get the BRP method name - only available when method capability is present
    pub fn brp_method(&self) -> &str {
        &self.method_capability.method // Direct access to data in HasMethod
    }

    /// Extract brp method parameters from tool definition
    pub fn extract_params_from_definition(&self) -> Result<Option<Value>, McpError> {
        use std::str::FromStr;

        use crate::error::{Error as ServiceError, report_to_mcp_error};

        // Build params from parameter definitions
        let mut params_obj = serde_json::Map::new();
        let mut has_params = false;

        for param in self.tool_def.parameters() {
            // Parse parameter name string to ParameterName enum for type-safe extraction
            let param_name = ParameterName::from_str(param.name()).map_err(|_| {
                report_to_mcp_error(
                    &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                        "Unknown parameter name: {}",
                        param.name()
                    )))
                    .attach_printable("Parameter name not found in ParameterName enum"),
                )
            })?;

            // Extract parameter value using unified API
            let extracted_value = if param.required() {
                Some(self.extract_required(param_name)?)
            } else {
                self.extract(param_name)
            };

            // Convert ExtractedValue to JSON
            let json_value = if let Some(extracted) = extracted_value {
                match param_name.param_type() {
                    ParameterFieldType::Number => Some(json!(extracted.into_u64()?)),
                    ParameterFieldType::String => Some(json!(extracted.into_string()?)),
                    ParameterFieldType::Boolean => Some(json!(extracted.into_bool()?)),
                    ParameterFieldType::StringArray => Some(json!(extracted.into_string_array()?)),
                    ParameterFieldType::NumberArray => Some(json!(extracted.into_number_array()?)),
                    ParameterFieldType::Any => Some(json!(extracted.into_any()?)),
                    ParameterFieldType::DynamicParams => {
                        // For dynamic params, return the value directly
                        return Ok(Some(extracted.into_any()?));
                    } /* Note: Count and LineSplit are not available in ParameterFieldType - type
                       * safety achieved! */
                }
            } else {
                None
            };

            // Add to params if value exists
            if let Some(val) = json_value {
                params_obj.insert(param.name().to_string(), val);
                has_params = true;
            }
        }

        // Return params
        let params = if has_params {
            Some(Value::Object(params_obj))
        } else {
            None
        };

        Ok(params)
    }
}

// HasCallInfo implementations for each capability combination

// Local tools (no port, no method)
impl HasCallInfo for HandlerContext<NoPort, NoMethod> {
    fn call_info(&self) -> CallInfo {
        CallInfo::local(self.request.name.to_string())
    }
}

// Local tools with port (has port, no method)
impl HasCallInfo for HandlerContext<HasPort, NoMethod> {
    fn call_info(&self) -> CallInfo {
        CallInfo::local_with_port(self.request.name.to_string(), self.port())
    }
}

// BRP tools (has port and method)
impl HasCallInfo for HandlerContext<HasPort, HasMethod> {
    fn call_info(&self) -> CallInfo {
        CallInfo::brp(
            self.request.name.to_string(),
            self.brp_method().to_string(),
            self.port(),
        )
    }
}
