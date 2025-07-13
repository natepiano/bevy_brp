use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{HandlerContext, LocalContext};
use crate::tool::ToolHandlerImpl;

pub struct LocalToolHandler {
    context: HandlerContext<LocalContext>,
}

impl LocalToolHandler {
    pub const fn new(context: HandlerContext<LocalContext>) -> Self {
        Self { context }
    }
}

impl ToolHandlerImpl for LocalToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        crate::tool::local_tool_call(&self.context).await
    }
}
