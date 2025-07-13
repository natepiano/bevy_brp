use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{HandlerContext, LocalContext};
use crate::tool::ToolHandler;

pub struct LocalToolHandler {
    context: HandlerContext<LocalContext>,
}

impl LocalToolHandler {
    pub const fn new(context: HandlerContext<LocalContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl ToolHandler for LocalToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        crate::tool::local_tool_call(&self.context).await
    }
}
