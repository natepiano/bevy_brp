use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::mcp_service::McpService;
use crate::constants::{DEFAULT_BRP_PORT, PARAM_METHOD, PARAM_PORT};
use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::response::CallInfo;
use crate::tool::{HandlerType, LocalToolFunction, McpToolDef};

/// Trait for `HandlerContext` types that can provide `CallInfo`
pub trait HasCallInfo {
    fn call_info(&self) -> CallInfo;
}

/// Data type for local handler contexts (carries the extracted handler)
#[derive(Clone)]
pub struct LocalContext {
    pub handler: Arc<dyn LocalToolFunction>,
}

/// Data type for BRP handler contexts (carries extracted request data)
#[derive(Clone)]
pub struct BrpContext {
    pub method: String,
    pub port:   u16,
}

/// Typed context variants for pattern matching
pub enum TypedContext {
    Local(HandlerContext<LocalContext>),
    Brp(HandlerContext<BrpContext>),
}

/// Context passed to all handlers containing service, request, and MCP context
#[derive(Clone)]
pub struct HandlerContext<T = ()> {
    pub service:  Arc<McpService>,
    pub request:  CallToolRequestParam,
    pub context:  RequestContext<RoleServer>,
    handler_data: T,
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
}

// Helper functions for extracting parameters
fn extract_port_from_request(request: &CallToolRequestParam) -> u16 {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(PARAM_PORT))
        .and_then(serde_json::Value::as_u64)
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(DEFAULT_BRP_PORT)
}

fn extract_method_from_request(request: &CallToolRequestParam) -> Result<String, McpError> {
    request
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

// Conversion method for creating typed contexts
impl HandlerContext {
    pub fn into_typed(self) -> Result<TypedContext, McpError> {
        let binding = self.clone();
        let tool_def = binding.tool_def()?;
        match &tool_def.handler {
            HandlerType::Local { handler } => Ok(TypedContext::Local(HandlerContext {
                service:      self.service,
                request:      self.request,
                context:      self.context,
                handler_data: LocalContext {
                    handler: handler.clone(),
                },
            })),
            HandlerType::Brp { method } => {
                let method_string = (*method).to_string();
                let port = extract_port_from_request(&self.request);
                Ok(TypedContext::Brp(HandlerContext {
                    service:      self.service,
                    request:      self.request,
                    context:      self.context,
                    handler_data: BrpContext {
                        method: method_string,
                        port,
                    },
                }))
            }
            HandlerType::BrpExecute => {
                let method = extract_method_from_request(&self.request)?;
                let port = extract_port_from_request(&self.request);
                Ok(TypedContext::Brp(HandlerContext {
                    service:      self.service,
                    request:      self.request,
                    context:      self.context,
                    handler_data: BrpContext { method, port },
                }))
            }
        }
    }
}

// Type-specific implementations
impl HandlerContext<LocalContext> {
    pub fn call_info(&self) -> CallInfo {
        CallInfo::local(self.request.name.to_string())
    }

    pub fn handler(&self) -> &Arc<dyn LocalToolFunction> {
        &self.handler_data.handler
    }
}

impl HasCallInfo for HandlerContext<LocalContext> {
    fn call_info(&self) -> CallInfo {
        self.call_info()
    }
}

impl HandlerContext<BrpContext> {
    pub fn brp_method(&self) -> &str {
        &self.handler_data.method
    }

    pub fn call_info(&self) -> CallInfo {
        CallInfo::brp(
            self.request.name.to_string(),
            self.handler_data.method.clone(),
            self.handler_data.port,
        )
    }
}

impl HasCallInfo for HandlerContext<BrpContext> {
    fn call_info(&self) -> CallInfo {
        self.call_info()
    }
}
