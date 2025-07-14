//! Local tool definition type

use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use crate::response::ResponseSpecification;
use crate::service::{HandlerContext, LocalContext, McpService};
use crate::tool::tool_definition::{ParameterDefinition, PortParameter, ToolDefinition};
use crate::tool::{LocalParameter, LocalToolFunction, LocalToolHandler, ToolHandler};

/// Definition for local tools that execute within the MCP server
pub struct LocalToolDef {
    /// Tool name
    pub name:           &'static str,
    /// Tool description
    pub description:    &'static str,
    /// Handler function
    pub handler:        Arc<dyn LocalToolFunction>,
    /// Type-safe local parameters
    pub parameters:     Vec<LocalParameter>,
    /// Response formatting specification
    pub formatter:      ResponseSpecification,
    /// Port parameter requirement
    pub port_parameter: PortParameter,
}

impl ToolDefinition for LocalToolDef {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn formatter(&self) -> &ResponseSpecification {
        &self.formatter
    }

    fn parameters(&self) -> Vec<&dyn ParameterDefinition> {
        self.parameters
            .iter()
            .map(|p| p as &dyn ParameterDefinition)
            .collect()
    }

    fn port_parameter(&self) -> PortParameter {
        self.port_parameter
    }

    fn needs_method_parameter(&self) -> bool {
        false // Local tools never need method parameters
    }

    fn create_handler(
        &self,
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<Box<dyn ToolHandler + Send>, McpError> {
        // Create LocalContext
        let local_context = LocalContext {
            handler: Arc::clone(&self.handler),
        };

        // Create a new HandlerContext with LocalContext
        let local_handler_context =
            HandlerContext::with_data(service, request, context, local_context);

        Ok(Box::new(LocalToolHandler::new(local_handler_context)))
    }

    fn clone_box(&self) -> Box<dyn ToolDefinition> {
        Box::new(Self {
            name:           self.name,
            description:    self.description,
            handler:        self.handler.clone(),
            parameters:     self.parameters.clone(),
            formatter:      self.formatter.clone(),
            port_parameter: self.port_parameter,
        })
    }
}
