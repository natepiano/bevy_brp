//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;
use std::sync::Arc;

use rmcp::ErrorData;
use rmcp::model::CallToolRequestParam;
use rmcp::model::CallToolResult;
use schemars::generate::SchemaSettings;

use super::HandlerContext;
use super::annotations::Annotation;
use super::json_response::ToolCallJsonResponse;
use super::parameters::ParameterBuilder;
use super::tool_name::ToolName;
use super::types::ErasedToolFn;

/// Unified tool definition that can handle both BRP and Local tools
#[derive(Clone)]
pub struct ToolDef {
    /// Tool name and description
    pub tool_name:   ToolName,
    /// Tool annotations
    pub annotations: Annotation,
    /// Handler function
    pub handler:     Arc<dyn ErasedToolFn>,
    /// Function to build parameters for MCP registration
    pub parameters:  Option<fn() -> ParameterBuilder>,
}

impl ToolDef {
    pub fn name(&self) -> &'static str {
        self.tool_name.into()
    }

    pub async fn call_tool(
        &self,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> std::result::Result<CallToolResult, ErrorData> {
        // Create HandlerContext - all tools use the same context
        let ctx = HandlerContext::new(self.clone(), request, roots);

        // Tools now always return CallToolResult - errors are already formatted as responses
        Ok(self.handler.call_erased(ctx).await)
    }

    /// Generate unified output schema from the actual `ToolCallJsonResponse` struct
    fn generate_output_schema() -> Arc<rmcp::model::JsonObject> {
        let mut settings = SchemaSettings::default();
        settings.inline_subschemas = true;
        let generator = settings.into_generator();
        let schema = generator.into_root_schema_for::<ToolCallJsonResponse>();

        let Ok(schema_value) = serde_json::to_value(schema) else {
            // Fallback to empty schema if serialization fails
            return Arc::new(rmcp::model::JsonObject::new());
        };

        let schema_object = schema_value
            .as_object()
            .map_or_else(rmcp::model::JsonObject::new, Clone::clone);

        Arc::new(schema_object)
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
            name:          <&'static str>::from(self.tool_name).into(),
            title:         Some(self.tool_name.short_title()),
            description:   Some(self.tool_name.description().into()),
            input_schema:  builder.build(),
            output_schema: Some(Self::generate_output_schema()),
            annotations:   Some(enhanced_annotations.into()),
            icons:         None,
            meta:          None,
        }
    }
}
