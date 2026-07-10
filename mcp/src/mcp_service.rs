use std::collections::HashMap;

use itertools::Itertools;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::ServerHandler;
use rmcp::model::CallToolRequestParams;
use rmcp::model::CallToolResult;
use rmcp::model::ListToolsResult;
use rmcp::model::PaginatedRequestParams;
use rmcp::model::ServerCapabilities;
use rmcp::model::ServerInfo;
use rmcp::model::Tool;
use rmcp::service::RequestContext;

use super::tool;
use super::tool::ToolDef;

/// MCP service implementation for Bevy Remote Protocol integration.
///
/// This service provides tools for interacting with Bevy applications through BRP,
/// including entity manipulation, component management, and resource access.
pub(crate) struct McpService {
    /// Tool definitions `HashMap` for O(1) lookup by name
    tool_defs: HashMap<String, ToolDef>,
    /// Pre-converted MCP tools for list operations
    tools:     Vec<Tool>,
}

impl McpService {
    pub(crate) fn new() -> Self {
        let all_defs = tool::get_all_tool_definitions();

        // Build the `ToolDef` lookup table.
        let tool_defs = all_defs
            .iter()
            .map(|tool_def| (tool_def.name().to_string(), tool_def.clone()))
            .collect();

        // Store a sorted `Vec<Tool>` for `McpService::list_mcp_tools`.
        let tools: Vec<_> = all_defs
            .iter()
            .map(ToolDef::to_tool)
            .sorted_by_key(|tool| {
                tool.annotations
                    .as_ref()
                    .and_then(|ann| ann.title.as_ref())
                    .map_or_else(|| tool.name.as_ref(), String::as_str)
                    .to_string()
            })
            .collect();

        Self { tool_defs, tools }
    }

    /// Get tool definition by name with O(1) lookup
    fn get_tool_def(&self, name: &str) -> Option<&ToolDef> { self.tool_defs.get(name) }

    /// List all MCP tools using pre-converted and sorted tools
    fn list_mcp_tools(&self) -> ListToolsResult {
        ListToolsResult {
            meta:        None,
            next_cursor: None,
            tools:       self.tools.clone(),
        }
    }
}

impl ServerHandler for McpService {
    fn get_info(&self) -> ServerInfo {
        let mut info = rmcp::model::ServerInfo::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(self.list_mcp_tools())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_def = self.get_tool_def(&request.name).ok_or_else(|| {
            McpError::invalid_params(format!("unknown tool: {}", request.name), None)
        })?;

        tool_def.call_tool(request).await
    }
}
