//! Tool generation from declarative definitions.
//!
//! Converts declarative tool definitions into MCP tool registrations and request handlers.
//! Supports both BRP (remote) and local handlers with automatic response formatting.
//!
//! # Handler Types
//!
//! - **BRP handlers**: Execute remote method calls via Bevy Remote Protocol
//! - **Local handlers**: Execute functions within the MCP server
//!
//! # Response Formatting
//!
//! - **`LocalPassthrough`**: Preserves pre-structured responses (e.g., status operations)
//! - **`LocalStandard`**: Standard formatting for simple operations
//! - **`EntityOperation`/`ResourceOperation`**: BRP-specific operations with field extraction

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
        FormatterType::LocalStandard => ResponseFormatterFactory::local_standard(),
        FormatterType::LocalCollection => ResponseFormatterFactory::local_collection(),
        FormatterType::LocalPassthrough => ResponseFormatterFactory::local_passthrough(),
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
#[allow(clippy::too_many_lines)]
async fn generate_local_handler(
    def: &BrpToolDef,
    service: &BrpMcpService,
    request: CallToolRequestParam,
    context: RequestContext<RoleServer>,
    handler: &str,
) -> Result<CallToolResult, McpError> {
    let (formatter_factory, formatter_context) = create_formatter_from_def(def, &request);

    // Route to the appropriate local handler based on the handler name
    // and format the result using the ResponseFormatter
    match handler {
        HANDLER_LIST_LOGS => {
            let result = log_tools::list_logs::handle(&request)
                .map(|data| serde_json::to_value(data).unwrap_or(serde_json::Value::Null));
            format_handler_result(result, "list_logs", &formatter_factory, &formatter_context)
        }
        HANDLER_READ_LOG => {
            let result = log_tools::read_log::handle(&request)
                .map(|data| serde_json::to_value(data).unwrap_or(serde_json::Value::Null));
            format_handler_result(result, "read_log", &formatter_factory, &formatter_context)
        }
        HANDLER_CLEANUP_LOGS => {
            let result = log_tools::cleanup_logs::handle(&request)
                .map(|data| serde_json::to_value(data).unwrap_or(serde_json::Value::Null));
            format_handler_result(
                result,
                "cleanup_logs",
                &formatter_factory,
                &formatter_context,
            )
        }
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
        HANDLER_SHUTDOWN => format_handler_result(
            app_tools::brp_shutdown::handle(request).await,
            "shutdown",
            &formatter_factory,
            &formatter_context,
        ),
        HANDLER_STATUS => format_handler_result(
            app_tools::brp_status::handle(request).await,
            "brp_status",
            &formatter_factory,
            &formatter_context,
        ),
        HANDLER_GET_TRACE_LOG_PATH => {
            let result = log_tools::get_trace_log_path::handle();
            let value = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
            let metadata = crate::brp_tools::support::response_formatter::BrpMetadata::new(
                "get_trace_log_path",
                0,
            );
            Ok(formatter_factory
                .create(formatter_context.clone())
                .format_success(&value, metadata))
        }
        HANDLER_SET_TRACING_LEVEL => {
            let result = log_tools::set_tracing_level::handle(&request)
                .map(|data| serde_json::to_value(data).unwrap_or(serde_json::Value::Null));
            format_handler_result(
                result,
                "set_tracing_level",
                &formatter_factory,
                &formatter_context,
            )
        }
        HANDLER_BEVY_GET_WATCH => format_handler_result(
            brp_tools::watch::bevy_get_watch::handle(request).await,
            "bevy_get_watch",
            &formatter_factory,
            &formatter_context,
        ),
        HANDLER_BEVY_LIST_WATCH => format_handler_result(
            brp_tools::watch::bevy_list_watch::handle(service, request, context).await,
            "bevy_list_watch",
            &formatter_factory,
            &formatter_context,
        ),
        HANDLER_BRP_STOP_WATCH => format_handler_result(
            brp_tools::watch::brp_stop_watch::handle(service, request, context).await,
            "brp_stop_watch",
            &formatter_factory,
            &formatter_context,
        ),
        HANDLER_BRP_LIST_ACTIVE_WATCHES => format_handler_result(
            brp_tools::watch::brp_list_active::handle(service, request, context).await,
            "brp_list_active_watches",
            &formatter_factory,
            &formatter_context,
        ),
        _ => Err(error::report_to_mcp_error(
            &error_stack::Report::new(error::Error::ParameterExtraction(format!(
                "unknown local handler: {handler}"
            )))
            .attach_printable("Invalid handler parameter"),
        )),
    }
}

/// Create formatter factory and context from tool definition
fn create_formatter_from_def(
    def: &BrpToolDef,
    request: &CallToolRequestParam,
) -> (ResponseFormatterFactory, FormatterContext) {
    // Create the formatter factory based on the definition
    let mut formatter_builder = match &def.formatter.formatter_type {
        FormatterType::EntityOperation(field) => ResponseFormatterFactory::entity_operation(field),
        FormatterType::ResourceOperation => ResponseFormatterFactory::resource_operation(""),
        FormatterType::Simple => ResponseFormatterFactory::list_operation(),
        FormatterType::LocalStandard => ResponseFormatterFactory::local_standard(),
        FormatterType::LocalCollection => ResponseFormatterFactory::local_collection(),
        FormatterType::LocalPassthrough => ResponseFormatterFactory::local_passthrough(),
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

    // Create the formatter
    let formatter_factory = formatter_builder.build();
    let formatter_context = FormatterContext {
        params:           request.arguments.clone().map(serde_json::Value::Object),
        format_corrected: None,
    };

    (formatter_factory, formatter_context)
}

/// Format the result of a handler that returns `Result<Value, McpError>`
fn format_handler_result(
    result: Result<serde_json::Value, McpError>,
    method_name: &str,
    formatter_factory: &ResponseFormatterFactory,
    formatter_context: &FormatterContext,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => {
            let metadata =
                crate::brp_tools::support::response_formatter::BrpMetadata::new(method_name, 0);
            Ok(formatter_factory
                .create(formatter_context.clone())
                .format_success(&value, metadata))
        }
        Err(e) => Err(e),
    }
}

/// Convert our `ExtractorType` enum to the actual extractor function
fn convert_extractor_type(extractor_type: &ExtractorType) -> brp_tools::support::FieldExtractor {
    match extractor_type {
        ExtractorType::EntityFromParams => Box::new(extractors::entity_from_params),
        ExtractorType::ResourceFromParams => Box::new(extractors::resource_from_params),
        ExtractorType::PassThroughData => Box::new(extractors::pass_through_data),
        ExtractorType::PassThroughResult => Box::new(|data, _| data.clone()),
        ExtractorType::EntityCountFromData | ExtractorType::ComponentCountFromData => {
            Box::new(extractors::array_count)
        }
        ExtractorType::EntityFromResponse => Box::new(extract_entity_from_response),
        ExtractorType::QueryComponentCount => Box::new(extract_query_component_count),
        ExtractorType::QueryParamsFromContext => Box::new(extract_query_params_from_context),
        ExtractorType::ParamFromContext(param_name) => match *param_name {
            "components" => Box::new(|data, context| {
                extract_field_from_context(JSON_FIELD_COMPONENTS, data, context)
            }),
            "entities" => Box::new(|data, context| {
                extract_field_from_context(JSON_FIELD_ENTITIES, data, context)
            }),
            "parent" => Box::new(|data, context| {
                extract_field_from_context(JSON_FIELD_PARENT, data, context)
            }),
            "path" => {
                Box::new(|data, context| extract_field_from_context(JSON_FIELD_PATH, data, context))
            }
            "port" => {
                Box::new(|data, context| extract_field_from_context(JSON_FIELD_PORT, data, context))
            }
            _ => Box::new(|_data, _context| serde_json::Value::Null),
        },
        ExtractorType::CountFromData => Box::new(extractors::count_from_data),
        ExtractorType::MessageFromParams => Box::new(extractors::message_from_params),
        ExtractorType::DataField(field_name) => {
            let field = (*field_name).to_string();
            Box::new(move |data, _| data.get(&field).cloned().unwrap_or(serde_json::Value::Null))
        }
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
