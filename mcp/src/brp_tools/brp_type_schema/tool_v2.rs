//! V2 tool implementation for type schema discovery
//!
//! This module provides a temporary V2 tool that uses only the new engine
//! for testing and comparison purposes. This will be removed once the V2
//! implementation is validated and replaces the original tool.

use bevy_brp_mcp_macros::ResultStruct;

use super::TypeSchemaParams;
use super::engine_v2::TypeSchemaEngineV2;
use super::result_types::TypeSchemaResponseV2;
use crate::error::Result;
use crate::tool::{HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Temporary V2 tool for type schema discovery using the new engine
pub struct TypeSchemaV2;

impl ToolFn for TypeSchemaV2 {
    type Output = TypeSchemaResultV2;
    type Params = TypeSchemaParams;

    fn call(&self, ctx: HandlerContext) -> HandlerResult<ToolResult<Self::Output, Self::Params>> {
        Box::pin(async move {
            let params: TypeSchemaParams = ctx.extract_parameter_values()?;
            let result = handle_v2_impl(params.clone()).await;
            Ok(ToolResult {
                result,
                params: Some(params),
            })
        })
    }
}

/// V2 result type that wraps the V2 response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ResultStruct)]
pub struct TypeSchemaResultV2 {
    /// The V2 response data
    #[to_result]
    response:         TypeSchemaResponseV2,
    /// Type count for message template
    #[to_metadata]
    type_count:       usize,
    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

impl TypeSchemaResultV2 {
    /// Create a new V2 result from response with message
    pub const fn new_with_message(
        response: TypeSchemaResponseV2,
        type_count: usize,
        message: String,
    ) -> Self {
        Self {
            response,
            type_count,
            message_template: Some(message),
        }
    }
}

/// V2 implementation handler that uses only the new engine
async fn handle_v2_impl(params: TypeSchemaParams) -> Result<TypeSchemaResultV2> {
    // Use ONLY the new V2 engine
    let engine = TypeSchemaEngineV2::new(params.port).await?;
    let response = engine.generate_response(&params.types);
    let type_count = response.discovered_count;

    Ok(TypeSchemaResultV2::new_with_message(
        response,
        type_count,
        format!("V2: Discovered {type_count} type(s)"),
    ))
}
