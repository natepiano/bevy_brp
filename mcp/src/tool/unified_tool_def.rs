//! Unified tool definition that can handle both BRP and Local tools

use std::sync::Arc;

use rmcp::model::CallToolRequestParam;
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use super::brp_tool_def::BrpMethodSource;
use super::parameters::{ParameterDefinition, UnifiedParameter};
use super::tool_definition::{PortParameter, ToolDefinition};
use super::unified_handler::UnifiedToolHandler;
use super::{HandlerFn, ToolHandler};
use crate::response::ResponseSpecification;
use crate::service::McpService;

/// Unified tool definition that can handle both BRP and Local tools
pub struct UnifiedToolDef {
    /// Tool name
    pub name: &'static str,
    /// Tool description  
    pub description: &'static str,
    /// Handler function with method source information
    pub handler: HandlerFn,
    /// Type-safe parameters (unified)
    pub parameters: Vec<UnifiedParameter>,
    /// Response formatting specification
    pub response_format: ResponseSpecification,
}

impl ToolDefinition for UnifiedToolDef {
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
        match &self.handler {
            HandlerFn::Local(_) => PortParameter::NotUsed,
            HandlerFn::LocalWithPort(_) | HandlerFn::Brp { .. } => PortParameter::Required,
        }
    }

    fn needs_method_parameter(&self) -> bool {
        match &self.handler {
            HandlerFn::Brp { method_source, .. } => {
                matches!(method_source, BrpMethodSource::Dynamic)
            }
            _ => false, // Local tools never need method parameters
        }
    }

    fn create_handler(
        &self,
        service: Arc<McpService>,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<Box<dyn ToolHandler + Send>, McpError> {
        use crate::service::{BaseContext, HandlerContext};
        use crate::tool::types::ToolContext;

        // Create base context - the ONLY way to start
        let base_ctx = HandlerContext::<BaseContext>::new(service, request, context);

        match &self.handler {
            HandlerFn::Local(_) => {
                // Local tool - no port needed
                let local_handler_context = base_ctx.into_local(None);
                let tool_context = ToolContext::Local(local_handler_context);
                Ok(Box::new(UnifiedToolHandler::new(
                    self.handler.clone(),
                    tool_context,
                )))
            }
            HandlerFn::LocalWithPort(_) => {
                // Local tool with port
                let port = base_ctx.extract_port()?;
                let local_handler_context = base_ctx.into_local(Some(port));
                let tool_context = ToolContext::Local(local_handler_context);
                Ok(Box::new(UnifiedToolHandler::new(
                    self.handler.clone(),
                    tool_context,
                )))
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

                Ok(Box::new(UnifiedToolHandler::new(
                    self.handler.clone(),
                    tool_context,
                )))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn ToolDefinition> {
        Box::new(Self {
            name: self.name,
            description: self.description,
            handler: self.handler.clone(),
            parameters: self.parameters.clone(),
            response_format: self.response_format.clone(),
        })
    }
}