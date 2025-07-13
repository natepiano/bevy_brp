use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};
use serde_json::Value;

use super::mcp_service::McpService;
use crate::response::CallInfo;
use crate::tool::McpToolDef;

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

impl HandlerContext {
    pub const fn new(
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Self {
        Self {
            service,
            request,
            context,
            handler_data: (),
        }
    }
}

impl<T> HandlerContext<T> {
    /// Create a new `HandlerContext` with specific handler data
    pub const fn with_data(
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
    pub fn tool_def(&self) -> Result<&McpToolDef, McpError> {
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

    /// Extract port from request arguments, defaulting to `DEFAULT_BRP_PORT`
    pub fn extract_port_param(&self) -> u16 {
        use crate::constants::{DEFAULT_BRP_PORT, PARAM_PORT};

        self.request
            .arguments
            .as_ref()
            .and_then(|args| args.get(PARAM_PORT))
            .and_then(serde_json::Value::as_u64)
            .and_then(|p| u16::try_from(p).ok())
            .unwrap_or(DEFAULT_BRP_PORT)
    }

    /// Extract method from request arguments
    ///
    /// # Errors
    ///
    /// Returns an error if the method parameter is missing.
    pub fn extract_method_param(&self) -> Result<String, McpError> {
        use crate::constants::PARAM_METHOD;
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        self.request
            .arguments
            .as_ref()
            .and_then(|args| args.get(PARAM_METHOD))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| {
                report_to_mcp_error(
                    &error_stack::Report::new(ServiceError::InvalidArgument(
                        "Missing BRP method parameter".to_string(),
                    ))
                    .attach_printable("BrpExecute requires a method parameter"),
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

    /// Extract port parameter with default value
    pub fn extract_port(&self) -> Result<u16, McpError> {
        use crate::constants::{DEFAULT_BRP_PORT, PARAM_PORT};
        use crate::error::{Error as ServiceError, report_to_mcp_error};

        let port_u64 = self.extract_optional_number(PARAM_PORT, u64::from(DEFAULT_BRP_PORT));

        let port = u16::try_from(port_u64).map_err(|_| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(
                    "Invalid port parameter".to_string(),
                ))
                .attach_printable("Value too large for u16"),
            )
        })?;

        // Validate port range (1024-65535 for non-privileged ports)
        if port < 1024 {
            return Err(report_to_mcp_error(
                &error_stack::Report::new(ServiceError::InvalidArgument(
                    "Invalid port parameter".to_string(),
                ))
                .attach_printable("Port must be >= 1024 (non-privileged ports only)"),
            ));
        }

        Ok(port)
    }
}
