use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rmcp::model::{
    CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities,
};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, Peer, RoleServer, ServerHandler};

use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::registry;

/// MCP service implementation for Bevy Remote Protocol integration.
///
/// This service provides tools for interacting with Bevy applications through BRP,
/// including entity manipulation, component management, and resource access.
#[derive(Clone)]
pub struct BrpMcpService {
    /// Project root directories configured by the MCP client.
    ///
    /// These paths are used to locate Bevy applications and projects
    /// for scanning and launching operations.
    pub roots: Arc<Mutex<Vec<PathBuf>>>,
}

impl BrpMcpService {
    pub fn new() -> Self {
        Self {
            roots: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Fetches search roots from the connected MCP client.
    ///
    /// # Errors
    ///
    /// Returns an error if the MCP client cannot be contacted or if the `list_roots` call fails.
    ///
    /// # Panics
    ///
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

impl ServerHandler for BrpMcpService {
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
        Ok(registry::register_tools())
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        registry::handle_tool_call(self, request, context).await
    }
}

/// Fetch roots from the client and return the search paths
async fn fetch_roots_and_get_paths(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
) -> Result<Vec<PathBuf>, McpError> {
    // Fetch current roots from client
    tracing::debug!("Fetching current roots from client...");
    if let Err(e) = service.fetch_roots_from_client(context.peer.clone()).await {
        tracing::debug!("Failed to fetch roots: {}", e);
    }

    Ok(service
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

/// Handler wrapper for binary listing operations that fetches search paths
/// This eliminates the repetitive pattern of fetching roots in `list_bevy_apps` and
/// `list_bevy_examples`
pub async fn handle_list_binaries<F, Fut>(
    service: &BrpMcpService,
    context: RequestContext<RoleServer>,
    handler: F,
) -> Result<CallToolResult, McpError>
where
    F: FnOnce(Vec<PathBuf>) -> Fut,
    Fut: Future<Output = Result<CallToolResult, McpError>>,
{
    let search_paths = fetch_roots_and_get_paths(service, context).await?;
    handler(search_paths).await
}

/// Handler wrapper for binary launch operations that fetches search paths and extracts request data
/// This eliminates boilerplate for handlers that launch binaries with parameters used by
/// `launch_bevy_app` and `launch_bevy_example`
pub async fn handle_launch_binary<F, Fut>(
    service: &BrpMcpService,
    request: CallToolRequestParam,
    context: RequestContext<RoleServer>,
    handler: F,
) -> Result<CallToolResult, McpError>
where
    F: FnOnce(CallToolRequestParam, Vec<PathBuf>) -> Fut,
    Fut: Future<Output = Result<CallToolResult, McpError>>,
{
    let search_paths = fetch_roots_and_get_paths(service, context).await?;
    handler(request, search_paths).await
}
