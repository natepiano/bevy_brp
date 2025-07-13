use std::sync::Arc;

use super::{HandlerContext, HasCallInfo};
use crate::response::CallInfo;
use crate::tool::LocalToolFunction;

/// Data type for local handler contexts (carries the extracted handler)
#[derive(Clone)]
pub struct LocalContext {
    pub handler: Arc<dyn LocalToolFunction>,
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

    pub fn handler(&self) -> &Arc<dyn LocalToolFunction> {
        &self.handler_data.handler
    }
}
