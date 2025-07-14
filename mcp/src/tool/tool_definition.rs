//! Common trait for tool definitions

use std::sync::Arc;

use rmcp::model::{CallToolRequestParam, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use crate::response::ResponseSpecification;
use crate::service::McpService;
use crate::tool::{LocalParameter, ToolHandler};

/// Specifies whether a tool requires a port parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortParameter {
    /// Tool requires a port parameter
    Required,
    /// Tool does not use a port parameter
    NotUsed,
}

/// Common interface for parameter definitions
pub trait ParameterDefinition {
    /// Get the parameter name as string
    fn name(&self) -> &str;

    /// Check if the parameter is required
    fn required(&self) -> bool;

    /// Get the parameter description
    fn description(&self) -> &'static str;

    /// Get the parameter type (we need to import `ParamType`)
    fn param_type(&self) -> &crate::tool::ParamType;
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

    /// Convert to MCP Tool for registration
    fn to_tool(&self) -> Tool {
        use crate::support::schema;
        use crate::tool::ParamType;

        let mut builder = schema::SchemaBuilder::new();

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

        // Add port parameter if needed
        if self.port_parameter() == PortParameter::Required {
            let port_param = LocalParameter::port();
            builder = builder.add_number_property(
                port_param.name(),
                port_param.description(),
                port_param.required(),
            );
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
