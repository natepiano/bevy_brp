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

use rmcp::Error as McpError;
use rmcp::model::{CallToolRequestParam, CallToolResult, Tool};

use super::definitions::{BrpMethodParamCategory, McpToolDef};
use super::parameters::ParamType;
use crate::brp_tools::request_handler::{BrpHandlerConfig, handle_brp_method_tool_call};
use crate::constants::{
    JSON_FIELD_ENTITY, JSON_FIELD_RESOURCE, PARAM_PARAMS, PARAM_WITH_CRATES, PARAM_WITH_TYPES,
    PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::extractors::{ExtractedParams, McpCallExtractor};
use crate::response::{
    self, FormatterContext, ResponseBuilder, ResponseFormatterFactory, ResponseSpecification,
};
use crate::service::{BrpContext, HandlerContext, LocalContext};
use crate::support::schema;

/// Generate tool registration from a declarative definition
pub fn get_tool(def: McpToolDef) -> Tool {
    let mut builder = schema::SchemaBuilder::new();

    // Add all parameters to the schema
    for param in &def.parameters {
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

/// Extract parameters based on the extractor type
fn extract_params_for_type(
    extractor_type: &BrpMethodParamCategory,
    request: &CallToolRequestParam,
) -> Result<ExtractedParams, McpError> {
    let extractor = McpCallExtractor::from_request(request);
    let port = extractor.get_port()?;

    match extractor_type {
        BrpMethodParamCategory::Passthrough => {
            // Pass through all arguments as params
            let params = request.arguments.clone().map(serde_json::Value::Object);
            Ok(ExtractedParams { params, port })
        }
        BrpMethodParamCategory::Entity { required } => {
            // Extract entity parameter
            let params = if *required {
                let entity = extractor.get_entity_id()?;
                Some(serde_json::json!({ JSON_FIELD_ENTITY: entity }))
            } else {
                // For optional entity (like list), include it if present
                extractor
                    .entity_id()
                    .map(|entity| serde_json::json!({ JSON_FIELD_ENTITY: entity }))
            };

            Ok(ExtractedParams { params, port })
        }
        BrpMethodParamCategory::Resource => {
            // Extract resource parameter
            let resource = extractor.get_required_string(JSON_FIELD_RESOURCE, "resource name")?;
            let params = Some(serde_json::json!({ JSON_FIELD_RESOURCE: resource }));

            Ok(ExtractedParams { params, port })
        }
        BrpMethodParamCategory::EmptyParams => {
            // Just extract port, no other params
            Ok(ExtractedParams { params: None, port })
        }
        BrpMethodParamCategory::BrpExecute => {
            // Extract params for brp_execute
            let params = extractor.field(PARAM_PARAMS).cloned();

            Ok(ExtractedParams { params, port })
        }
        BrpMethodParamCategory::RegistrySchema => {
            // Extract optional filter parameters for registry schema
            let with_crates = extractor.optional_string_array(PARAM_WITH_CRATES);
            let without_crates = extractor.optional_string_array(PARAM_WITHOUT_CRATES);
            let with_types = extractor.optional_string_array(PARAM_WITH_TYPES);
            let without_types = extractor.optional_string_array(PARAM_WITHOUT_TYPES);

            let mut params_obj = serde_json::Map::new();
            if let Some(crates) = with_crates {
                params_obj.insert(PARAM_WITH_CRATES.to_string(), serde_json::json!(crates));
            }
            if let Some(crates) = without_crates {
                params_obj.insert(PARAM_WITHOUT_CRATES.to_string(), serde_json::json!(crates));
            }
            if let Some(types) = with_types {
                params_obj.insert(PARAM_WITH_TYPES.to_string(), serde_json::json!(types));
            }
            if let Some(types) = without_types {
                params_obj.insert(PARAM_WITHOUT_TYPES.to_string(), serde_json::json!(types));
            }

            let params = if params_obj.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(params_obj))
            };

            Ok(ExtractedParams { params, port })
        }
    }
}

/// Generate a `LocalFn` handler using function pointer approach
pub async fn local_tool_call(
    handler_context: &HandlerContext<LocalContext>,
) -> Result<CallToolResult, McpError> {
    let handler = handler_context.handler().as_ref();

    let (formatter_factory, formatter_context) = create_formatter_from_def(handler_context)?;

    // Handler returns typed result, we ALWAYS pass it through format_handler_result
    let result = handler
        .call(handler_context)
        .await
        .map(|typed_result| typed_result.to_json());

    format_tool_call_result(
        result,
        handler_context,
        &formatter_factory,
        &formatter_context,
    )
}

/// Build a formatter factory from a tool definition's response specification
fn build_formatter_factory_from_spec(
    formatter_spec: &ResponseSpecification,
) -> ResponseFormatterFactory {
    // Create the formatter factory for structured responses
    let mut formatter_builder = ResponseFormatterFactory::standard();

    // Set the template if provided
    let template = formatter_spec.message_template;
    if !template.is_empty() {
        formatter_builder = formatter_builder.with_template(template);
    }

    // Add response fields
    for field in &formatter_spec.response_fields {
        let (extractor, placement) = response::convert_response_field(field);
        formatter_builder =
            formatter_builder.with_response_field_placed(field.name(), extractor, placement);
    }

    formatter_builder.build()
}

/// Generate a BRP handler
pub async fn brp_method_tool_call(
    handler_context: &HandlerContext<BrpContext>,
) -> Result<CallToolResult, McpError> {
    let tool_def = handler_context.tool_def()?;

    // Extract parameters directly based on the definition
    let extracted_params =
        extract_params_for_type(&tool_def.parameter_extractor, &handler_context.request)?;

    // Use the shared function to build the formatter factory
    let formatter_factory = build_formatter_factory_from_spec(&tool_def.formatter);

    let config = BrpHandlerConfig {
        extracted_params,
        formatter_factory,
    };

    handle_brp_method_tool_call(handler_context.clone(), &config).await
}

/// Create formatter factory and context from tool definition
fn create_formatter_from_def(
    handler_context: &HandlerContext<LocalContext>,
) -> Result<(ResponseFormatterFactory, FormatterContext), McpError> {
    let tool_def = handler_context.tool_def()?;

    // Use the shared function to build the formatter factory
    let formatter_factory = build_formatter_factory_from_spec(&tool_def.formatter);

    let formatter_context = FormatterContext {
        params:           handler_context
            .request
            .arguments
            .clone()
            .map(serde_json::Value::Object),
        format_corrected: None,
    };

    Ok((formatter_factory, formatter_context))
}

/// Format the result of a handler that returns `Result<Value, McpError>` using `HandlerContext`
/// todo: address the cognitive load of all of these conditionals - can we refactor to use the type
/// system?
fn format_tool_call_result(
    result: Result<serde_json::Value, McpError>,
    handler_context: &HandlerContext<LocalContext>,
    formatter_factory: &ResponseFormatterFactory,
    formatter_context: &FormatterContext,
) -> Result<CallToolResult, McpError> {
    match result {
        Ok(value) => {
            // Check if the value contains a status field indicating an error
            let is_error = value
                .get("status")
                .and_then(|s| s.as_str())
                .is_some_and(|s| s == "error");

            let call_info = handler_context.call_info();

            if is_error {
                // For error responses, build the error response directly
                let message = value
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Operation failed");

                // Build error response with all the data fields
                let error_response = ResponseBuilder::error(call_info.clone()).message(message);

                // For disambiguation errors, only include specific fields
                let error_response = if let serde_json::Value::Object(map) = &value {
                    // Check if this is a disambiguation error by looking for duplicate_paths
                    let is_disambiguation = map
                        .get("duplicate_paths")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| !arr.is_empty());

                    if is_disambiguation {
                        // For disambiguation errors, only include the name field and
                        // duplicate_paths
                        map.iter()
                            .filter(|(key, val)| {
                                let k = key.as_str();
                                k != "status"
                                    && k != "message"
                                    && (k == "duplicate_paths"
                                        || k == "app_name"
                                        || k == "example_name")
                                    && !val.is_null()
                            })
                            .try_fold(error_response, |builder, (key, val)| {
                                builder.add_field(key, val)
                            })
                            .unwrap_or_else(|_| {
                                // If adding fields failed, just return the basic error response
                                ResponseBuilder::error(call_info.clone()).message(message)
                            })
                    } else {
                        // For other errors, include all non-null fields
                        map.iter()
                            .filter(|(key, val)| {
                                key.as_str() != "status"
                                    && key.as_str() != "message"
                                    && !val.is_null()
                            })
                            .try_fold(error_response, |builder, (key, val)| {
                                builder.add_field(key, val)
                            })
                            .unwrap_or_else(|_| {
                                // If adding fields failed, just return the basic error response
                                ResponseBuilder::error(call_info.clone()).message(message)
                            })
                    }
                } else {
                    error_response
                };

                Ok(error_response.build().to_call_tool_result())
            } else {
                // Use the new format_success_with_context to include call_info
                Ok(formatter_factory
                    .create(formatter_context.clone())
                    .format_success(&value, handler_context))
            }
        }
        Err(e) => Err(e),
    }
}
