//! Orchestration and retry logic for format discovery
//!
//! # Format Discovery Tier Flow
//!
//! The format discovery engine uses a tiered approach to correct format errors:
//!
//! ## Tier 1: Serialization Diagnostics
//! Checks if the type supports BRP serialization at all.
//! ```text
//! Input: {"MyComponent": {"value": 42}}
//! Check: Does MyComponent have Serialize/Deserialize traits?
//! Result: If no, return educational message about trait requirements
//! ```
//!
//! ## Tier 2: Direct Discovery (requires `bevy_brp_extras`)
//! Queries the Bevy app for the correct format directly.
//! ```text
//! Input: {"Transform": {"translation": {"x": 1, "y": 2, "z": 3}}}
//! Query: bevy_brp_extras/discover_format for Transform
//! Result: Rich response with:
//!   - Corrected format: {"translation": [1.0, 2.0, 3.0], ...}
//!   - supported_operations: ["spawn", "insert", "mutate"]
//!   - mutation_paths: [".translation.x", ".translation.y", ...]
//!   - type_category: "Component"
//! ```
//!
//! ## Tier 3: Pattern-Based Transformation
//! Uses error patterns to apply deterministic transformations.
//! ```text
//! Input: {"Vec3": {"x": 1, "y": 2, "z": 3}}
//! Error: "AccessError: expected array"
//! Transform: Convert object to array [1.0, 2.0, 3.0]
//! Result: Corrected format with pattern-based hint
//! ```
//!
//! ## Tier 4: Educational Response (Generic Fallback)
//! When format cannot be corrected, provides educational guidance.
//! ```text
//! Input: [1, 2, 3] for unknown type
//! Result: Educational message explaining:
//!   - Why the format is ambiguous
//!   - How to use format discovery tools
//!   - Available metadata if discovered
//! ```

use serde_json::Value;
use tracing::debug;

use super::constants::FORMAT_DISCOVERY_METHODS;
use crate::brp_tools::support::brp_client::BrpResult;
use crate::error::Result;

/// Location of type items in method parameters
#[derive(Debug, Clone, Copy)]
pub enum ParameterLocation {
    /// Type items are in a "components" object (spawn, insert)
    Components,
    /// Single type value in "value" field (`mutate_component`)
    ComponentValue,
    /// Single type value in "value" field (`insert_resource`, `mutate_resource`)
    ResourceValue,
}

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone)]
pub struct FormatCorrection {
    pub component:            String, // Keep field name for API compatibility
    pub original_format:      Value,
    pub corrected_format:     Value,
    pub hint:                 String,
    pub supported_operations: Option<Vec<String>>,
    pub mutation_paths:       Option<Vec<String>>,
    pub type_category:        Option<String>,
}

impl FormatCorrection {
    /// Helper method to check if rich metadata is available
    /// This ensures the fields are recognized as used by the compiler
    pub const fn has_rich_metadata(&self) -> bool {
        self.supported_operations.is_some()
            || self.mutation_paths.is_some()
            || self.type_category.is_some()
    }
}

/// Enhanced response with format corrections
#[derive(Debug, Clone)]
pub struct EnhancedBrpResult {
    pub result:             BrpResult,
    pub format_corrections: Vec<FormatCorrection>,
}

/// Execute a BRP method with automatic format discovery
pub async fn execute_brp_method_with_format_discovery(
    method: &str,
    params: Option<Value>,
    port: Option<u16>,
) -> Result<EnhancedBrpResult> {
    use crate::brp_tools::request_handler::format_discovery::phases::context::DiscoveryContext;
    use crate::brp_tools::request_handler::format_discovery::phases::{
        error_analysis, initial_attempt, result_building, tier_execution,
    };

    // Initialize the discovery context
    let mut context = DiscoveryContext::new(method, params, port);

    // Phase 1: Execute initial attempt
    let initial_result = initial_attempt::execute(&mut context).await?;

    // Phase 2: Check if error analysis indicates recovery is possible
    if let Some(error) = error_analysis::needs_format_discovery(&initial_result, method) {
        DiscoveryContext::add_debug(format!(
            "Format Discovery: Got error code {}, checking if method '{}' supports format discovery",
            error.code, method
        ));

        DiscoveryContext::add_debug(format!(
            "Format Discovery: Method '{method}' is in FORMAT_DISCOVERY_METHODS"
        ));

        DiscoveryContext::add_debug(
            "Format Discovery: Error is type format error, attempting discovery".to_string(),
        );

        // Phase 3: Run tiered discovery to find corrections
        let discovery_data = tier_execution::run_discovery_tiers(&context).await?;

        // Phase 4: Build final result with corrections
        return result_building::build_final_result(&context, discovery_data).await;
    }

    // Log appropriate message based on the result
    if let BrpResult::Error(ref error) = initial_result {
        if FORMAT_DISCOVERY_METHODS.contains(&method) {
            DiscoveryContext::add_debug(format!(
                "Format Discovery: Error is NOT a type format error (code: {})",
                error.code
            ));
        } else {
            DiscoveryContext::add_debug(format!(
                "Format Discovery: Method '{method}' is NOT in FORMAT_DISCOVERY_METHODS"
            ));
        }
    } else {
        DiscoveryContext::add_debug(
            "Format Discovery: Initial request succeeded, no discovery needed".to_string(),
        );
    }

    // Return original result if no format discovery needed/possible
    debug!("Format Discovery: Returning original result");

    Ok(EnhancedBrpResult {
        result:             initial_result,
        format_corrections: Vec::new(),
    })
}
