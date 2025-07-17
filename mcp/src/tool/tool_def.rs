//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;

use rmcp::Error as McpError;
use rmcp::model::CallToolRequestParam;

use super::HandlerFn;
use super::mcp_tool_schema::McpToolSchemaBuilder;
use super::parameters::{ParamType, Parameter, ParameterDefinition, PortParameter};
use super::types::{BrpMethodSource, ToolHandler};
use crate::constants::{PARAM_METHOD, PARAM_PORT};
use crate::response::ResponseSpecification;

/// Unified tool definition that can handle both BRP and Local tools
#[derive(Clone)]
pub struct ToolDef {
    /// Tool name
    pub name:            &'static str,
    /// Tool description
    pub description:     &'static str,
    /// Handler function with method source information
    pub handler:         HandlerFn,
    /// Type-safe parameters
    pub parameters:      Vec<Parameter>,
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

    pub fn parameters(&self) -> Vec<&dyn ParameterDefinition> {
        self.parameters
            .iter()
            .map(|p| p as &dyn ParameterDefinition)
            .collect()
    }

    pub const fn port_parameter(&self) -> PortParameter {
        match &self.handler {
            HandlerFn::Local(_) => PortParameter::NotUsed,
            HandlerFn::LocalWithPort(_) | HandlerFn::Brp { .. } => PortParameter::Required,
        }
    }

    pub const fn needs_method_parameter(&self) -> bool {
        match &self.handler {
            HandlerFn::Brp { method_source, .. } => {
                matches!(method_source, BrpMethodSource::Dynamic)
            }
            _ => false, // Local tools never need method parameters
        }
    }

    pub fn create_handler(
        &self,
        request: CallToolRequestParam,
        roots: Vec<PathBuf>,
    ) -> Result<ToolHandler, McpError> {
        use crate::service::{BaseContext, HandlerContext};
        use crate::tool::types::ToolContext;

        // Create base context - the ONLY way to start
        let base_ctx = HandlerContext::<BaseContext>::new(self.clone(), request, roots);

        match &self.handler {
            HandlerFn::Local(_) => {
                // Local tool - no port needed
                let local_handler_context = base_ctx.into_local(None);
                let tool_context = ToolContext::Local(local_handler_context);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::LocalWithPort(_) => {
                // Local tool with port
                let port = base_ctx.extract_port()?;
                let local_handler_context = base_ctx.into_local(Some(port));
                let tool_context = ToolContext::Local(local_handler_context);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::Brp { method_source, .. } => {
                // BRP tool
                let port = base_ctx.extract_port()?;
                let method = match method_source {
                    BrpMethodSource::Static(method_name) => (*method_name).to_string(),
                    BrpMethodSource::Dynamic => base_ctx.extract_method_param()?,
                };

                // Transition to BrpContext
                let brp_handler_context = base_ctx.into_brp(method, port);
                let tool_context = ToolContext::Brp(brp_handler_context);

                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
        }
    }

    /// Convert to MCP Tool for registration
    pub fn to_tool(&self) -> rmcp::model::Tool {
        let mut builder = McpToolSchemaBuilder::new();

        // Add tool-specific parameters
        for param in self.parameters() {
            builder = match param.param_type() {
                ParamType::String => {
                    builder.add_string_property(param.name(), param.description(), param.required())
                }
                ParamType::Number => {
                    builder.add_number_property(param.name(), param.description(), param.required())
                }
                ParamType::Boolean => builder.add_boolean_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParamType::StringArray => builder.add_string_array_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParamType::NumberArray => builder.add_number_array_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParamType::Any | ParamType::DynamicParams => {
                    builder.add_any_property(param.name(), param.description(), param.required())
                }
            };
        }

        // Add method parameter if needed (for dynamic BRP tools)
        if self.needs_method_parameter() {
            builder = builder.add_string_property(
                PARAM_METHOD,
                "The BRP method to execute (e.g., 'rpc.discover', 'bevy/get', 'bevy/query')",
                true,
            );
        }

        // Add port parameter if needed
        if self.port_parameter() == PortParameter::Required {
            builder =
                builder.add_number_property(PARAM_PORT, "The BRP port (default: 15702)", false);
        }

        rmcp::model::Tool {
            name:         self.name.into(),
            description:  self.description.into(),
            input_schema: builder.build(),
        }
    }
}
