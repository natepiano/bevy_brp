//! Orchestration and flow control for format discovery
//!
//! # Architecture Overview
//!
//! The format discovery engine implements a clean two-phase architecture:
//!
//! ## Level 1: Normal Path (Direct BRP Execution)
//! Most requests succeed without any format discovery overhead.
//! ```text
//! Request: bevy/spawn with correct format
//! Result: Direct success, no discovery needed
//! ```
//!
//! ## Exception Path: Format Error Recovery
//! When Level 1 Normal Path fails with format errors, enter the exception path with a 3-level
//! decision tree:
//!
//! ### Level 1: Registry/Serialization Checks
//! Verify type registration and serialization support.
//! ```text
//! Check: Is type in registry? Does it have Serialize/Deserialize?
//! Result: Educational guidance for unsupported types
//! ```
//!
//! ### Level 2: Direct Discovery (requires `bevy_brp_extras`)
//! Query the Bevy app for authoritative format information.
//! ```text
//! Query: `bevy_brp_extras`/discover_format for type
//! Result: Corrected format with rich metadata
//! ```
//!
//! ### Level 3: Pattern-Based Transformations
//! Apply deterministic transformations based on error patterns.
//! ```text
//! Pattern: Vec3 object→array conversion, enum variant access
//! Result: Corrected format with transformation hints
//! ```
//! Succinct call flow notes:
//! The format discovery engine makes the initial attempt at the BRP call. This MUST use
//! `execute_direct()` to avoid infinite recursion since it's part of the format discovery flow
//! itself.
//! - `format_discovery/engine.rs` - Makes the initial BRP call attempt
//! - `format_discovery/registry_integration.rs` - Queries registry for type info
//! - `format_discovery/extras_integration.rs` - Calls discovery endpoint
//! - `format_discovery/recovery_engine.rs` - Retries with corrected params

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use super::flow_types::{CorrectionResult, FormatRecoveryResult};
use super::recovery_engine::{self, LevelResult};
use super::type_discovery_context::TypeDiscoveryContext;
use super::unified_types::{DiscoverySource, UnifiedTypeInfo};
use crate::brp_tools::{BrpClientError, Port, ResponseStatus};
use crate::error::Result;
use crate::tool::BrpMethod;

/// Engine for format discovery and correction
///
/// Encapsulates the multi-tiered format discovery system that intelligently
/// corrects type serialization errors in BRP operations.
pub struct FormatDiscoveryEngine {
    method:         BrpMethod,
    port:           Port,
    params:         Value,
    original_error: BrpClientError,
}

impl FormatDiscoveryEngine {
    /// Create a new format discovery engine for a specific method and port
    pub const fn new(
        method: BrpMethod,
        port: Port,
        params: Value,
        original_error: BrpClientError,
    ) -> Self {
        Self {
            method,
            port,
            params,
            original_error,
        }
    }

    pub async fn attempt_discovery_with_recovery(&self) -> Result<FormatRecoveryResult> {
        // Skip Level 1 - we already failed
        // Check if error is format-related
        if !self.original_error.is_format_error() {
            return Ok(FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            });
        }

        // Extract type names once for reuse
        let type_names = recovery_engine::extract_type_names_from_params(self.method, &self.params);

        // Early exit if no types to process
        if type_names.is_empty() {
            debug!("FormatDiscoveryEngine: No type names found in parameters, cannot recover");
            return Ok(FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            });
        }

        // Create discovery context
        let type_context =
            TypeDiscoveryContext::fetch_from_registry(self.port, type_names.clone()).await?;

        // Execute the discovery process
        let flow_result = self
            .attempt_format_discovery_with_type_infos(type_context.as_hashmap(), type_names)
            .await;

        Ok(flow_result)
    }

    /// Execute format discovery using the 3-level decision tree
    async fn attempt_format_discovery_with_type_infos(
        &self,
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
        type_names: Vec<String>,
    ) -> FormatRecoveryResult {
        debug!(
            "FormatDiscoveryEngine: Starting multi-level discovery for method '{}' with {} pre-fetched type info(s)",
            self.method,
            registry_type_info.len()
        );

        debug!(
            "FormatDiscoveryEngine: Found {} type names to process",
            type_names.len()
        );

        // Level 1 (added back): Check for serialization issues
        if let Some(educational_message) =
            self.check_serialization_support(&type_names, &registry_type_info)
        {
            debug!("FormatDiscoveryEngine: Level 1 detected serialization issue");
            let corrections = type_names
                .into_iter()
                .map(|type_name| {
                    let type_info =
                        registry_type_info
                            .get(&type_name)
                            .cloned()
                            .unwrap_or_else(|| {
                                UnifiedTypeInfo::new(
                                    type_name.clone(),
                                    DiscoverySource::TypeRegistry,
                                )
                            });
                    CorrectionResult::CannotCorrect {
                        type_info,
                        reason: educational_message.clone(),
                    }
                })
                .collect();
            return self.build_recovery_success(corrections).await;
        }

        // Level 2: Direct Discovery via bevy_brp_extras
        debug!("FormatDiscoveryEngine: Beginning Level 2 - Direct discovery");
        let level_2_type_infos = match self
            .execute_level_2_direct_discovery(&type_names, &registry_type_info)
            .await
        {
            LevelResult::Success(corrections) => {
                debug!("FormatDiscoveryEngine: Level 2 succeeded with direct discovery");
                return self.build_recovery_success(corrections).await;
            }
            LevelResult::Continue(type_infos) => {
                debug!(
                    "FormatDiscoveryEngine: Level 2 complete, proceeding to Level 3 with {} type infos",
                    type_infos.len()
                );
                type_infos
            }
        };

        // Level 3: Pattern-Based Transformations
        debug!("FormatDiscoveryEngine: Level 3 - Pattern-based transformations");

        match self.execute_level_3_pattern_transformations(&type_names, &level_2_type_infos) {
            LevelResult::Success(corrections) => {
                debug!("FormatDiscoveryEngine: Level 3 succeeded with pattern-based corrections");
                self.build_recovery_success(corrections).await
            }
            LevelResult::Continue(_) => {
                debug!("FormatDiscoveryEngine: All levels exhausted, no recovery possible");
                FormatRecoveryResult::NotRecoverable {
                    corrections: Vec::new(),
                }
            }
        }
    }

    /// Level 2: Direct discovery via `bevy_brp_extras/discover_format`
    async fn execute_level_2_direct_discovery(
        &self,
        type_names: &[String],
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    ) -> LevelResult {
        // TODO: Move implementation from recovery_engine
        recovery_engine::execute_level_2_direct_discovery(
            type_names,
            self.method,
            registry_type_info,
            &self.params,
            self.port,
        )
        .await
    }

    /// Level 3: Pattern-based transformations
    fn execute_level_3_pattern_transformations(
        &self,
        type_names: &[String],
        level_2_type_infos: &HashMap<String, UnifiedTypeInfo>,
    ) -> LevelResult {
        // TODO: Move implementation from recovery_engine
        recovery_engine::execute_level_3_pattern_transformations(
            type_names,
            self.method,
            &self.params,
            &self.original_error,
            level_2_type_infos,
        )
    }

    /// Build a successful recovery result
    async fn build_recovery_success(
        &self,
        corrections: Vec<CorrectionResult>,
    ) -> FormatRecoveryResult {
        // TODO: Move implementation from recovery_engine
        recovery_engine::build_recovery_success(
            corrections,
            self.method,
            &self.params,
            &ResponseStatus::Error(self.original_error.clone()),
            self.port,
        )
        .await
    }

    /// Check if any types lack serialization support
    fn check_serialization_support(
        &self,
        type_names: &[String],
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    ) -> Option<String> {
        // Only check for spawn/insert methods with UnknownComponentType errors
        if !matches!(self.method, BrpMethod::BevySpawn | BrpMethod::BevyInsert) {
            return None;
        }

        if !self
            .original_error
            .message
            .contains("Unknown component type")
        {
            return None;
        }

        debug!("Checking for serialization errors in registry type infos");

        // Check each type for serialization support
        for type_name in type_names {
            if let Some(type_info) = registry_type_info.get(type_name) {
                debug!(
                    "Component '{}' found in registry, brp_compatible={}",
                    type_name, type_info.serialization.brp_compatible
                );

                // Component is registered but lacks serialization - short circuit
                if type_info.registry_status.in_registry && !type_info.serialization.brp_compatible
                {
                    debug!(
                        "Component '{}' lacks serialization, returning educational message",
                        type_name
                    );
                    return Some(format!(
                        "Component '{}' is registered but lacks Serialize and Deserialize traits required for {} operations. \
                        Add #[derive(Serialize, Deserialize)] to the component definition.",
                        type_name,
                        self.method.as_str()
                    ));
                }
            }
        }

        debug!("All components have serialization support or are not in registry");
        None
    }
}

/// Format correction information for a type (component or resource)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatCorrection {
    pub component:            String, // Keep field name for API compatibility
    pub original_format:      Value,
    pub corrected_format:     Value,
    pub hint:                 String,
    pub supported_operations: Option<Vec<String>>,
    pub mutation_paths:       Option<Vec<String>>,
    pub type_category:        Option<String>,
}

impl FormatCorrection {}

/// Status of format correction attempts
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatCorrectionStatus {
    /// Format discovery was not enabled for this request
    NotApplicable,
    /// No format correction was attempted
    NotAttempted,
    /// Format correction was applied and the operation succeeded
    Succeeded,
    /// Format correction was attempted but the operation still failed
    AttemptedButFailed,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::brp_tools::brp_client::format_discovery::unified_types::DiscoverySource;

    fn create_test_engine(method: BrpMethod, error_message: &str) -> FormatDiscoveryEngine {
        FormatDiscoveryEngine {
            method,
            port: Port(15702),
            params: serde_json::json!({
                "components": {
                    "bevy_render::view::visibility::Visibility": "Hidden"
                }
            }),
            original_error: BrpClientError {
                code:    -23402,
                message: error_message.to_string(),
                data:    None,
            },
        }
    }

    fn create_type_info_without_serialization(type_name: &str) -> UnifiedTypeInfo {
        let mut info = UnifiedTypeInfo::new(type_name.to_string(), DiscoverySource::TypeRegistry);
        info.registry_status.in_registry = true;
        info.serialization.has_serialize = false;
        info.serialization.has_deserialize = false;
        info.serialization.brp_compatible = false;
        info
    }

    fn create_type_info_with_serialization(type_name: &str) -> UnifiedTypeInfo {
        let mut info = UnifiedTypeInfo::new(type_name.to_string(), DiscoverySource::TypeRegistry);
        info.registry_status.in_registry = true;
        info.serialization.has_serialize = true;
        info.serialization.has_deserialize = true;
        info.serialization.brp_compatible = true;
        info
    }

    #[test]
    fn test_check_serialization_support_missing_traits() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let type_names = vec!["bevy_render::view::visibility::Visibility".to_string()];
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.check_serialization_support(&type_names, &registry_info);

        let message = result.unwrap();
        assert!(message.contains("lacks Serialize and Deserialize traits"));
        assert!(message.contains("bevy_render::view::visibility::Visibility"));
        assert!(message.contains("Add #[derive(Serialize, Deserialize)]"));
    }

    #[test]
    fn test_check_serialization_support_with_traits() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let type_names = vec!["bevy_transform::components::transform::Transform".to_string()];
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_transform::components::transform::Transform".to_string(),
            create_type_info_with_serialization("bevy_transform::components::transform::Transform"),
        );

        let result = engine.check_serialization_support(&type_names, &registry_info);

        // Should return None because type has serialization support
        assert!(result.is_none());
    }

    #[test]
    fn test_check_serialization_support_non_spawn_method() {
        let engine = create_test_engine(
            BrpMethod::BevyQuery, // Not spawn/insert
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let type_names = vec!["bevy_render::view::visibility::Visibility".to_string()];
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.check_serialization_support(&type_names, &registry_info);

        // Should return None because it's not a spawn/insert method
        assert!(result.is_none());
    }

    #[test]
    fn test_check_serialization_support_different_error() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Some other error message", // Not UnknownComponentType
        );

        let type_names = vec!["bevy_render::view::visibility::Visibility".to_string()];
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.check_serialization_support(&type_names, &registry_info);

        // Should return None because error message doesn't match
        assert!(result.is_none());
    }

    #[test]
    fn test_check_serialization_support_type_not_in_registry() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let type_names = vec!["my_game::MyComponent".to_string()];
        let registry_info = HashMap::new(); // Empty registry

        let result = engine.check_serialization_support(&type_names, &registry_info);

        // Should return None because type is not in registry
        assert!(result.is_none());
    }

    #[test]
    fn test_check_serialization_support_insert_method() {
        let engine = create_test_engine(
            BrpMethod::BevyInsert, // Also should work for insert
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let type_names = vec!["bevy_render::view::visibility::Visibility".to_string()];
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.check_serialization_support(&type_names, &registry_info);

        let message = result.unwrap();
        assert!(message.contains("lacks Serialize and Deserialize traits"));
        assert!(message.contains("insert operations")); // Should say "insert" not "spawn"
    }
}
