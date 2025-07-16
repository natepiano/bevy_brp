use std::sync::Arc;

use super::{HandlerContext, HasCallInfo};
use crate::response::CallInfo;
use crate::tool::{LocalToolFunction, LocalToolFunctionWithPort};

/// Enum to hold either basic handler or handler with port
#[derive(Clone)]
pub enum LocalHandler {
    Basic(Arc<dyn LocalToolFunction>),
    WithPort(Arc<dyn LocalToolFunctionWithPort>),
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
    pub(super) port: u16,
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

    pub const fn port(&self) -> u16 {
        self.handler_data.port
    }
}
