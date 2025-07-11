use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rmcp::model::{
    CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities,
};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, Peer, RoleServer, ServerHandler};

use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::tool;

/// Context passed to all local handlers containing service, request, and MCP context
pub struct HandlerContext {
    pub service: Arc<McpService>,
    pub request: CallToolRequestParam,
    pub context: RequestContext<RoleServer>,
}

impl HandlerContext {
    pub const fn new(
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Self {
        Self {
            service,
            request,
            context,
        }
    }
}

/// MCP service implementation for Bevy Remote Protocol integration.
///
/// This service provides tools for interacting with Bevy applications through BRP,
/// including entity manipulation, component management, and resource access.
#[derive(Clone)]
pub struct McpService {
    /// Project root directories configured by the MCP client.
    ///
    /// These paths are used to locate Bevy applications and projects
    /// for scanning and launching operations.
    pub roots: Arc<Mutex<Vec<PathBuf>>>,
}

impl McpService {
    pub fn new() -> Self {
        Self {
            roots: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Fetches search roots from the connected MCP client.
    ///
    /// # Errors
    /// Returns an error if the MCP client cannot be contacted or if the `list_roots` call fails.
    ///
    /// # Panics
    /// Panics if the mutex lock on roots is poisoned.
    pub async fn fetch_roots_from_client(
        &self,
        peer: Peer<RoleServer>,
    ) -> Result<(), Box<dyn Error>> {
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
        Ok(list_mcp_tools())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let handler_context = HandlerContext::new(Arc::new(self.clone()), request, context);
        find_and_call_tool(handler_context).await
    }
}

/// Fetch roots from the client and return the search paths
pub async fn fetch_roots_and_get_paths(
    handler_context: &HandlerContext,
) -> Result<Vec<PathBuf>, McpError> {
    // Fetch current roots from client
    tracing::debug!("Fetching current roots from client...");
    if let Err(e) = handler_context
        .service
        .fetch_roots_from_client(handler_context.context.peer.clone())
        .await
    {
        tracing::debug!("Failed to fetch roots: {}", e);
    }

    Ok(handler_context
        .service
        .roots
        .lock()
        .map_err(|e| {
            report_to_mcp_error(
                &error_stack::Report::new(ServiceError::MutexPoisoned("roots lock".to_string()))
                    .attach_printable(format!("Lock error: {e}")),
            )
        })?
        .clone())
}

fn list_mcp_tools() -> ListToolsResult {
    ListToolsResult {
        next_cursor: None,
        tools:       {
            let mut tools: Vec<_> = tool::get_all_tool_definitions()
                .into_iter()
                .map(tool::get_tool)
                .collect();
            tools.sort_by_key(|tool| tool.name.clone());
            tools
        },
    }
}

async fn find_and_call_tool(handler_context: HandlerContext) -> Result<CallToolResult, McpError> {
    // Check if this is one of the declaratively defined tools
    let all_tools = tool::get_all_tool_definitions();
    if let Some(def) = all_tools
        .iter()
        .find(|d| d.name == handler_context.request.name)
    {
        return tool::handle_call_tool(def, handler_context).await;
    }

    // All tools have been migrated to declarative definitions
    let tool_name = &handler_context.request.name;
    Err(report_to_mcp_error(
        &error_stack::Report::new(ServiceError::InvalidArgument(format!(
            "unknown tool: {tool_name}"
        )))
        .attach_printable("Tool not found"),
    ))
}
