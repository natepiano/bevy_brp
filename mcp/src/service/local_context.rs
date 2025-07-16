use super::{HandlerContext, HasCallInfo};
use crate::response::CallInfo;

/// Data type for local handler contexts
#[derive(Clone)]
pub struct LocalContext {
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

    pub const fn port(&self) -> Option<u16> {
        self.handler_data.port
    }
}
