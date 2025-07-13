use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::service::{BrpContext, HandlerContext};
use crate::tool::ToolHandlerImpl;

pub struct BrpToolHandler {
    context: HandlerContext<BrpContext>,
}

impl BrpToolHandler {
    pub const fn new(context: HandlerContext<BrpContext>) -> Self {
        Self { context }
    }
}

impl ToolHandlerImpl for BrpToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        crate::tool::brp_method_tool_call(&self.context).await
    }
}
