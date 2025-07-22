//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;

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
    ) -> ToolHandler {
        use super::handler_context::HandlerContext;
        use crate::tool::types::ToolContext;

        // Simplified context creation - all tools use same simple context
        match &self.handler {
            HandlerFn::Local(_) => {
                // Create simple HandlerContext - all local tools use this unified context
                let ctx = HandlerContext::new(self.clone(), request, roots);
                let tool_context = ToolContext::Local(ctx);
                ToolHandler::new(self.handler.clone(), tool_context)
            }
            HandlerFn::Brp(_) => {
                // Create simple HandlerContext - BRP tools extract port/method themselves
                let ctx = HandlerContext::new(self.clone(), request, roots);
                let tool_context = ToolContext::Brp(ctx);
                ToolHandler::new(self.handler.clone(), tool_context)
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
                HandlerFn::Brp(_) => {
                    // Method is now compile-time via trait, use base title for now
                    // TODO: Consider adding method to display if needed
                    format!("{category_prefix}: {base_title}")
                }
                HandlerFn::Local(_) => {
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
