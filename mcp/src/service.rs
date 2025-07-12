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

use crate::constants::{DEFAULT_BRP_PORT, PARAM_METHOD, PARAM_PORT};
use crate::error::{Error as ServiceError, report_to_mcp_error};
use crate::response::CallInfo;
use crate::tool;
use crate::tool::{HandlerType, McpToolDef};

/// Context passed to all local handlers containing service, request, and MCP context
#[derive(Clone)]
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

    /// Get tool definition by looking up the request name in the service's tool registry
    ///
    /// # Errors
    ///
    /// Returns an error if the tool definition is not found.
    pub fn tool_def(&self) -> Result<&McpToolDef, McpError> {
        self.service
            .get_tool_def(&self.request.name)
            .ok_or_else(|| {
                crate::error::report_to_mcp_error(
                    &error_stack::Report::new(crate::error::Error::InvalidArgument(format!(
                        "unknown tool: {}",
                        self.request.name
                    )))
                    .attach_printable("Tool not found"),
                )
            })
    }

    /// Extract port number from request arguments
    fn extract_port(&self) -> Option<u16> {
        self.request
            .arguments
            .as_ref()
            .and_then(|args| args.get(PARAM_PORT))
            .and_then(serde_json::Value::as_u64)
            .and_then(|p| u16::try_from(p).ok())
    }

    /// Extract method parameter from request arguments
    /// Only used with `brp_execute` which allows you to execute
    /// arbitrary brp methods
    fn extract_method_param(&self) -> Option<String> {
        self.request
            .arguments
            .as_ref()
            .and_then(|args| args.get(PARAM_METHOD))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
    }

    /// Get the BRP method name from request arguments (e.g., "bevy/spawn")
    /// Returns None for local tools that don't use BRP
    pub fn brp_method(&self) -> Option<String> {
        let tool_def = self.tool_def().ok()?;
        match &tool_def.handler {
            HandlerType::Local { .. } => None,
            HandlerType::Brp { method } => Some((*method).to_string()),
            HandlerType::BrpExecute => self.extract_method_param(),
        }
    }

    /// Get the BRP port number from request arguments
    /// Returns None for local tools that don't use BRP
    /// For BRP tools, returns provided port or DEFAULT_BRP_PORT if none provided
    pub fn port(&self) -> Option<u16> {
        let tool_def = self.tool_def().ok()?;
        match &tool_def.handler {
            HandlerType::Local { .. } => None,
            HandlerType::Brp { .. } | HandlerType::BrpExecute => {
                Some(self.extract_port().unwrap_or(DEFAULT_BRP_PORT))
            }
        }
    }

    /// Build `CallInfo` structure from the current request context
    pub fn call_info(&self) -> CallInfo {
        let mcp_tool = self.request.name.to_string();
        let tool_def = self.tool_def().expect("Tool def must exist by this point");

        match &tool_def.handler {
            HandlerType::Local { .. } => CallInfo::local(mcp_tool),
            HandlerType::Brp { .. } | HandlerType::BrpExecute => {
                let brp_method = self.brp_method().expect("BRP tools must have brp_method");
                let port = self.port().expect("BRP tools must have port");
                CallInfo::brp(mcp_tool, brp_method, port)
            }
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
    roots:     Arc<Mutex<Vec<PathBuf>>>,
    /// Tool definitions `HashMap` for O(1) lookup by name
    tool_defs: HashMap<String, McpToolDef>,
    /// Pre-converted MCP tools for list operations
    tools:     Vec<Tool>,
}

impl McpService {
    pub fn new() -> Self {
        let all_defs = tool::get_all_tool_definitions();
        let tool_defs = all_defs
            .iter()
            .map(|def| (def.name.to_string(), def.clone()))
            .collect();
        let mut tools: Vec<_> = all_defs.into_iter().map(tool::get_tool).collect();
        tools.sort_by_key(|tool| tool.name.clone());

        Self {
            roots: Arc::new(Mutex::new(Vec::new())),
            tool_defs,
            tools,
        }
    }

    /// Get tool definition by name with O(1) lookup
    pub fn get_tool_def(&self, name: &str) -> Option<&McpToolDef> {
        self.tool_defs.get(name)
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
        let handler_context = HandlerContext::new(Arc::new(self.clone()), request, context);
        tool::handle_call_tool(handler_context).await
    }
}
