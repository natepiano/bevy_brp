use rmcp::Error as McpError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::traits::{ExtractedParams, ParamExtractor};
use crate::brp_tools::constants::{
    DEFAULT_BRP_PORT, JSON_FIELD_ENTITY, JSON_FIELD_PORT, JSON_FIELD_RESOURCE, PARAM_WITH_CRATES,
    PARAM_WITH_TYPES, PARAM_WITHOUT_CRATES, PARAM_WITHOUT_TYPES,
};
use crate::error::{Error, report_to_mcp_error};
use crate::support::params::{
    extract_any_value, extract_optional_number, extract_optional_string_array_from_request,
    extract_required_number, extract_required_string,
};

/// Parameters for BRP execute tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrpExecuteParams {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default = "default_port")]
    pub port:   u16,
}

const fn default_port() -> u16 {
    DEFAULT_BRP_PORT
}

/// Helper function to extract and validate port from request
fn extract_port(request: &rmcp::model::CallToolRequestParam) -> Result<u16, McpError> {
    u16::try_from(extract_optional_number(
        request,
        JSON_FIELD_PORT,
        u64::from(DEFAULT_BRP_PORT),
    )?)
    .map_err(|_| {
        report_to_mcp_error(
            &error_stack::Report::new(Error::ParameterExtraction(
                "Invalid port number".to_string(),
            ))
            .attach_printable("Port must be a valid u16")
            .attach_printable(format!("Field: {JSON_FIELD_PORT}")),
        )
    })
}

/// Simple parameter extractor that just extracts port
pub struct SimplePortExtractor;

impl ParamExtractor for SimplePortExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let port = extract_port(request)?;
        Ok(ExtractedParams {
            method: None,
            params: None,
            port,
        })
    }
}

/// Parameter extractor that extracts port and passes through all arguments as params
pub struct PassthroughExtractor;

impl ParamExtractor for PassthroughExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let port = extract_port(request)?;
        let params = request.arguments.clone().map(serde_json::Value::Object);
        Ok(ExtractedParams {
            method: None,
            params,
            port,
        })
    }
}

/// Parameter extractor for entity-based operations (destroy, list)
pub struct EntityParamExtractor {
    /// Whether entity is required
    pub required: bool,
}

impl ParamExtractor for EntityParamExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let port = extract_port(request)?;

        let params = if self.required {
            let entity = extract_required_number(request, JSON_FIELD_ENTITY)?;
            Some(json!({ JSON_FIELD_ENTITY: entity }))
        } else {
            // For optional entity (like in list)
            extract_any_value(request, JSON_FIELD_ENTITY)
                .and_then(serde_json::Value::as_u64)
                .map(|id| json!({ JSON_FIELD_ENTITY: id }))
        };

        Ok(ExtractedParams {
            method: None,
            params,
            port,
        })
    }
}

/// Parameter extractor for `brp_execute` operations
pub struct BrpExecuteExtractor;

impl ParamExtractor for BrpExecuteExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let params: BrpExecuteParams = serde_json::from_value(serde_json::Value::Object(
            request.arguments.clone().unwrap_or_default(),
        ))
        .map_err(|e| {
            report_to_mcp_error(
                &error_stack::Report::new(Error::ParameterExtraction(
                    "Invalid parameters for brp_execute".to_string(),
                ))
                .attach_printable(format!("Deserialization error: {e}"))
                .attach_printable("Expected valid BrpExecuteParams structure"),
            )
        })?;

        Ok(ExtractedParams {
            method: Some(params.method),
            params: params.params,
            port:   params.port,
        })
    }
}

/// Parameter extractor for resource-based operations
pub struct ResourceParamExtractor;

impl ParamExtractor for ResourceParamExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let port = extract_port(request)?;
        let resource = extract_required_string(request, JSON_FIELD_RESOURCE)?;

        let params = Some(json!({ JSON_FIELD_RESOURCE: resource }));

        Ok(ExtractedParams {
            method: None,
            params,
            port,
        })
    }
}

/// Parameter extractor for registry/schema method
///
/// Transforms individual filter parameters into the query structure expected by the BRP method:
/// - `with_crates`: Include only types from specified crates
/// - `without_crates`: Exclude types from specified crates
/// - `with_types`: Include only types with specified reflect traits
/// - `without_types`: Exclude types with specified reflect traits
pub struct RegistrySchemaParamExtractor;

impl ParamExtractor for RegistrySchemaParamExtractor {
    fn extract(
        &self,
        request: &rmcp::model::CallToolRequestParam,
    ) -> Result<ExtractedParams, McpError> {
        let port = extract_port(request)?;

        // Extract the individual filter parameters
        let with_crates = extract_optional_string_array_from_request(request, PARAM_WITH_CRATES)?;
        let without_crates =
            extract_optional_string_array_from_request(request, PARAM_WITHOUT_CRATES)?;
        let with_types = extract_optional_string_array_from_request(request, PARAM_WITH_TYPES)?;
        let without_types =
            extract_optional_string_array_from_request(request, PARAM_WITHOUT_TYPES)?;

        // Build the query object if any filters are provided
        // The BRP method expects a JSON object with filter fields
        let params = if with_crates.is_some()
            || without_crates.is_some()
            || with_types.is_some()
            || without_types.is_some()
        {
            let mut query = serde_json::Map::new();

            // Add crate filters
            if let Some(crates) = with_crates {
                query.insert(PARAM_WITH_CRATES.to_string(), json!(crates));
            }
            if let Some(crates) = without_crates {
                query.insert(PARAM_WITHOUT_CRATES.to_string(), json!(crates));
            }

            // Add reflect trait filters
            if let Some(types) = with_types {
                query.insert(PARAM_WITH_TYPES.to_string(), json!(types));
            }
            if let Some(types) = without_types {
                query.insert(PARAM_WITHOUT_TYPES.to_string(), json!(types));
            }

            // Return the query object directly as a Value::Object
            Some(Value::Object(query))
        } else {
            // No filters provided, return None to get all schemas
            None
        };

        Ok(ExtractedParams {
            method: None,
            params,
            port,
        })
    }
}
