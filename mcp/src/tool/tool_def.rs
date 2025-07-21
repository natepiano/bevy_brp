//! Unified tool definition that can handle both BRP and Local tools

use std::path::PathBuf;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolRequestParam;

use super::HandlerFn;
use super::annotations::BrpToolAnnotations;
use super::mcp_tool_schema::McpToolSchemaBuilder;
use super::types::{BrpMethodSource, ToolHandler};
use crate::constants::{PARAM_METHOD, PARAM_PORT};
use crate::field_extraction::{Parameter, ParameterFieldType, PortParameter};
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

    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
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
        use super::handler_context::{HandlerContext, HasMethod, HasPort, NoMethod, NoPort};
        use crate::tool::types::ToolContext;

        // Direct context creation - pure capability-based approach
        match &self.handler {
            HandlerFn::Local(_) => {
                // Create HandlerContext<NoPort, NoMethod>
                let ctx = HandlerContext::with_data(self.clone(), request, roots, NoPort, NoMethod);
                let tool_context = ToolContext::Local(ctx);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::LocalWithPort(_) => {
                // Extract port and create HandlerContext<HasPort, NoMethod>
                let port = extract_port_directly(&request)?;
                let ctx = HandlerContext::with_data(
                    self.clone(),
                    request,
                    roots,
                    HasPort { port },
                    NoMethod,
                );
                let tool_context = ToolContext::LocalWithPort(ctx);
                Ok(ToolHandler::new(self.handler.clone(), tool_context))
            }
            HandlerFn::Brp { method_source, .. } => {
                // Extract port and method, create HandlerContext<HasPort, HasMethod>
                let port = extract_port_directly(&request)?;
                let method = match method_source {
                    BrpMethodSource::Static(method_name) => (*method_name).to_string(),
                    BrpMethodSource::Dynamic => extract_method_directly(&request)?,
                };
                let ctx = HandlerContext::with_data(
                    self.clone(),
                    request,
                    roots,
                    HasPort { port },
                    HasMethod { method },
                );
                let tool_context = ToolContext::Brp(ctx);
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
                ParameterFieldType::String => {
                    builder.add_string_property(param.name(), param.description(), param.required())
                }
                ParameterFieldType::Number => {
                    builder.add_number_property(param.name(), param.description(), param.required())
                }
                ParameterFieldType::Boolean => builder.add_boolean_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParameterFieldType::StringArray => builder.add_string_array_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParameterFieldType::NumberArray => builder.add_number_array_property(
                    param.name(),
                    param.description(),
                    param.required(),
                ),
                ParameterFieldType::Any | ParameterFieldType::DynamicParams => {
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

        // Enhance title with category prefix and optional method name
        let enhanced_annotations = {
            let mut enhanced = self.annotations.clone();

            // Start with category prefix
            let category_prefix = enhanced.category.as_ref();
            let base_title = &enhanced.title;

            // Add method name for BRP tools
            let full_title = match &self.handler {
                HandlerFn::Brp {
                    method_source: BrpMethodSource::Static(method),
                    ..
                } => {
                    format!("{category_prefix}: {base_title} ({method})")
                }
                HandlerFn::Brp {
                    method_source: BrpMethodSource::Dynamic,
                    ..
                }
                | HandlerFn::Local(_)
                | HandlerFn::LocalWithPort(_) => {
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

/// Extract port parameter directly from request arguments\
/// Used during context creation, then discarded
fn extract_port_directly(request: &CallToolRequestParam) -> Result<u16, McpError> {
    use crate::constants::{DEFAULT_BRP_PORT, PARAM_PORT, VALID_PORT_RANGE};

    let port_u64 = request
        .arguments
        .as_ref()
        .and_then(|args| args.get(PARAM_PORT))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_else(|| u64::from(DEFAULT_BRP_PORT));

    let port = u16::try_from(port_u64).map_err(|_| {
        McpError::invalid_params("Invalid port parameter: value too large for u16", None)
    })?;

    // Validate port range (1024-65535 for non-privileged ports)
    if !VALID_PORT_RANGE.contains(&port) {
        return Err(McpError::invalid_params(
            format!(
                "Invalid port {port}: must be in range {}-{}",
                VALID_PORT_RANGE.start(),
                VALID_PORT_RANGE.end()
            ),
            None,
        ));
    }

    Ok(port)
}

/// Extract method parameter directly from request arguments
/// Used during context creation, then discarded
fn extract_method_directly(request: &CallToolRequestParam) -> Result<String, McpError> {
    use crate::constants::PARAM_METHOD;
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(PARAM_METHOD))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("Missing method parameter", None))
        .map(std::string::ToString::to_string)
}
