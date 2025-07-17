use std::path::PathBuf;

use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;

use crate::constants::{DEFAULT_BRP_PORT, VALID_PORT_RANGE};
use crate::service::brp_context::BrpContext;
use crate::service::handler_context::HandlerContext;
use crate::service::local_context::LocalContext;
use crate::tool::ToolDef;

/// The base context type that is the only one that can be constructed directly.
/// This is the entry point for all handler context creation and is the only
/// context that has extraction methods.
pub struct BaseContext;

impl HandlerContext<BaseContext> {
    /// This is the only way to create a `HandlerContext`
    pub const fn new(
        tool_def: ToolDef,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            tool_def,
            request,
            roots,
            handler_data: BaseContext,
        }
    }

    /// Extract the port parameter from the request arguments.
    /// Only available on `BaseContext`.
    pub fn extract_port(&self) -> Result<u16, McpError> {
        use crate::constants::PARAM_PORT;

        let port_u64 = self.extract_optional_number(PARAM_PORT, u64::from(DEFAULT_BRP_PORT));

        let port = u16::try_from(port_u64).map_err(|_| {
            McpError::invalid_params("Invalid port parameter: value too large for u16", None)
        })?;

        // Validate port range (1024-65535 for non-privileged ports)
        if !VALID_PORT_RANGE.contains(&port) {
            return Err(McpError::invalid_params(
                format!(
                    "Invalid port {port}: must be in range {}-{}",
                    VALID_PORT_RANGE.start(),
                    VALID_PORT_RANGE.end()
                ),
                None,
            ));
        }

        Ok(port)
    }

    /// Extract the method parameter from the request arguments.
    /// Only available on `BaseContext`.
    pub fn extract_method_param(&self) -> Result<String, McpError> {
        self.extract_required_string("method", "method name")
            .map(|s| (*s).to_string())
    }

    /// Transition to `LocalContext` with the specified port.
    pub fn into_local(self, port: Option<u16>) -> HandlerContext<LocalContext> {
        HandlerContext::with_data(
            self.tool_def,
            self.request,
            self.roots,
            LocalContext { port },
        )
    }

    /// Transition to `BrpContext` with the specified method and port.
    pub fn into_brp(self, method: String, port: u16) -> HandlerContext<BrpContext> {
        HandlerContext::with_data(
            self.tool_def,
            self.request,
            self.roots,
            BrpContext { method, port },
        )
    }
}
