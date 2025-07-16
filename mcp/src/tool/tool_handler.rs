use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use super::handler_fn::HandlerFn;
use super::types::ToolContext;

/// Unified tool handler that works with any `HandlerFn` variant
pub struct ToolHandler {
    handler: HandlerFn,
    context: ToolContext,
}

impl ToolHandler {
    pub const fn new(handler: HandlerFn, context: ToolContext) -> Self {
        Self { handler, context }
    }
}

impl ToolHandler {
    pub async fn call_tool(self) -> Result<CallToolResult, McpError> {
        self.handler.call_handler(&self.context).await
    }
}
