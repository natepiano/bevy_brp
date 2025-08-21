//! `brp_type_schema` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide the same information as `brp_extras_discover_format`.
//!
//! Refactored to introduce a small `TypeSchemaEngine` that owns request-scoped state
//! (port + `registry_data`) and implements the workflow. The `TypeSchema` tool is kept thin
//! and simply delegates to the engine.

use bevy_brp_mcp_macros::{ParamStruct, ResultStruct};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::engine_v2::TypeSchemaEngine;
use super::result_types::TypeSchemaResponse;
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
    debug!("Executing brp_type_schema for {} types", params.types.len());

    // Construct V2 engine
    let engine = TypeSchemaEngine::new(params.port).await?;

    // Run the engine to produce the typed response (V2 is synchronous)
    let response = engine.generate_response(&params.types);
    let type_count = response.discovered_count;

    Ok(TypeSchemaResult::new(response, type_count)
        .with_message_template(format!("Discovered {type_count} type(s)")))
}

/* Engine implementation moved to `engine.rs`. The tool delegates to the engine via
`super::TypeSchemaEngine` (re-exported from the module). */

// BrpTypeSchema is exported as an alias for backward compatibility
pub use TypeSchema as BrpTypeSchema;
