//! # Bevy BRP MCP Server
//!
//! A Model Context Protocol server that provides tools for interacting with
//! Bevy applications through the Bevy Remote Protocol (BRP).
//!
//! This server enables remote debugging, inspection, and manipulation of
//! Bevy applications at runtime through a standardized MCP interface.

use std::error::Error;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rmcp::model::{
    CallToolRequestParam, CallToolResult, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities,
};
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{Error as McpError, RoleServer, ServerHandler, ServiceExt};

mod app_tools;
mod brp_tools;
mod constants;
mod error;
mod log_tools;
mod registry;
mod support;
mod tool_definitions;
mod tool_generator;
mod tools;

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
    fn new() -> Self {
        Self {
            roots: Arc::new(Mutex::new(Vec::new())),
        }
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

impl BrpMcpService {
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
        peer: rmcp::service::Peer<RoleServer>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize file-based tracing with dynamic level management
    let _tracing_guard = support::tracing::init_file_tracing();

    // Don't log anything here - it would create the file and violate "do no harm"
    // The file should only be created when the user explicitly sets a tracing level

    // Initialize the watch manager
    brp_tools::watch::support::manager::initialize_watch_manager().await;

    let service = BrpMcpService::new();

    let server = service.serve(stdio()).await?;
    server.waiting().await?;

    Ok(())
}
