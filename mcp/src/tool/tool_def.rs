//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolRequestParam;

use super::HandlerFn;
use super::annotations::BrpToolAnnotations;
use super::mcp_tool_schema::ParameterBuilder;
use super::types::ToolHandler;
use crate::response::ResponseSpecification;

/// Unified tool definition that can handle both BRP and Local tools
#[derive(Clone)]
pub struct ToolDef {
    /// Tool name
    pub name:            &'static str,
    /// Tool description
    pub description:     &'static str,
    /// Tool annotations
    pub annotations:     BrpToolAnnotations,
    /// Handler function with method source information
    pub handler:         HandlerFn,
    /// Function to build parameters for MCP registration
    pub parameters:      Option<fn() -> ParameterBuilder>,
    /// Response formatting specification
    pub response_format: ResponseSpecification,
}

impl ToolDef {
    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn formatter(&self) -> &ResponseSpecification {
        &self.response_format
    }

    pub fn create_handler(
        &self,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> Result<ToolHandler, McpError> {
        use super::handler_context::{HandlerContext, HasMethod, HasPort, NoMethod, NoPort};
        use crate::tool::types::ToolContext;

        // Direct context creation - pure capability-based approach
        match &self.handler {
            HandlerFn::Local(_) => {
                // Create HandlerContext<NoPort, NoMethod>
                let ctx = HandlerContext::with_data(self.clone(), request, roots, NoPort, NoMethod);
                let tool_context = ToolContext::Local(ctx);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::LocalWithPort(_) => {
                // Extract port and create HandlerContext<HasPort, NoMethod>
                let port = extract_port_directly(&request)?;
                let ctx = HandlerContext::with_data(
                    self.clone(),
                    request,
                    roots,
                    HasPort { port },
                    NoMethod,
                );
                let tool_context = ToolContext::LocalWithPort(ctx);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::Brp { method, .. } => {
                // Extract port and use static method, create HandlerContext<HasPort, HasMethod>
                let port = extract_port_directly(&request)?;
                let ctx = HandlerContext::with_data(
                    self.clone(),
                    request,
                    roots,
                    HasPort { port },
                    HasMethod {
                        method: (*method).to_string(),
                    },
                );
                let tool_context = ToolContext::Brp(ctx);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
        }
    }

    /// Convert to MCP Tool for registration
    pub fn to_tool(&self) -> rmcp::model::Tool {
        // Build parameters using the provided builder function, or create empty builder
        let builder = self
            .parameters
            .map_or_else(ParameterBuilder::new, |builder_fn| builder_fn());

        // Enhance title with category prefix and optional method name
        let enhanced_annotations = {
            let mut enhanced = self.annotations.clone();

            // Start with category prefix
            let category_prefix = enhanced.category.as_ref();
            let base_title = &enhanced.title;

            // Add method name for BRP tools
            let full_title = match &self.handler {
                HandlerFn::Brp { method, .. } => {
                    format!("{category_prefix}: {base_title} ({method})")
                }
                HandlerFn::Local(_) | HandlerFn::LocalWithPort(_) => {
                    format!("{category_prefix}: {base_title}")
                }
            };

            enhanced.title = full_title;
            enhanced
        };

        rmcp::model::Tool {
            name:         self.name.into(),
            description:  Some(self.description.into()),
            input_schema: builder.build(),
            annotations:  Some(enhanced_annotations.into()),
        }
    }
}

/// Extract port parameter directly from request arguments\
/// Used during context creation, then discarded
fn extract_port_directly(request: &CallToolRequestParam) -> Result<u16, McpError> {
    use crate::constants::{DEFAULT_BRP_PORT, PARAM_PORT, VALID_PORT_RANGE};

    let port_u64 = request
        .arguments
        .as_ref()
        .and_then(|args| args.get(PARAM_PORT))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| u64::from(DEFAULT_BRP_PORT));

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
