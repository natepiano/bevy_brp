//! Local tool definition type

use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::local_handler::LocalToolHandler;
use super::parameters::{LocalParameter, ParameterDefinition};
use super::tool_definition::{PortParameter, ToolDefinition};
use crate::response::ResponseSpecification;
use crate::service::{LocalHandler, McpService};
use crate::tool::ToolHandler;

/// Definition for local tools that execute within the MCP server
pub struct LocalToolDef {
    /// Tool name
    pub name:        &'static str,
    /// Tool description
    pub description: &'static str,
    /// Handler function
    pub handler:     LocalHandler,
    /// Type-safe local parameters
    pub parameters:  Vec<LocalParameter>,
    /// Response formatting specification
    pub formatter:   ResponseSpecification,
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
        use crate::service::LocalHandler;

        match &self.handler {
            LocalHandler::Basic(_) => PortParameter::NotUsed,
            LocalHandler::WithPort(_) => PortParameter::Required,
        }
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
        use crate::service::{BaseContext, HandlerContext};

        // Create base context - the ONLY way to start
        let base_ctx = HandlerContext::<BaseContext>::new(service, request, context);

        // Extract port only if handler needs it
        let port = match &self.handler {
            LocalHandler::Basic(_) => None,
            LocalHandler::WithPort(_) => Some(base_ctx.extract_port()?),
        };

        // Use the handler directly - no conversion needed
        let local_handler_context = base_ctx.into_local(self.handler.clone(), port);
        Ok(Box::new(LocalToolHandler::new(local_handler_context)))
    }

    fn clone_box(&self) -> Box<dyn ToolDefinition> {
        Box::new(Self {
            name:        self.name,
            description: self.description,
            handler:     self.handler.clone(),
            parameters:  self.parameters.clone(),
            formatter:   self.formatter.clone(),
        })
    }
}
