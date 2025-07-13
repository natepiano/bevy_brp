use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use super::{BrpToolHandler, LocalToolHandler};
use crate::service::{BrpContext, HandlerContext, LocalContext};

/// Determines how BRP method names are resolved
#[derive(Clone)]
pub enum BrpMethodSource {
    /// Static method name known at compile time
    Static { method: &'static str },
    /// Dynamic method name extracted from request at runtime
    Dynamic,
}

/// Trait for individual tool handler implementations
/// `#[async_trait]` allows us to use `ToolHandler` in `Box<dyn ToolHandler>` situations
#[async_trait]
pub trait ToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError>;
}

/// Type alias for the response from local handlers
///
/// Breaking down the type:
/// - `Pin<Box<...>>`: Heap-allocated Future that won't move in memory
/// - `dyn Future`: Async function that can be awaited
/// - `Output = Result<...>`: Can fail with `McpError`
/// - `Box<dyn HandlerResult>`: Type-erased result implementing `HandlerResult` trait
/// - `+ Send + 'a`: Can be sent between threads, lifetime tied to handler
pub type HandlerResponse<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn HandlerResult>, McpError>> + Send + 'a>>;

/// Result type that all local handlers must return
pub trait HandlerResult: Send + Sync {
    /// Serialize this result to a JSON value (required due to dyn compatibility)
    fn to_json(&self) -> serde_json::Value;
}

/// Trait for local handlers using function pointer approach
pub trait LocalToolFunction: Send + Sync {
    /// Handle the request and return a typed result
    fn call(&self, ctx: &HandlerContext<LocalContext>) -> HandlerResponse<'_>;
}

/// Type of handler for the tool
#[derive(Clone)]
pub enum HandlerType {
    /// BRP handler - calls a BRP method
    Brp {
        /// Source of the BRP method name
        method_source: BrpMethodSource,
    },

    /// Local handler using function pointer approach
    Local {
        /// Handler trait object
        handler: Arc<dyn LocalToolFunction>,
    },
}

impl HandlerType {
    /// Create a BRP handler with a static method name
    pub const fn brp(method: &'static str) -> Self {
        Self::Brp {
            method_source: BrpMethodSource::Static { method },
        }
    }

    /// Create a BRP handler that extracts the method name from the request
    pub const fn brp_execute() -> Self {
        Self::Brp {
            method_source: BrpMethodSource::Dynamic,
        }
    }

    /// Create the appropriate tool handler based on the handler type
    pub fn create_handler(
        &self,
        context: HandlerContext,
    ) -> Result<Box<dyn ToolHandler + Send>, McpError> {
        match self {
            Self::Local { handler } => {
                let local_context = HandlerContext::with_data(
                    context.service,
                    context.request,
                    context.context,
                    LocalContext {
                        handler: handler.clone(),
                    },
                );
                Ok(Box::new(LocalToolHandler::new(local_context)))
            }
            Self::Brp { method_source } => {
                let (method, port) = match method_source {
                    BrpMethodSource::Static { method } => {
                        let method_string = (*method).to_string();
                        let port = context.extract_port_param();
                        (method_string, port)
                    }
                    BrpMethodSource::Dynamic => {
                        let method = context.extract_method_param()?;
                        let port = context.extract_port_param();
                        (method, port)
                    }
                };
                let brp_context = HandlerContext::with_data(
                    context.service,
                    context.request,
                    context.context,
                    BrpContext { method, port },
                );
                Ok(Box::new(BrpToolHandler::new(brp_context)))
            }
        }
    }
}
