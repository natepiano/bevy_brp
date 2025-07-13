use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{BrpContext, HandlerContext};
use crate::tool::ToolHandler;

pub struct BrpToolHandler {
    context: HandlerContext<BrpContext>,
}

impl BrpToolHandler {
    pub const fn new(context: HandlerContext<BrpContext>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl ToolHandler for BrpToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        crate::tool::brp_method_tool_call(&self.context).await
    }
}
