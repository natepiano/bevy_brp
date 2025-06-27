use std::future::Future;
use std::path::PathBuf;

use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use crate::BrpMcpService;
use crate::error::{Error, report_to_mcp_error};

/// Fetch roots from the client and return the search paths
pub async fn fetch_roots_and_get_paths(
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
                &error_stack::Report::new(Error::MutexPoisoned("roots lock".to_string()))
                    .attach_printable(format!("Lock error: {e}")),
            )
        })?
        .clone())
}

/// Generic handler wrapper that fetches search paths and calls the provided handler
/// This eliminates the repetitive pattern of fetching roots in every tool handler
pub async fn handle_with_paths<F, Fut>(
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

/// Generic handler wrapper that fetches search paths and extracts request data
/// This eliminates even more boilerplate for handlers that need request data
pub async fn handle_with_request_and_paths<F, Fut>(
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
