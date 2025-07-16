use std::sync::Arc;

use rmcp::Error as McpError;

use super::{HandlerContext, HasCallInfo};
use crate::response::CallInfo;
use crate::tool::{HandlerResponse, LocalToolFunction, LocalToolFunctionWithPort};

/// Enum to hold either basic handler or handler with port
#[derive(Clone)]
pub enum LocalHandler {
    Basic(Arc<dyn LocalToolFunction>),
    WithPort(Arc<dyn LocalToolFunctionWithPort>),
}

impl LocalHandler {
    // Convenience methods removed - use From trait implementations instead

    /// Dispatch method that calls the appropriate handler based on type
    pub fn call_handler<'a>(
        &'a self,
        ctx: &'a HandlerContext<LocalContext>,
    ) -> HandlerResponse<'a> {
        match self {
            Self::Basic(handler) => handler.call(ctx),
            Self::WithPort(handler) => {
                let port = ctx.port().ok_or_else(|| {
                    McpError::invalid_params("WithPort handler called without port parameter", None)
                });

                match port {
                    Ok(p) => handler.call(ctx, p),
                    Err(e) => Box::pin(async move { Err(e) }),
                }
            }
        }
    }
}

impl LocalHandler {
    /// Create a Basic handler with automatic Arc wrapping
    pub fn basic<T: LocalToolFunction + 'static>(handler: T) -> Self {
        Self::Basic(Arc::new(handler))
    }

    /// Create a `WithPort` handler with automatic Arc wrapping
    pub fn with_port<T: LocalToolFunctionWithPort + 'static>(handler: T) -> Self {
        Self::WithPort(Arc::new(handler))
    }
}

/// Data type for local handler contexts (carries the extracted handler)
#[derive(Clone)]
pub struct LocalContext {
    pub handler:     LocalHandler,
    pub(super) port: Option<u16>,
}

impl HasCallInfo for HandlerContext<LocalContext> {
    fn call_info(&self) -> CallInfo {
        self.call_info()
    }
}

// Type-specific implementations
impl HandlerContext<LocalContext> {
    pub fn call_info(&self) -> CallInfo {
        CallInfo::local(self.request.name.to_string())
    }

    pub const fn handler(&self) -> &LocalHandler {
        &self.handler_data.handler
    }

    pub const fn port(&self) -> Option<u16> {
        self.handler_data.port
    }
}
