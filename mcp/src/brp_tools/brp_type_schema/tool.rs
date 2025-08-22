//! `brp_type_schema` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide the same information as `brp_extras_discover_format`.

use std::collections::HashMap;

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::registry_cache::get_full_registry;
use super::result_types::{TypeInfo, TypeSchemaResponse, TypeSchemaSummary};
use super::types::BrpTypeName;
use crate::brp_tools::Port;
use crate::error::Result;
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Parameters for the `brp_type_schema` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TypeSchemaParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,

    /// Force refresh of the type registry cache (default: false)
    /// Use this when you've made changes to your Bevy app's types and need to fetch the latest
    /// registry information
    #[serde(default)]
    pub refresh_cache: bool,
}

/// Result for the `brp_type_schema` tool
#[derive(Debug, Clone, Serialize, Deserialize, ResultStruct)]
pub struct TypeSchemaResult {
    /// The type schema information containing format discovery results
    #[to_result]
    result: TypeSchemaResponse,

    /// Count of types discovered
    #[to_metadata]
    type_count: usize,

    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

/// V2 engine for type schema generation using complete registry approach
struct TypeSchemaEngine {
    registry: HashMap<BrpTypeName, Value>,
}

impl TypeSchemaEngine {
    /// Create a new V2 engine instance by fetching the complete registry
    ///
    /// If `refresh_cache` is true, the registry cache will be cleared before fetching,
    /// ensuring fresh type information is retrieved.
    async fn new(port: Port, refresh_cache: bool) -> Result<Self> {
        let registry = get_full_registry(port, refresh_cache).await?;
        Ok(Self { registry })
    }

    /// Generate response for requested types using the V2 approach
    fn generate_response(&self, requested_types: &[String]) -> TypeSchemaResponse {
        let mut response = TypeSchemaResponse {
            discovered_count: 0,
            requested_types:  requested_types.to_vec(),
            summary:          TypeSchemaSummary {
                failed_discoveries:     0,
                successful_discoveries: 0,
                total_requested:        requested_types.len(),
            },
            type_info:        HashMap::new(),
        };

        for brp_type_name in requested_types.iter().map(BrpTypeName::from) {
            let type_info = if let Some(type_schema) = self.registry.get(&brp_type_name) {
                response.discovered_count += 1;
                response.summary.successful_discoveries += 1;
                TypeInfo::from_schema(brp_type_name.clone(), type_schema, &self.registry)
            } else {
                response.summary.failed_discoveries += 1;
                TypeInfo::not_found(
                    brp_type_name.clone(),
                    "Type not found in registry".to_string(),
                )
            };

            response.type_info.insert(brp_type_name, type_info);
        }

        response
    }
}

/// The main tool struct for type schema discovery
pub struct TypeSchema;

impl ToolFn for TypeSchema {
    type Output = TypeSchemaResult;
    type Params = TypeSchemaParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: TypeSchemaParams = ctx.extract_parameter_values()?;
            let result = handle_impl(params.clone()).await;
            Ok(ToolResult {
                result,
                params: Some(params),
            })
        })
    }
}

/// Thin orchestration function: build engine and delegate the work to it.
async fn handle_impl(params: TypeSchemaParams) -> Result<TypeSchemaResult> {
    // Construct V2 engine with optional cache refresh
    let engine = TypeSchemaEngine::new(params.port, params.refresh_cache).await?;

    // Run the engine to produce the typed response
    let response = engine.generate_response(&params.types);
    let type_count = response.discovered_count;

    Ok(TypeSchemaResult::new(response, type_count)
        .with_message_template(format!("Discovered {type_count} type(s)")))
}
