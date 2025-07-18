use std::collections::HashMap;
use std::path::PathBuf;

use rmcp::model::{
    CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, Peer, RoleServer, ServerHandler};

use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::tool::{self, ToolDef};

/// MCP service implementation for Bevy Remote Protocol integration.
///
/// This service provides tools for interacting with Bevy applications through BRP,
/// including entity manipulation, component management, and resource access.
pub struct McpService {
    /// Tool definitions `HashMap` for O(1) lookup by name
    tool_defs: HashMap<String, ToolDef>,
    /// Pre-converted MCP tools for list operations
    tools:     Vec<Tool>,
}

impl McpService {
    pub fn new() -> Self {
        let all_defs = tool::get_all_tool_definitions();
        let tool_defs = all_defs
            .iter()
            .map(|def| (def.name().to_string(), def.clone()))
            .collect();
        let mut tools: Vec<_> = all_defs.iter().map(ToolDef::to_tool).collect();
        tools.sort_by_key(|tool| tool.name.clone());

        Self { tool_defs, tools }
    }

    /// Get tool definition by name with O(1) lookup
    pub fn get_tool_def(&self, name: &str) -> Option<&ToolDef> {
        self.tool_defs.get(name)
    }

    /// List all MCP tools using pre-converted and sorted tools
    fn list_mcp_tools(&self) -> ListToolsResult {
        ListToolsResult {
            next_cursor: None,
            tools:       self.tools.clone(),
        }
    }

    /// Fetch roots from the client and return the search paths
    ///
    /// # Errors
    /// Returns an error if the MCP client cannot be contacted or if the `list_roots` call fails.
    pub async fn fetch_roots_and_get_paths(
        &self,
        peer: Peer<RoleServer>,
    ) -> Result<Vec<PathBuf>, McpError> {
        // Fetch current roots from client
        tracing::debug!("Fetching current roots from client...");

        match peer.list_roots().await {
            Ok(result) => {
                tracing::debug!("Received {} roots from client", result.roots.len());
                for (i, root) in result.roots.iter().enumerate() {
                    tracing::debug!(
                        "  Root {}: {} ({})",
                        i + 1,
                        root.uri,
                        root.name.as_deref().unwrap_or("unnamed")
                    );
                }

                let paths: Vec<PathBuf> = result
                    .roots
                    .iter()
                    .filter_map(|root| {
                        // Parse the file:// URI
                        root.uri.strip_prefix("file://").map_or_else(
                            || {
                                tracing::warn!("Ignoring non-file URI: {}", root.uri);
                                None
                            },
                            |path| Some(PathBuf::from(path)),
                        )
                    })
                    .collect();

                tracing::debug!("Processed roots: {:?}", paths);
                Ok(paths)
            }
            Err(e) => {
                tracing::error!("Failed to send roots/list request: {}", e);
                Err(report_to_mcp_error(&error_stack::Report::new(
                    ServiceError::McpClientCommunication(format!("Failed to list roots: {e}")),
                )))
            }
        }
    }
}

impl Clone for McpService {
    fn clone(&self) -> Self {
        Self {
            tool_defs: self
                .tool_defs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            tools:     self.tools.clone(),
        }
    }
}

impl ServerHandler for McpService {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        rmcp::model::ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(self.list_mcp_tools())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Fetch roots and get paths
        let roots = self.fetch_roots_and_get_paths(context.peer.clone()).await?;

        let tool_def = self.get_tool_def(&request.name).ok_or_else(|| {
            crate::error::report_to_mcp_error(
                &error_stack::Report::new(crate::error::Error::InvalidArgument(format!(
                    "unknown tool: {}",
                    request.name
                )))
                .attach_printable("Tool not found"),
            )
        })?;

        tool_def.create_handler(request, roots)?.call_tool().await
    }
}
