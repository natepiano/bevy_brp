use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rmcp::model::{
    CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, Peer, RoleServer, ServerHandler};

use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::tool::{self, ToolDefinition};

/// MCP service implementation for Bevy Remote Protocol integration.
///
/// This service provides tools for interacting with Bevy applications through BRP,
/// including entity manipulation, component management, and resource access.
pub struct McpService {
    /// Project root directories configured by the MCP client.
    ///
    /// These paths are used to locate Bevy applications and projects
    /// for scanning and launching operations.
    roots:     Arc<Mutex<Vec<PathBuf>>>,
    /// Tool definitions `HashMap` for O(1) lookup by name
    tool_defs: HashMap<String, Box<dyn ToolDefinition>>,
    /// Pre-converted MCP tools for list operations
    tools:     Vec<Tool>,
}

impl McpService {
    pub fn new() -> Self {
        let all_defs = tool::get_all_tool_definitions();
        let tool_defs = all_defs
            .iter()
            .map(|def| (def.name().to_string(), def.clone_box()))
            .collect();
        let mut tools: Vec<_> = all_defs.iter().map(|def| def.to_tool()).collect();
        tools.sort_by_key(|tool| tool.name.clone());

        Self {
            roots: Arc::new(Mutex::new(Vec::new())),
            tool_defs,
            tools,
        }
    }

    /// Get tool definition by name with O(1) lookup
    pub fn get_tool_def(&self, name: &str) -> Option<&dyn ToolDefinition> {
        self.tool_defs.get(name).map(std::convert::AsRef::as_ref)
    }

    /// List all MCP tools using pre-converted and sorted tools
    pub fn list_mcp_tools(&self) -> ListToolsResult {
        ListToolsResult {
            next_cursor: None,
            tools:       self.tools.clone(),
        }
    }

    /// Fetch roots from the client and return the search paths
    ///
    /// # Errors
    /// Returns an error if the MCP client cannot be contacted, if the `list_roots` call fails,
    /// or if the mutex lock on roots is poisoned.
    pub async fn fetch_roots_and_get_paths(
        &self,
        peer: Peer<RoleServer>,
    ) -> Result<Vec<PathBuf>, McpError> {
        // Fetch current roots from client
        tracing::debug!("Fetching current roots from client...");
        if let Err(e) = self.fetch_roots_from_client(peer.clone()).await {
            tracing::debug!("Failed to fetch roots: {}", e);
        }

        Ok(self
            .roots
            .lock()
            .map_err(|e| {
                report_to_mcp_error(
                    &error_stack::Report::new(ServiceError::MutexPoisoned(
                        "roots lock".to_string(),
                    ))
                    .attach_printable(format!("Lock error: {e}")),
                )
            })?
            .clone())
    }

    /// Fetches search roots from the connected MCP client.
    ///
    /// # Errors
    /// Returns an error if the MCP client cannot be contacted or if the `list_roots` call fails.
    ///
    /// # Panics
    /// Panics if the mutex lock on roots is poisoned.
    async fn fetch_roots_from_client(&self, peer: Peer<RoleServer>) -> Result<(), Box<dyn Error>> {
        // Use the peer extension method to list roots
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

                // Update our roots
                let mut roots = self
                    .roots
                    .lock()
                    .map_err(|e| format!("Failed to acquire roots lock: {e}"))?;
                *roots = paths;
                tracing::debug!("Processed roots: {:?}", *roots);
            }
            Err(e) => {
                tracing::error!("Failed to send roots/list request: {}", e);
            }
        }

        Ok(())
    }
}

impl Clone for McpService {
    fn clone(&self) -> Self {
        Self {
            roots:     Arc::clone(&self.roots),
            tool_defs: self
                .tool_defs
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_box()))
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
        let service_arc = Arc::new(self.clone());
        let tool_def = service_arc.get_tool_def(&request.name).ok_or_else(|| {
            crate::error::report_to_mcp_error(
                &error_stack::Report::new(crate::error::Error::InvalidArgument(format!(
                    "unknown tool: {}",
                    request.name
                )))
                .attach_printable("Tool not found"),
            )
        })?;

        tool_def
            .create_handler(Arc::clone(&service_arc), request, context)?
            .call_tool()
            .await
    }
}
