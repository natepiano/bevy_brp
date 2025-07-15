//! Common trait for tool definitions

use std::sync::Arc;

use rmcp::model::{CallToolRequestParam, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::parameters::{ParamType, ParameterDefinition};
use crate::constants::{PARAM_METHOD, PARAM_PORT};
use crate::response::ResponseSpecification;
use crate::service::McpService;
use super::mcp_tool_schema::McpToolSchemaBuilder;
use crate::tool::ToolHandler;

/// Specifies whether a tool requires a port parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortParameter {
    /// Tool requires a port parameter
    Required,
    /// Tool does not use a port parameter
    NotUsed,
}

/// Common interface for all tool definitions
pub trait ToolDefinition: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &'static str;

    /// Get the tool description
    fn description(&self) -> &'static str;

    /// Get the response formatter specification
    fn formatter(&self) -> &ResponseSpecification;

    /// Get the parameters for this tool
    fn parameters(&self) -> Vec<&dyn ParameterDefinition>;

    /// Get the port parameter requirement for this tool
    fn port_parameter(&self) -> PortParameter;

    /// Check if this tool needs a method parameter (for dynamic BRP tools)
    fn needs_method_parameter(&self) -> bool;

    /// Convert to MCP Tool for registration
    fn to_tool(&self) -> Tool {
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
                ParamType::Any => {
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
        // can't be created any other way as neither LocalParameterName or BrpParameterName
        // has a Port variant
        if self.port_parameter() == PortParameter::Required {
            builder =
                builder.add_number_property(PARAM_PORT, "The BRP port (default: 15702)", false);
        }

        Tool {
            name:         self.name().into(),
            description:  self.description().into(),
            input_schema: builder.build(),
        }
    }

    /// Create a handler instance for this tool
    fn create_handler(
        &self,
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<Box<dyn ToolHandler + Send>, McpError>;

    /// Clone this tool definition (for compatibility)
    fn clone_box(&self) -> Box<dyn ToolDefinition>;
}
