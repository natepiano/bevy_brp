use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use crate::brp_tools::request_handler::handle_brp_method_tool_call;
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
        brp_method_tool_call(self.context).await
    }
}

/// Generate a BRP handler
async fn brp_method_tool_call(
    handler_context: HandlerContext<BrpContext>,
) -> Result<CallToolResult, McpError> {
    let tool_def = handler_context.tool_def()?;

    // Build the formatter config from the response specification
    let formatter_config = tool_def.formatter().build_formatter_config();

    handle_brp_method_tool_call(handler_context.clone(), formatter_config).await
}
