//! `brp_all_type_guides` tool - Get type guides for all registered types
//!
//! This tool fetches all registered component and resource types from the Bevy app and returns
//! their type schema information in a single call. It combines `world.list_components`,
//! `world.list_resources`, and `brp_type_guide` functionality for convenience.

use bevy_brp_mcp_macros::{ParamStruct, ToolFn};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tool_type_guide::{TypeGuideEngine, TypeGuideResult};
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
    // Fetch component types
    let component_types = fetch_type_list(
        BrpMethod::WorldListComponents,
        params.port,
        "world.list_components",
    )
    .await?;

    // Fetch resource types
    let resource_types = fetch_type_list(
        BrpMethod::WorldListResources,
        params.port,
        "world.list_resources",
    )
    .await?;

    // Merge both lists
    let mut all_types = component_types;
    all_types.extend(resource_types);

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

/// Helper function to fetch a list of type names from a BRP method
async fn fetch_type_list(method: BrpMethod, port: Port, method_name: &str) -> Result<Vec<String>> {
    let client = BrpClient::new(method, port, None);

    match client.execute_direct_internal_no_enhancement().await {
        Ok(ResponseStatus::Success(Some(types_data))) => types_data.as_array().map_or_else(
            || {
                Err(Error::BrpCommunication(format!(
                    "{method_name} did not return an array of types"
                ))
                .into())
            },
            |types_array| {
                Ok(types_array
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect())
            },
        ),
        Ok(ResponseStatus::Success(None)) => {
            Err(Error::BrpCommunication(format!("{method_name} returned no data")).into())
        }
        Ok(ResponseStatus::Error(err)) => Err(Error::BrpCommunication(format!(
            "{method_name} failed: {}",
            err.get_message()
        ))
        .into()),
        Err(e) => Err(e),
    }
}
