//! `brp_all_type_guides` tool - Get type guides for all registered types
//!
//! This tool fetches all registered component types from the Bevy app and returns
//! their type schema information in a single call. It combines `bevy/list` and
//! `brp_type_guide` functionality for convenience.

use bevy_brp_mcp_macros::{ParamStruct, ToolFn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tool::{TypeGuideEngine, TypeGuideResult};
use crate::brp_tools::{BrpClient, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, HandlerContext, HandlerResult, ToolFn, ToolResult};

/// Parameters for the `brp_all_type_guides` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct AllTypeGuidesParams {
    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// The main tool struct for getting all type guides
#[derive(ToolFn)]
#[tool_fn(params = "AllTypeGuidesParams", output = "TypeGuideResult")]
pub struct BrpAllTypeGuides;

/// Implementation that fetches all types then gets their guides
async fn handle_impl(params: AllTypeGuidesParams) -> Result<TypeGuideResult> {
    // First, get all registered types using bevy/list without entity parameter
    let list_client = BrpClient::new(
        BrpMethod::BevyList,
        params.port,
        None, // No params means get all types
    );

    let all_types = match list_client.execute_direct_internal_no_enhancement().await {
        Ok(ResponseStatus::Success(Some(types_data))) => {
            // Extract the array of type names
            if let Some(types_array) = types_data.as_array() {
                types_array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            } else {
                return Err(Error::BrpCommunication(
                    "bevy/list did not return an array of types".to_string(),
                )
                .into());
            }
        }
        Ok(ResponseStatus::Success(None)) => {
            return Err(Error::BrpCommunication("bevy/list returned no data".to_string()).into());
        }
        Ok(ResponseStatus::Error(err)) => {
            return Err(Error::BrpCommunication(format!(
                "bevy/list failed: {}",
                err.get_message()
            ))
            .into());
        }
        Err(e) => return Err(e),
    };

    // Construct TypeSchemaEngine and generate response for all types
    let engine = TypeGuideEngine::new(params.port).await?;
    let response = engine.generate_response(&all_types);
    let type_count = response.discovered_count;

    Ok(
        TypeGuideResult::new(response, type_count).with_message_template(format!(
            "Discovered schemas for all {type_count} registered type(s)"
        )),
    )
}
