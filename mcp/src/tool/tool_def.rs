//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::model::CallToolRequestParam;

use super::annotations::BrpToolAnnotations;
use super::parameters::ParameterBuilder;
use super::tool_name::ToolName;
use super::types::{ErasedUnifiedToolFn, ToolHandler};
use crate::response::ResponseDef;

/// Unified tool definition that can handle both BRP and Local tools
#[derive(Clone)]
pub struct ToolDef {
    /// Tool name and description
    pub tool_name:   ToolName,
    /// Tool annotations
    pub annotations: BrpToolAnnotations,
    /// Handler function
    pub handler:     Arc<dyn ErasedUnifiedToolFn>,
    /// Function to build parameters for MCP registration
    pub parameters:  Option<fn() -> ParameterBuilder>,
    /// Response formatting specification
    pub response:    ResponseDef,
}

impl ToolDef {
    pub fn name(&self) -> &'static str {
        self.tool_name.into()
    }

    pub const fn response_def(&self) -> &ResponseDef {
        &self.response
    }

    pub fn create_handler(
        &self,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> ToolHandler {
        use super::handler_context::HandlerContext;

        // Create simple HandlerContext - all tools use the same context
        let ctx = HandlerContext::new(self.clone(), request, roots);
        ToolHandler::new(self.handler.clone(), ctx)
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

            // All tools use the same title format now
            let full_title = format!("{category_prefix}: {base_title}");

            enhanced.title = full_title;
            enhanced
        };

        rmcp::model::Tool {
            name:         <&'static str>::from(self.tool_name).into(),
            description:  Some(self.tool_name.description().into()),
            input_schema: builder.build(),
            annotations:  Some(enhanced_annotations.into()),
        }
    }
}
