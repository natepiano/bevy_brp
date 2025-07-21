use std::path::PathBuf;

use rmcp::model::CallToolRequestParam;
use serde_json::Value;

use crate::response::CallInfo;
use crate::tool::ToolDef;

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

    // Note: extract_method_param() and extract_port() now available on all HandlerContext types

    /// Extract typed parameters from request using serde deserialization
    #[allow(dead_code)]
    pub fn extract_typed_params<T>(&self) -> crate::error::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::error::Error;

        // Get request arguments as JSON Value
        let args_value = self.request.arguments.as_ref().map_or_else(
            || serde_json::Value::Object(serde_json::Map::new()),
            |args| serde_json::Value::Object(args.clone()),
        );

        // Deserialize into target type
        serde_json::from_value(args_value).map_err(|e| {
            error_stack::Report::new(Error::InvalidArgument(format!(
                "Failed to parse parameters: {e}"
            )))
            .attach_printable("Parameter validation failed")
            .attach_printable(format!("Expected type: {}", std::any::type_name::<T>()))
        })
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
