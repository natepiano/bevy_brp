//! `brp_type_guide` tool - Local registry-based type schema discovery
//!
//! This tool provides type schema information for types in your Bevy app without requiring
//! the `bevy_brp_extras` plugin. It uses registry schema calls combined with hardcoded BRP
//! serialization knowledge to provide accurate type schema information for BRP operations.

use std::collections::HashMap;
use std::sync::Arc;

use bevy_brp_mcp_macros::ParamStruct;
use bevy_brp_mcp_macros::ResultStruct;
use bevy_brp_mcp_macros::ToolFn;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use serde_json::json;

use super::brp_type_name::BrpTypeName;
use super::guide::TypeGuide;
use super::response_types::TypeGuideResponse;
use super::response_types::TypeGuideSummary;
use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::brp_tools::ResponseStatus;
use crate::error::Error;
use crate::error::Result;
use crate::tool::BrpMethod;
use crate::tool::HandlerContext;
use crate::tool::HandlerResult;
use crate::tool::ToolFn;
use crate::tool::ToolResult;

/// Parameters for the `brp_type_guide` tool
#[derive(Clone, Deserialize, Serialize, JsonSchema, ParamStruct)]
pub struct TypeGuideParams {
    /// Array of fully-qualified component type names to discover formats for
    pub types: Vec<String>,

    /// The BRP port (default: 15702)
    #[serde(default)]
    pub port: Port,
}

/// Result for the `brp_type_guide` tool
#[derive(Debug, Clone, Serialize, ResultStruct)]
pub struct TypeGuideResult {
    /// The type schema information containing format discovery results
    #[to_result]
    result: TypeGuideResponse,

    /// Count of types discovered
    #[to_metadata]
    type_count: usize,

    /// Message template for formatting responses
    #[to_message]
    message_template: Option<String>,
}

/// The main tool struct for type schema discovery
#[derive(ToolFn)]
#[tool_fn(params = "TypeGuideParams", output = "TypeGuideResult")]
pub struct BrpTypeGuide;

/// Thin orchestration function: build engine and delegate the work to it.
async fn handle_impl(params: TypeGuideParams) -> Result<TypeGuideResult> {
    // Construct V2 engine
    let engine = TypeGuideEngine::new(params.port).await?;

    // Run the engine to produce the typed response
    let response = engine.generate_response(&params.types);
    let type_count = response.discovered_count;

    Ok(TypeGuideResult::new(response, type_count)
        .with_message_template(format!("Discovered {type_count} type(s)")))
}

/// orchestrates type schema generation using a single call to get the complete registry
pub struct TypeGuideEngine {
    registry: Arc<HashMap<BrpTypeName, Value>>,
}

impl TypeGuideEngine {
    /// Create a new engine instance by fetching the complete registry
    pub async fn new(port: Port) -> Result<Self> {
        let registry = Arc::new(Self::get_full_registry(port).await?);
        Ok(Self { registry })
    }

    /// Get the complete registry
    ///
    /// Fetches fresh registry data from the BRP server on each call.
    async fn get_full_registry(port: Port) -> Result<HashMap<BrpTypeName, Value>> {
        // Fetch full registry from BRP
        let client = BrpClient::new(BrpMethod::RegistrySchema, port, Some(json!({})));

        match client.execute_direct_internal_no_enhancement().await {
            Ok(ResponseStatus::Success(Some(registry_data))) => {
                // Convert to HashMap with BrpTypeName keys
                let mut registry_map = HashMap::new();

                if let Some(obj) = registry_data.as_object() {
                    for (key, value) in obj {
                        let brp_type_name = BrpTypeName::from(key);
                        registry_map.insert(brp_type_name, value.clone());
                    }
                }

                Ok(registry_map)
            }
            Ok(_) => {
                Err(Error::BrpCommunication("Registry call returned no data".to_string()).into())
            }
            Err(e) => Err(e),
        }
    }

    /// Generate response for requested types
    pub fn generate_response(&self, requested_types: &[String]) -> TypeGuideResponse {
        // Build the type_guide HashMap functionally
        let type_guide: HashMap<BrpTypeName, TypeGuide> = requested_types
            .iter()
            .map(BrpTypeName::from)
            .map(|brp_type_name| {
                let type_info = TypeGuide::build(brp_type_name.clone(), Arc::clone(&self.registry))
                    .unwrap_or_else(|e| {
                        // Processing failed - type was found but building failed
                        TypeGuide::processing_failed(
                            brp_type_name.clone(),
                            format!("Failed to process type: {e}"),
                        )
                    });
                (brp_type_name, type_info)
            })
            .collect();

        // Calculate summary statistics from the results
        let successful_discoveries = type_guide
            .values()
            .filter(|tg| tg.in_registry && tg.error.is_none())
            .count();
        let failed_discoveries = type_guide
            .values()
            .filter(|tg| !tg.in_registry || tg.error.is_some())
            .count();

        TypeGuideResponse {
            discovered_count: successful_discoveries,
            requested_types: requested_types.to_vec(),
            summary: TypeGuideSummary {
                failed_discoveries,
                successful_discoveries,
                total_requested: requested_types.len(),
            },
            type_guide,
        }
    }
}
