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

use serde_json::Value;
use tracing::debug;

use super::discovery_context::DiscoveryContext;
use super::recovery_engine::{self, LevelResult};
use super::recovery_result::FormatRecoveryResult;
use super::types::CorrectionResult;
use super::unified_types::{DiscoverySource, UnifiedTypeInfo};
use crate::brp_tools::{BrpClientError, Port, ResponseStatus};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, JsonFieldAccess, ParameterName};

/// Engine for format discovery and correction
///
/// Encapsulates the multi-tiered format discovery system that intelligently
/// corrects type serialization errors in BRP operations.
pub struct FormatDiscoveryEngine {
    method:         BrpMethod,
    port:           Port,
    params:         Value,
    original_error: BrpClientError,
    type_names:     Vec<String>,
}

impl FormatDiscoveryEngine {
    /// Create a new format discovery engine for a specific method and port
    ///
    /// Returns an error if the parameters are invalid for format discovery
    /// (e.g., None when format discovery requires parameters, or error is not a format error)
    pub fn new(
        method: BrpMethod,
        port: Port,
        params: Option<Value>,
        original_error: BrpClientError,
    ) -> Result<Self> {
        // Check if we can recover from this error type
        if !original_error.is_format_error() {
            return Err(Error::InvalidArgument(
                "Format discovery can only be used with format errors".to_string(),
            )
            .into());
        }

        // Validate that parameters exist for format discovery
        // they actually have to or they wouldn't be methods called
        // `ExecuteMode::WithFormatDiscovery` however we want to take them out of the Option
        // here so we can stop Option wrangling. it is an Option until now because Other tools
        // don't require parameters.
        let params = params.ok_or_else(|| {
            Error::InvalidArgument(
                "Format discovery requires parameters to extract type information".to_string(),
            )
        })?;

        // Extract type names once for reuse
        let type_names = extract_type_names_from_params(method, &params);

        // Early exit if no types to process
        if type_names.is_empty() {
            return Err(Error::InvalidArgument(
                "No type names found in parameters, cannot perform format discovery".to_string(),
            )
            .into());
        }

        Ok(Self {
            method,
            port,
            params,
            original_error,
            type_names,
        })
    }

    /// Entry point for the work of format discovery
    pub async fn attempt_discovery_with_recovery(&self) -> Result<FormatRecoveryResult> {
        // Create discovery context
        let discovery_context =
            DiscoveryContext::fetch_from_registry(self.port, self.type_names.clone()).await?;

        let registry_type_info = discovery_context.as_hashmap();

        debug!(
            "FormatDiscoveryEngine: Starting multi-level discovery for method '{}' with {} pre-fetched type info(s)",
            self.method,
            registry_type_info.len()
        );

        debug!(
            "FormatDiscoveryEngine: Found {} type names to process",
            self.type_names.len()
        );

        // Level 1: Check for serialization issues
        if let Some(corrections) = self.detect_serialization_issues(registry_type_info) {
            debug!("FormatDiscoveryEngine: Level 1 detected serialization issue");
            return Ok(self.build_recovery_result(corrections).await);
        }

        // Level 2: Direct Discovery via bevy_brp_extras
        debug!("FormatDiscoveryEngine: Beginning Level 2 - Direct discovery");
        let level_2_type_infos = match self
            .execute_level_2_direct_discovery(registry_type_info)
            .await
        {
            LevelResult::Success(corrections) => {
                debug!("FormatDiscoveryEngine: Level 2 succeeded with direct discovery");
                return Ok(self.build_recovery_result(corrections).await);
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

        match self.execute_level_3_pattern_transformations(&level_2_type_infos) {
            LevelResult::Success(corrections) => {
                debug!("FormatDiscoveryEngine: Level 3 succeeded with pattern-based corrections");
                Ok(self.build_recovery_result(corrections).await)
            }
            LevelResult::Continue(_) => {
                debug!("FormatDiscoveryEngine: All levels exhausted, no recovery possible");
                Ok(FormatRecoveryResult::NotRecoverable {
                    corrections: Vec::new(),
                })
            }
        }
    }

    /// Level 2: Direct discovery via `bevy_brp_extras/discover_format`
    async fn execute_level_2_direct_discovery(
        &self,
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    ) -> LevelResult {
        // TODO: Move implementation from recovery_engine
        recovery_engine::execute_level_2_direct_discovery(
            &self.type_names,
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
        level_2_type_infos: &HashMap<String, UnifiedTypeInfo>,
    ) -> LevelResult {
        // TODO: Move implementation from recovery_engine
        recovery_engine::execute_level_3_pattern_transformations(
            &self.type_names,
            self.method,
            &self.params,
            &self.original_error,
            level_2_type_infos,
        )
    }

    /// Build a recovery result from corrections
    async fn build_recovery_result(
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

    /// Detect serialization issues and return corrections explaining the problems
    ///
    /// Returns `Some(corrections)` if serialization issues are found,
    /// `None` if no issues are detected.
    fn detect_serialization_issues(
        &self,
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    ) -> Option<Vec<CorrectionResult>> {
        // Only check for spawn/insert methods with UnknownComponentType errors
        if !matches!(self.method, BrpMethod::BevySpawn | BrpMethod::BevyInsert) {
            return None;
        }

        // we allow continuing through the serialization detection if the error is
        // Unknown component type due to a bug in bevy where it will throw an error with this
        // when a component does have serialization support that is required for the mutation
        if !self
            .original_error
            .message
            .contains("Unknown component type")
        {
            return None;
        }

        debug!("Checking for serialization errors in registry type infos");

        // Check each type for serialization support
        for type_name in &self.type_names {
            if let Some(type_info) = registry_type_info.get(type_name) {
                debug!(
                    "Component '{}' found in registry, brp_compatible={}",
                    type_name, type_info.serialization.brp_compatible
                );

                // Component is registered but lacks serialization - short circuit
                if type_info.registry_status.in_registry && !type_info.serialization.brp_compatible
                {
                    debug!(
                        "Component '{}' lacks serialization, building corrections",
                        type_name
                    );
                    let educational_message = format!(
                        "Component '{}' is registered but lacks Serialize and Deserialize traits required for {} operations. \
                        Add #[derive(Serialize, Deserialize)] to the component definition.",
                        type_name,
                        self.method.as_str()
                    );

                    let corrections = self
                        .type_names
                        .iter()
                        .map(|type_name| {
                            let type_info = registry_type_info
                                .get(type_name)
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
                    return Some(corrections);
                }
            }
        }

        debug!("All components have serialization support or are not in registry");
        None
    }
}

/// Extract type names from BRP method parameters based on method type
fn extract_type_names_from_params(method: BrpMethod, params: &Value) -> Vec<String> {
    let mut type_names = Vec::new();

    match method {
        BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
            // Types are keys in the "components" object
            if let Some(components) = ParameterName::Components.get_object_from(params) {
                for type_name in components.keys() {
                    type_names.push(type_name.clone());
                }
            }
        }
        BrpMethod::BevyMutateComponent => {
            // Single type in "component" field
            if let Some(component) = params
                .get(ParameterName::Component.as_ref())
                .and_then(|c| c.as_str())
            {
                type_names.push(component.to_string());
            }
        }
        BrpMethod::BevyInsertResource | BrpMethod::BevyMutateResource => {
            // Single type in "resource" field
            if let Some(resource) = ParameterName::Resource.get_str_from(params) {
                type_names.push(resource.to_string());
            }
        }
        _ => {
            // For other methods, we don't currently support type extraction
        }
    }

    type_names
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::brp_tools::brp_client::format_discovery::unified_types::DiscoverySource;

    fn create_test_engine(method: BrpMethod, error_message: &str) -> FormatDiscoveryEngine {
        let params = Some(serde_json::json!({
            "components": {
                "bevy_render::view::visibility::Visibility": "Hidden"
            }
        }));

        FormatDiscoveryEngine::new(
            method,
            Port(15702),
            params,
            BrpClientError {
                code:    -23402,
                message: error_message.to_string(),
                data:    None,
            },
        )
        .expect("Test engine creation should succeed")
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
    fn test_detect_serialization_issues_missing_traits() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        // Create registry info for the type that the engine extracted from its parameters
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        let corrections = result.unwrap();
        assert!(!corrections.is_empty());
        if let CorrectionResult::CannotCorrect { reason, .. } = &corrections[0] {
            assert!(reason.contains("lacks Serialize and Deserialize traits"));
            assert!(reason.contains("bevy_render::view::visibility::Visibility"));
            assert!(reason.contains("Add #[derive(Serialize, Deserialize)]"));
        } else {
            panic!("Expected CannotCorrect result");
        }
    }

    #[test]
    fn test_detect_serialization_issues_with_traits() {
        // Create engine with Transform component parameters
        let params = Some(serde_json::json!({
            "components": {
                "bevy_transform::components::transform::Transform": {}
            }
        }));

        let engine = FormatDiscoveryEngine::new(
            BrpMethod::BevySpawn,
            Port(15702),
            params,
            BrpClientError {
                code:    -23402,
                message: "Unknown component type: `bevy_reflect::DynamicEnum`".to_string(),
                data:    None,
            },
        )
        .expect("Test engine creation should succeed");

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_transform::components::transform::Transform".to_string(),
            create_type_info_with_serialization("bevy_transform::components::transform::Transform"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because type has serialization support
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_serialization_issues_non_spawn_method() {
        // Test with MutateComponent which extracts type names but is not spawn/insert
        let params = Some(serde_json::json!({
            "component": "bevy_render::view::visibility::Visibility"
        }));

        let engine = FormatDiscoveryEngine::new(
            BrpMethod::BevyMutateComponent, // Not spawn/insert
            Port(15702),
            params,
            BrpClientError {
                code:    -23402,
                message: "Unknown component type: `bevy_reflect::DynamicEnum`".to_string(),
                data:    None,
            },
        )
        .expect("Test engine creation should succeed");

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because it's not a spawn/insert method
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_serialization_issues_different_error() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Some other error message", // Not UnknownComponentType
        );

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because error message doesn't match
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_serialization_issues_type_not_in_registry() {
        // Create engine with a component that won't be in the registry
        let params = Some(serde_json::json!({
            "components": {
                "my_game::MyComponent": {}
            }
        }));

        let engine = FormatDiscoveryEngine::new(
            BrpMethod::BevySpawn,
            Port(15702),
            params,
            BrpClientError {
                code:    -23402,
                message: "Unknown component type: `bevy_reflect::DynamicEnum`".to_string(),
                data:    None,
            },
        )
        .expect("Test engine creation should succeed");

        let registry_info = HashMap::new(); // Empty registry

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because type is not in registry
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_serialization_issues_insert_method() {
        let engine = create_test_engine(
            BrpMethod::BevyInsert, // Also should work for insert
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        );

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        let corrections = result.unwrap();
        assert!(!corrections.is_empty());
        if let CorrectionResult::CannotCorrect { reason, .. } = &corrections[0] {
            assert!(reason.contains("lacks Serialize and Deserialize traits"));
            assert!(reason.contains("insert operations")); // Should say "insert" not "spawn"
        } else {
            panic!("Expected CannotCorrect result");
        }
    }
}
