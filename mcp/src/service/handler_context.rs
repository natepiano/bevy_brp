use std::path::PathBuf;

use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;
use serde_json::{Value, json};

use crate::response::CallInfo;
use crate::tool::{ParamType, ToolDef};

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

    /// Extract an optional number parameter with default
    pub fn extract_optional_number(&self, field_name: &str, default: u64) -> u64 {
        self.extract_optional_named_field(field_name)
            .and_then(Value::as_u64)
            .unwrap_or(default)
    }

    /// Extract an optional string array parameter
    pub fn extract_optional_string_array(&self, field_name: &str) -> Option<Vec<String>> {
        self.extract_optional_named_field(field_name)?
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            })
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

    /// Extract a required u64 parameter with error handling
    pub fn extract_required_u64(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<u64, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        self.extract_optional_named_field(field_name)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                        "Missing {field_description} parameter"
                    )))
                    .attach_printable(format!("Field name: {field_name}"))
                    .attach_printable("Expected: u64 number"),
                )
            })
    }

    /// Extract a required u32 parameter with error handling
    pub fn extract_required_u32(
        &self,
        field_name: &str,
        field_description: &str,
    ) -> Result<u32, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        let value = self.extract_required_u64(field_name, field_description)?;
        u32::try_from(value).map_err(|_| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                    "Invalid {field_description} value"
                )))
                .attach_printable(format!("Field name: {field_name}"))
                .attach_printable("Value too large for u32"),
            )
        })
    }

    /// Extract an optional string parameter with a default value
    pub fn extract_optional_string(&self, param_name: &str, default: &str) -> String {
        self.extract_optional_named_field(param_name)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }

    /// Extract an optional bool parameter with a default value
    pub fn extract_optional_bool(&self, param_name: &str, default: bool) -> bool {
        self.extract_optional_named_field(param_name)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    }

    /// Extract an optional u32 parameter with a default value
    pub fn extract_optional_u32(&self, param_name: &str, default: u32) -> Result<u32, McpError> {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        let value = self
            .extract_optional_named_field(param_name)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| u64::from(default));

        u32::try_from(value).map_err(|_| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                    "Invalid parameter '{param_name}'"
                )))
                .attach_printable(format!("Parameter name: {param_name}"))
                .attach_printable("Value too large for u32"),
            )
        })
    }

    /// Extract an optional path parameter
    /// Returns None if not provided or empty string
    pub fn extract_optional_path(&self) -> Option<String> {
        use crate::constants::PARAM_PATH;
        let path = self.extract_optional_string(PARAM_PATH, "");
        if path.is_empty() { None } else { Some(path) }
    }

    // Note: extract_method_param() and extract_port() now available on all HandlerContext types

    /// Generic helper for extracting typed parameters with unified required/optional logic
    ///
    /// This method encapsulates the common pattern of:
    /// - If required: extract and validate, return error if missing/invalid
    /// - If optional: extract if present, return None if missing
    ///
    /// The extractor function should return Some(value) for valid data, None for missing/invalid
    pub fn extract_typed_param<F, V>(
        &self,
        param_name: &str,
        param_description: &str,
        required: bool,
        extractor: F,
    ) -> Result<Option<V>, McpError>
    where
        F: Fn(&Value) -> Option<V>,
    {
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        self.extract_optional_named_field(param_name).map_or_else(
            || {
                if required {
                    Err(report_to_mcp_error(
                        &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                            "Missing {param_description} parameter"
                        )))
                        .attach_printable(format!("Field name: {param_name}"))
                        .attach_printable("Required parameter not provided"),
                    ))
                } else {
                    Ok(None)
                }
            },
            |value| {
                extractor(value).map_or_else(
                    || {
                        if required {
                            Err(report_to_mcp_error(
                                &error_stack::Report::new(ServiceError::InvalidArgument(format!(
                                    "Invalid {param_description} parameter"
                                )))
                                .attach_printable(format!("Field name: {param_name}"))
                                .attach_printable("Value present but invalid type/format"),
                            ))
                        } else {
                            Ok(None)
                        }
                    },
                    |extracted| Ok(Some(extracted)),
                )
            },
        )
    }
}

// Capability-based method access - Port access only available when Port = HasPort
impl<Method> HandlerContext<HasPort, Method> {
    /// Get the port number - only available when port capability is present
    pub const fn port(&self) -> u16 {
        self.port_capability.port // Direct access to data in HasPort
    }
}

// Capability-based method access - Method access only available when Method = HasMethod
impl<Port> HandlerContext<Port, HasMethod> {
    /// Get the BRP method name - only available when method capability is present
    pub fn brp_method(&self) -> &str {
        &self.method_capability.method // Direct access to data in HasMethod
    }

    /// Extract brp method parameters from tool definition
    pub fn extract_params_from_definition(&self) -> Result<Option<Value>, McpError> {
        // Build params from parameter definitions
        let mut params_obj = serde_json::Map::new();
        let mut has_params = false;

        for param in self.tool_def.parameters() {
            // Extract parameter value based on type
            let value = match param.param_type() {
                ParamType::Number => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        Value::as_u64,
                    )?
                    .map(|v| json!(v)),
                ParamType::String => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| v.as_str().map(std::string::ToString::to_string),
                    )?
                    .map(|s| json!(s)),
                ParamType::Boolean => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        Value::as_bool,
                    )?
                    .map(|b| json!(b)),
                ParamType::StringArray => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| {
                            v.as_array().and_then(|arr| {
                                arr.iter()
                                    .map(|item| item.as_str())
                                    .collect::<Option<Vec<_>>>()
                                    .map(|strings| {
                                        strings
                                            .into_iter()
                                            .map(String::from)
                                            .collect::<Vec<String>>()
                                    })
                            })
                        },
                    )?
                    .map(|array| json!(array)),
                ParamType::NumberArray => self
                    .extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| {
                            v.as_array().and_then(|arr| {
                                arr.iter().map(Value::as_u64).collect::<Option<Vec<_>>>()
                            })
                        },
                    )?
                    .map(|array| json!(array)),
                ParamType::Any => self.extract_typed_param(
                    param.name(),
                    param.description(),
                    param.required(),
                    |v| Some(v.clone()),
                )?,
                ParamType::DynamicParams => {
                    // For dynamic params, extract the value and return it directly as BRP
                    // parameters
                    let dynamic_value = self.extract_typed_param(
                        param.name(),
                        param.description(),
                        param.required(),
                        |v| Some(v.clone()),
                    )?;
                    return Ok(dynamic_value);
                }
            };

            // Add to params if value exists
            if let Some(val) = value {
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
