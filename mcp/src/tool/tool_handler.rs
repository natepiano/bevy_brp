use async_trait::async_trait;
use rmcp::Error as McpError;
use rmcp::model::CallToolResult;

use super::ToolHandlerTrait;
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

#[async_trait]
impl ToolHandlerTrait for ToolHandler {
    async fn call_tool(self: Box<Self>) -> Result<CallToolResult, McpError> {
        self.handler.call_handler(&self.context).await
    }
}
