//! Field extraction functions for response formatting.
//!
//! This module provides the bridge between `ExtractorType` enum variants
//! and the actual field extraction functions used by the response formatter.

use serde_json::Value;

use super::{BevyResponseExtractor, ExtractorType, FormatterContext, McpCallExtractor};
use crate::brp_tools::constants::{
    JSON_FIELD_COMPONENTS, JSON_FIELD_ENTITIES, JSON_FIELD_PARENT, JSON_FIELD_PATH, JSON_FIELD_PORT,
};
use crate::response::{ResponseExtractorType, ResponseFieldV2};

/// Function type for extracting field values from response data and context.
///
/// Takes:
/// - `&Value` - The response data (usually from BRP)
/// - `&FormatterContext` - Context including request parameters
///
/// Returns: `Value` - The extracted field value
pub type FieldExtractor = Box<dyn Fn(&Value, &FormatterContext) -> Value + Send + Sync>;

/// Convert an `ExtractorType` enum variant to a field extractor function.
///
/// This creates the actual closure that will extract data based on the extraction strategy
/// defined by the `ExtractorType` variant.
pub fn convert_extractor_type(extractor_type: &ExtractorType) -> FieldExtractor {
    match extractor_type {
        ExtractorType::EntityFromParams => Box::new(|_data, context| {
            context
                .params
                .as_ref()
                .and_then(|params| {
                    let extractor = McpCallExtractor::new(params);
                    extractor.entity_id().map(|id| Value::Number(id.into()))
                })
                .unwrap_or(Value::Null)
        }),
        ExtractorType::ResourceFromParams => Box::new(|_data, context| {
            context
                .params
                .as_ref()
                .and_then(|params| {
                    let extractor = McpCallExtractor::new(params);
                    extractor
                        .resource_name()
                        .map(|name| Value::String(name.to_string()))
                })
                .unwrap_or(Value::Null)
        }),
        ExtractorType::PassThroughData => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).pass_through().clone())
        }
        ExtractorType::PassThroughResult => Box::new(|data, _| data.clone()),
        ExtractorType::EntityCountFromData | ExtractorType::ComponentCountFromData => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).entity_count().into())
        }
        ExtractorType::EntityFromResponse => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).spawned_entity_id())
        }
        ExtractorType::QueryComponentCount => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).query_component_count())
        }
        ExtractorType::QueryParamsFromContext => {
            Box::new(|_data, context| context.params.clone().unwrap_or(Value::Null))
        }
        ExtractorType::ParamFromContext(param_name) => {
            let field_name = match *param_name {
                "components" => JSON_FIELD_COMPONENTS,
                "entities" => JSON_FIELD_ENTITIES,
                "parent" => JSON_FIELD_PARENT,
                "path" => JSON_FIELD_PATH,
                "port" => JSON_FIELD_PORT,
                _ => return Box::new(|_data, _context| Value::Null),
            };
            Box::new(move |_data, context| {
                context
                    .params
                    .as_ref()
                    .and_then(|p| p.get(field_name))
                    .cloned()
                    .unwrap_or(Value::Null)
            })
        }
        ExtractorType::CountFromData => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).count())
        }
        ExtractorType::DataField(field_name) => {
            let field = (*field_name).to_string();
            Box::new(move |data, _| data.get(&field).cloned().unwrap_or(serde_json::Value::Null))
        }
    }
}

/// Create a field accessor for already-extracted request parameters.
///
/// This function creates extractors that reference fields from the `ExtractedParams`,
/// which were already validated during the parameter extraction phase.
pub fn create_request_field_accessor(field: &'static str) -> FieldExtractor {
    Box::new(move |_data, context| {
        match field {
            "entity" => {
                // Extract entity ID from request parameters
                context
                    .params
                    .as_ref()
                    .and_then(|params| {
                        let extractor = McpCallExtractor::new(params);
                        extractor.entity_id().map(|id| Value::Number(id.into()))
                    })
                    .unwrap_or(Value::Null)
            }
            "resource" => {
                // Extract resource name from request parameters
                context
                    .params
                    .as_ref()
                    .and_then(|params| {
                        let extractor = McpCallExtractor::new(params);
                        extractor
                            .resource_name()
                            .map(|name| Value::String(name.to_string()))
                    })
                    .unwrap_or(Value::Null)
            }
            "path" => {
                // Extract path parameter from request
                context
                    .params
                    .as_ref()
                    .and_then(|params| params.get("path"))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            "port" => {
                // Extract port parameter from request
                context
                    .params
                    .as_ref()
                    .and_then(|params| params.get("port"))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            "components" => {
                // Extract components parameter from request
                context
                    .params
                    .as_ref()
                    .and_then(|params| params.get("components"))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            "entities" => {
                // Extract entities parameter from request
                context
                    .params
                    .as_ref()
                    .and_then(|params| params.get("entities"))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            "parent" => {
                // Extract parent parameter from request
                context
                    .params
                    .as_ref()
                    .and_then(|params| params.get("parent"))
                    .cloned()
                    .unwrap_or(Value::Null)
            }
            _ => Value::Null,
        }
    })
}

/// Create an extractor for response data.
///
/// This function creates extractors that extract data from the response payload,
/// which is the appropriate place for response data extraction.
pub fn create_response_extractor(extractor: &ResponseExtractorType) -> FieldExtractor {
    match extractor {
        ResponseExtractorType::PassThroughData => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).pass_through().clone())
        }
        ResponseExtractorType::PassThroughRaw => {
            Box::new(|data, _context| {
                // Extract fields from the JSON-RPC result and return them directly
                // This will be merged at the top level as peers of status/message
                BevyResponseExtractor::new(data).pass_through().clone()
            })
        }
        ResponseExtractorType::Field(field_name) => {
            let field = (*field_name).to_string();
            Box::new(move |data, _| data.get(&field).cloned().unwrap_or(Value::Null))
        }
        ResponseExtractorType::Count => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).count())
        }
        ResponseExtractorType::EntityCount => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).entity_count().into())
        }
        ResponseExtractorType::ComponentCount => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).entity_count().into())
        }
        ResponseExtractorType::QueryComponentCount => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).query_component_count())
        }
        ResponseExtractorType::EntityId => {
            Box::new(|data, _context| BevyResponseExtractor::new(data).spawned_entity_id())
        }
    }
}

/// Convert a `ResponseFieldV2` specification to a field extractor function.
///
/// This creates the actual closure that will extract data based on the new
/// separated extraction strategy defined by the `ResponseFieldV2` variant.
pub fn convert_response_field_v2(field: &ResponseFieldV2) -> FieldExtractor {
    match field {
        ResponseFieldV2::FromRequest { field, .. } => create_request_field_accessor(field),
        ResponseFieldV2::FromResponse { extractor, .. } => create_response_extractor(extractor),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

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
}
