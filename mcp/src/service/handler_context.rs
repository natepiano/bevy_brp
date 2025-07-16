use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use super::mcp_service::McpService;
use crate::response::CallInfo;
use crate::tool::UnifiedToolDef;

/// Trait for `HandlerContext` types that can provide `CallInfo`
pub trait HasCallInfo {
    fn call_info(&self) -> CallInfo;
}

/// Context passed to all handlers containing service, request, and MCP context
#[derive(Clone)]
pub struct HandlerContext<T = ()> {
    pub service:             Arc<McpService>,
    pub request:             CallToolRequestParam,
    pub context:             RequestContext<RoleServer>,
    pub(super) handler_data: T,
}

// Note: Generic HandlerContext::new() removed - use HandlerContext<BaseContext>::new() instead

impl<T> HandlerContext<T> {
    /// Create a new `HandlerContext` with specific handler data
    pub(crate) const fn with_data(
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
        handler_data: T,
    ) -> Self {
        Self {
            service,
            request,
            context,
            handler_data,
        }
    }
}

// Common methods available for all HandlerContext types
impl<T> HandlerContext<T> {
    /// Get tool definition by looking up the request name in the service's tool registry
    ///
    /// # Errors
    ///
    /// Returns an error if the tool definition is not found.
    pub fn tool_def(&self) -> Result<&UnifiedToolDef, McpError> {
        self.service
            .get_tool_def(&self.request.name)
            .ok_or_else(|| {
                crate::error::report_to_mcp_error(
                    &error_stack::Report::new(crate::error::Error::InvalidArgument(format!(
                        "unknown tool: {}",
                        self.request.name
                    )))
                    .attach_printable("Tool not found"),
                )
            })
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

    // Note: extract_method_param() and extract_port() moved to HandlerContext<BaseContext>

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
