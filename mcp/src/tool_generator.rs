//! Tool generation from declarative definitions.
//!
//! This module provides the runtime code generation logic that converts declarative
//! tool definitions into actual MCP tool registrations and request handlers. It acts
//! as the bridge between the static tool definitions in `tool_definitions.rs` and
//! the runtime tool handling system.
//!
//! # Architecture
//!
//! The generator provides two main functions:
//! - **`generate_tool_registration()`**: Converts a `BrpToolDef` into an MCP `Tool` for
//!   registration
//! - **`generate_tool_handler()`**: Creates a request handler that processes tool calls
//!
//! # Code Generation Process
//!
//! ## Tool Registration Generation
//! ```text
//! BrpToolDef → generate_tool_registration() → rmcp::model::Tool
//! ```
//!
//! The registration generator:
//! 1. Extracts tool name and description
//! 2. Builds JSON schema from parameter definitions
//! 3. Creates the MCP Tool structure for discovery
//!
//! ## Handler Generation
//! ```text
//! BrpToolDef + Request → generate_tool_handler() → CallToolResult
//! ```
//!
//! The handler generator:
//! 1. Selects appropriate parameter extractor based on tool definition
//! 2. Builds response formatter from formatter definition
//! 3. Configures BRP request handling pipeline
//! 4. Executes the BRP request and formats the response
//!
//! # Parameter Extractors
//!
//! Different tool types use different parameter extraction strategies:
//!
//! - **`Passthrough`**: Passes all parameters unchanged to BRP
//! - **`Entity`**: Extracts entity ID (required or optional)
//! - **`Resource`**: Extracts resource name parameter
//! - **`EmptyParams`**: No parameters (e.g., list operations)
//! - **`BrpExecute`**: Dynamic method selection from parameters
//! - **`RegistrySchema`**: Complex parameter transformation for registry queries
//!
//! # Response Formatters
//!
//! Response formatting is configured based on the operation type:
//!
//! - **`EntityOperation`**: Operations on specific entities with dynamic field extraction
//! - **`ResourceOperation`**: Operations on specific resources
//! - **`PassThrough`**: Direct response forwarding
//! - **`ListOperation`**: List-based responses
//!
//! # Example Usage
//!
//! ```rust
//! // Generate tool registration for discovery
//! let tool = generate_tool_registration(&brp_tool_def);
//!
//! // Handle incoming request
//! let result = generate_tool_handler(&brp_tool_def, service, request, context).await?;
//! ```
//!
//! # Error Handling
//!
//! The generator handles errors at multiple levels:
//! - Parameter validation errors
//! - BRP communication errors
//! - Response formatting errors
//! - Invalid tool definition errors
//!
//! All errors are converted to appropriate MCP error responses with helpful messages.

use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};
use rmcp::service::RequestContext;
use rmcp::{Error as McpError, RoleServer};

use crate::brp_tools::constants::{
    JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITIES, JSON_FIELD_ENTITY, JSON_FIELD_PARENT,
    JSON_FIELD_PATH, JSON_FIELD_PORT,
};
use crate::brp_tools::request_handler::{
    BrpExecuteExtractor, BrpHandlerConfig, EntityParamExtractor, FormatterContext, ParamExtractor,
    PassthroughExtractor, RegistrySchemaParamExtractor, ResourceParamExtractor,
    SimplePortExtractor, handle_brp_request,
};
use crate::brp_tools::support::{ResponseFormatterFactory, extractors};
use crate::support::schema;
use crate::tool_definitions::{
    BrpToolDef, ExtractorType, FormatterType, HandlerType, ParamExtractorType, ParamType,
};
use crate::tools::{
    HANDLER_BEVY_GET_WATCH, HANDLER_BEVY_LIST_WATCH, HANDLER_BRP_LIST_ACTIVE_WATCHES,
    HANDLER_BRP_STOP_WATCH, HANDLER_CLEANUP_LOGS, HANDLER_GET_TRACE_LOG_PATH,
    HANDLER_LAUNCH_BEVY_APP, HANDLER_LAUNCH_BEVY_EXAMPLE, HANDLER_LIST_BEVY_APPS,
    HANDLER_LIST_BEVY_EXAMPLES, HANDLER_LIST_BRP_APPS, HANDLER_LIST_LOGS, HANDLER_READ_LOG,
    HANDLER_SET_TRACING_LEVEL, HANDLER_SHUTDOWN, HANDLER_STATUS,
};
use crate::{BrpMcpService, app_tools, brp_tools, error, log_tools};

/// Generate tool registration from a declarative definition
pub fn generate_tool_registration(def: &BrpToolDef) -> Tool {
    let mut builder = schema::SchemaBuilder::new();

    // Add all parameters to the schema
    for param in &def.params {
        builder = match param.param_type {
            ParamType::Number => {
                builder.add_number_property(param.name, param.description, param.required)
            }
            ParamType::String => {
                builder.add_string_property(param.name, param.description, param.required)
            }
            ParamType::Boolean => {
                builder.add_boolean_property(param.name, param.description, param.required)
            }
            ParamType::StringArray => {
                builder.add_string_array_property(param.name, param.description, param.required)
            }
            ParamType::Any => {
                builder.add_any_property(param.name, param.description, param.required)
            }
        };
    }

    Tool {
        name:         def.name.into(),
        description:  def.description.into(),
        input_schema: builder.build(),
    }
}

/// Generate a handler function for a declarative tool definition
pub async fn generate_tool_handler(
    def: &BrpToolDef,
    service: &BrpMcpService,
    request: CallToolRequestParam,
    context: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    match &def.handler {
        HandlerType::Brp { method } => {
            // Handle BRP method calls
            generate_brp_handler(def, request, method).await
        }
        HandlerType::Local { handler } => {
            // Handle local method calls
            generate_local_handler(def, service, request, context, handler).await
        }
    }
}

/// Generate a BRP handler
async fn generate_brp_handler(
    def: &BrpToolDef,
    request: CallToolRequestParam,
    method: &'static str,
) -> Result<CallToolResult, McpError> {
    // Create the parameter extractor based on the definition
    let param_extractor: Box<dyn ParamExtractor> = match &def.param_extractor {
        ParamExtractorType::Passthrough => Box::new(PassthroughExtractor),
        ParamExtractorType::Entity { required } => Box::new(EntityParamExtractor {
            required: *required,
        }),
        ParamExtractorType::Resource => Box::new(ResourceParamExtractor),
        ParamExtractorType::EmptyParams => Box::new(SimplePortExtractor),
        ParamExtractorType::BrpExecute => Box::new(BrpExecuteExtractor),
        ParamExtractorType::RegistrySchema => Box::new(RegistrySchemaParamExtractor),
    };

    // Create the formatter factory based on the definition
    let mut formatter_builder = match &def.formatter.formatter_type {
        FormatterType::EntityOperation(field) => ResponseFormatterFactory::entity_operation(field),
        FormatterType::ResourceOperation => ResponseFormatterFactory::resource_operation(""),
        FormatterType::Simple => ResponseFormatterFactory::list_operation(),
    };

    // Set the template if provided
    if !def.formatter.template.is_empty() {
        formatter_builder = formatter_builder.with_template(def.formatter.template);
    }

    // Add response fields
    for field in &def.formatter.response_fields {
        formatter_builder = formatter_builder
            .with_response_field(field.name, convert_extractor_type(&field.extractor));
    }

    // All errors now route through format_error_default automatically

    let config = BrpHandlerConfig {
        method: Some(method),
        param_extractor,
        formatter_factory: formatter_builder.build(),
    };

    handle_brp_request(request, &config).await
}

/// Generate a local handler
async fn generate_local_handler(
    _def: &BrpToolDef,
    service: &BrpMcpService,
    request: CallToolRequestParam,
    context: RequestContext<RoleServer>,
    handler: &str,
) -> Result<CallToolResult, McpError> {
    // Route to the appropriate local handler based on the handler name
    match handler {
        HANDLER_LIST_LOGS => log_tools::list_logs::handle(&request),
        HANDLER_READ_LOG => log_tools::read_log::handle(&request),
        HANDLER_CLEANUP_LOGS => log_tools::cleanup_logs::handle(&request),
        HANDLER_LIST_BEVY_APPS => app_tools::brp_list_bevy_apps::handle(service, context).await,
        HANDLER_LIST_BRP_APPS => app_tools::brp_list_brp_apps::handle(service, context).await,
        HANDLER_LIST_BEVY_EXAMPLES => {
            app_tools::brp_list_bevy_examples::handle(service, context).await
        }
        HANDLER_LAUNCH_BEVY_APP => {
            app_tools::brp_launch_bevy_app::handle(service, request, context).await
        }
        HANDLER_LAUNCH_BEVY_EXAMPLE => {
            app_tools::brp_launch_bevy_example::handle(service, request, context).await
        }
        HANDLER_SHUTDOWN => app_tools::brp_shutdown::handle(request).await,
        HANDLER_STATUS => app_tools::brp_status::handle(request).await,
        HANDLER_GET_TRACE_LOG_PATH => log_tools::get_trace_log_path::handle(),
        HANDLER_SET_TRACING_LEVEL => log_tools::set_tracing_level::handle(&request),
        HANDLER_BEVY_GET_WATCH => brp_tools::watch::bevy_get_watch::handle(request).await,
        HANDLER_BEVY_LIST_WATCH => {
            brp_tools::watch::bevy_list_watch::handle(service, request, context).await
        }
        HANDLER_BRP_STOP_WATCH => {
            brp_tools::watch::brp_stop_watch::handle(service, request, context).await
        }
        HANDLER_BRP_LIST_ACTIVE_WATCHES => {
            brp_tools::watch::brp_list_active::handle(service, request, context).await
        }
        _ => Err(error::report_to_mcp_error(
            &error_stack::Report::new(error::Error::ParameterExtraction(format!(
                "unknown local handler: {handler}"
            )))
            .attach_printable("Invalid handler parameter"),
        )),
    }
}

/// Convert our `ExtractorType` enum to the actual extractor function
fn convert_extractor_type(extractor_type: &ExtractorType) -> brp_tools::support::FieldExtractor {
    // Pre-create common extractors to avoid repetitive closures
    static PASS_THROUGH_RESULT: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, _| data.clone();
    static NULL_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |_data, _context| serde_json::Value::Null;
    static COMPONENTS_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, context| extract_field_from_context(JSON_FIELD_COMPONENTS, data, context);
    static ENTITIES_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, context| extract_field_from_context(JSON_FIELD_ENTITIES, data, context);
    static PARENT_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, context| extract_field_from_context(JSON_FIELD_PARENT, data, context);
    static PATH_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, context| extract_field_from_context(JSON_FIELD_PATH, data, context);
    static PORT_EXTRACTOR: fn(&serde_json::Value, &FormatterContext) -> serde_json::Value =
        |data, context| extract_field_from_context(JSON_FIELD_PORT, data, context);

    match extractor_type {
        ExtractorType::EntityFromParams => extractors::entity_from_params,
        ExtractorType::ResourceFromParams => extractors::resource_from_params,
        ExtractorType::PassThroughData => extractors::pass_through_data,
        ExtractorType::PassThroughResult => PASS_THROUGH_RESULT,
        ExtractorType::EntityCountFromData | ExtractorType::ComponentCountFromData => {
            extractors::array_count
        }
        ExtractorType::EntityFromResponse => extract_entity_from_response,
        ExtractorType::QueryComponentCount => extract_query_component_count,
        ExtractorType::QueryParamsFromContext => extract_query_params_from_context,
        ExtractorType::ParamFromContext(param_name) => match *param_name {
            "components" => COMPONENTS_EXTRACTOR,
            "entities" => ENTITIES_EXTRACTOR,
            "parent" => PARENT_EXTRACTOR,
            "path" => PATH_EXTRACTOR,
            "port" => PORT_EXTRACTOR,
            _ => NULL_EXTRACTOR,
        },
    }
}

/// Extract entity ID from response data (for spawn operation)
fn extract_entity_from_response(
    data: &serde_json::Value,
    _context: &FormatterContext,
) -> serde_json::Value {
    data.get(JSON_FIELD_ENTITY)
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Number(serde_json::Number::from(0)))
}

/// Extract total component count from nested query results
fn extract_query_component_count(
    data: &serde_json::Value,
    _context: &FormatterContext,
) -> serde_json::Value {
    let total = data.as_array().map_or(0, |entities| {
        entities
            .iter()
            .filter_map(|e| e.as_object())
            .map(serde_json::Map::len)
            .sum::<usize>()
    });
    serde_json::Value::Number(serde_json::Number::from(total))
}

/// Extract query parameters from request context
fn extract_query_params_from_context(
    _data: &serde_json::Value,
    context: &FormatterContext,
) -> serde_json::Value {
    context.params.clone().unwrap_or(serde_json::Value::Null)
}

/// Generic field extraction from context parameters
fn extract_field_from_context(
    field_name: &str,
    _data: &serde_json::Value,
    context: &FormatterContext,
) -> serde_json::Value {
    context
        .params
        .as_ref()
        .and_then(|p| p.get(field_name))
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::tool_definitions::{FormatterDef, ParamDef, ParamType};

    #[test]
    fn test_generate_tool_registration() {
        let def = BrpToolDef {
            name:            "test_tool",
            description:     "A test tool",
            handler:         HandlerType::Brp {
                method: "test/method",
            },
            params:          vec![
                ParamDef {
                    name:        "entity",
                    description: "Entity ID",
                    required:    true,
                    param_type:  ParamType::Number,
                },
                ParamDef {
                    name:        "optional_param",
                    description: "Optional parameter",
                    required:    false,
                    param_type:  ParamType::String,
                },
            ],
            param_extractor: ParamExtractorType::Passthrough,
            formatter:       FormatterDef {
                formatter_type:  FormatterType::Simple,
                template:        "Test successful",
                response_fields: vec![],
            },
        };

        let tool = generate_tool_registration(&def);

        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
        assert!(tool.input_schema.contains_key("type"));
        assert_eq!(tool.input_schema.get("type"), Some(&"object".into()));
    }

    #[test]
    fn test_convert_extractor_type_pass_through_result() {
        let extractor = convert_extractor_type(&ExtractorType::PassThroughResult);
        let test_data = json!({"key": "value"});
        let context = FormatterContext {
            params:           None,
            format_corrected: None,
        };

        let result = extractor(&test_data, &context);
        assert_eq!(result, test_data);
    }

    #[test]
    fn test_convert_extractor_type_param_from_context() {
        let extractor = convert_extractor_type(&ExtractorType::ParamFromContext("components"));
        let test_data = json!({});
        let context = FormatterContext {
            params:           Some(json!({"components": ["Component1", "Component2"]})),
            format_corrected: None,
        };

        let result = extractor(&test_data, &context);
        assert_eq!(result, json!(["Component1", "Component2"]));
    }

    #[test]
    fn test_convert_extractor_type_unknown_param() {
        let extractor = convert_extractor_type(&ExtractorType::ParamFromContext("unknown"));
        let test_data = json!({});
        let context = FormatterContext {
            params:           Some(json!({"components": ["Component1"]})),
            format_corrected: None,
        };

        let result = extractor(&test_data, &context);
        assert_eq!(result, serde_json::Value::Null);
    }

    #[test]
    fn test_convert_extractor_type_path_param() {
        let extractor = convert_extractor_type(&ExtractorType::ParamFromContext("path"));
        let test_data = json!({});
        let context = FormatterContext {
            params:           Some(json!({"path": "/tmp/screenshot.png", "port": 15702})),
            format_corrected: None,
        };

        let result = extractor(&test_data, &context);
        assert_eq!(result, json!("/tmp/screenshot.png"));
    }

    #[test]
    fn test_convert_extractor_type_port_param() {
        let extractor = convert_extractor_type(&ExtractorType::ParamFromContext("port"));
        let test_data = json!({});
        let context = FormatterContext {
            params:           Some(json!({"path": "/tmp/screenshot.png", "port": 15702})),
            format_corrected: None,
        };

        let result = extractor(&test_data, &context);
        assert_eq!(result, json!(15702));
    }

    #[test]
    fn test_extract_entity_from_response() {
        let data = json!({"entity": 123});
        let context = FormatterContext {
            params:           None,
            format_corrected: None,
        };

        let result = extract_entity_from_response(&data, &context);
        assert_eq!(result, json!(123));
    }

    #[test]
    fn test_extract_entity_from_response_missing() {
        let data = json!({});
        let context = FormatterContext {
            params:           None,
            format_corrected: None,
        };

        let result = extract_entity_from_response(&data, &context);
        assert_eq!(result, json!(0));
    }

    #[test]
    fn test_extract_query_component_count() {
        let data = json!([
            {"Component1": {}, "Component2": {}},
            {"Component1": {}}
        ]);
        let context = FormatterContext {
            params:           None,
            format_corrected: None,
        };

        let result = extract_query_component_count(&data, &context);
        assert_eq!(result, json!(3)); // 2 + 1 components
    }

    #[test]
    fn test_extract_query_params_from_context() {
        let data = json!({});
        let test_params = json!({"filter": {"with": ["Transform"]}});
        let context = FormatterContext {
            params:           Some(test_params.clone()),
            format_corrected: None,
        };

        let result = extract_query_params_from_context(&data, &context);
        assert_eq!(result, test_params);
    }

    #[test]
    fn test_extract_field_from_context() {
        let data = json!({});
        let context = FormatterContext {
            params:           Some(json!({"components": ["Transform"], "entity": 42})),
            format_corrected: None,
        };

        let result = extract_field_from_context("components", &data, &context);
        assert_eq!(result, json!(["Transform"]));

        let result = extract_field_from_context("entity", &data, &context);
        assert_eq!(result, json!(42));

        let result = extract_field_from_context("missing", &data, &context);
        assert_eq!(result, serde_json::Value::Null);
    }
}
