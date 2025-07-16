//! BRP tool definition type

use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::ToolHandler;
use super::brp_handler::BrpToolHandler;
use super::parameters::{BrpParameter, ParameterDefinition};
use super::tool_definition::{PortParameter, ToolDefinition};
use crate::response::ResponseSpecification;
use crate::service::McpService;

/// Source for BRP method name resolution
#[derive(Debug, Clone)]
pub enum BrpMethodSource {
    /// Static method name known at compile time
    Static(&'static str),
    /// Method name comes from parameter in request
    Dynamic,
}

/// Definition for BRP tools that communicate with Bevy Remote Protocol
pub struct BrpToolDef {
    /// Tool name
    pub name:            &'static str,
    /// Tool description
    pub description:     &'static str,
    /// BRP method name resolution strategy
    pub method_source:   BrpMethodSource,
    /// Type-safe BRP parameters (excludes port)
    pub parameters:      Vec<BrpParameter>,
    /// Response formatting specification
    pub response_format: ResponseSpecification,
}

impl ToolDefinition for BrpToolDef {
    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn formatter(&self) -> &ResponseSpecification {
        &self.response_format
    }

    fn parameters(&self) -> Vec<&dyn ParameterDefinition> {
        self.parameters
            .iter()
            .map(|p| p as &dyn ParameterDefinition)
            .collect()
    }

    fn port_parameter(&self) -> PortParameter {
        PortParameter::Required // All BRP tools require a port
    }

    fn needs_method_parameter(&self) -> bool {
        matches!(self.method_source, BrpMethodSource::Dynamic)
    }

    fn create_handler(
        &self,
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<Box<dyn ToolHandler + Send>, McpError> {
        // Create base context - the ONLY way to start
        let base_ctx = crate::service::HandlerContext::<crate::service::BaseContext>::new(
            service, request, context,
        );

        // Extract what we need
        let port = base_ctx.extract_port()?;
        let method = match &self.method_source {
            BrpMethodSource::Static(method_name) => (*method_name).to_string(),
            BrpMethodSource::Dynamic => base_ctx.extract_method_param()?,
        };

        // Transition to BrpContext
        let brp_handler_context = base_ctx.into_brp(method, port);

        Ok(Box::new(BrpToolHandler::new(brp_handler_context)))
    }

    fn clone_box(&self) -> Box<dyn ToolDefinition> {
        Box::new(Self {
            name:            self.name,
            description:     self.description,
            method_source:   self.method_source.clone(),
            parameters:      self.parameters.clone(),
            response_format: self.response_format.clone(),
        })
    }
}
