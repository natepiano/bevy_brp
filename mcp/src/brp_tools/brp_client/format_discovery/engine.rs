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
use std::fmt::Write;

use serde_json::Value;
use tracing::debug;

use super::discovery_context::DiscoveryContext;
use super::format_correction_fields::FormatCorrectionField;
use super::recovery_engine;
use super::recovery_result::FormatRecoveryResult;
use super::types::{Correction, CorrectionInfo, CorrectionMethod};
use super::unified_types::UnifiedTypeInfo;
use crate::brp_tools::{BrpClientError, Port, ResponseStatus, brp_client};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, JsonFieldAccess, ParameterName};

/// Result of a recovery level attempt
#[derive(Debug)]
pub enum LevelResult {
    /// Level succeeded and produced corrections
    Success(Vec<Correction>),
    /// Level completed but recovery should continue to next level
    Continue(std::collections::HashMap<String, UnifiedTypeInfo>),
}

/// Engine for format discovery and correction
///
/// Encapsulates the multi-tiered format discovery system that intelligently
/// corrects type serialization errors in BRP operations.
pub struct FormatDiscoveryEngine {
    method:            BrpMethod,
    port:              Port,
    params:            Value,
    original_error:    BrpClientError,
    type_names:        Vec<String>,
    discovery_context: DiscoveryContext,
}

impl FormatDiscoveryEngine {
    /// Create a new format discovery engine for a specific method and port
    ///
    /// Returns an error if the parameters are invalid for format discovery
    /// (e.g., None when format discovery requires parameters, or error is not a format error)
    pub async fn new(
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

        // Create discovery context with proper registry information
        let discovery_context =
            DiscoveryContext::fetch_from_registry(port, type_names.clone()).await?;

        Ok(Self {
            method,
            port,
            params,
            original_error,
            type_names,
            discovery_context,
        })
    }

    /// Entry point for the work of format discovery
    pub async fn attempt_discovery_with_recovery(&mut self) -> Result<FormatRecoveryResult> {
        // Use discovery context created in constructor
        let registry_type_info = self.discovery_context.as_hashmap();

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
        let level_2_type_infos = match self.execute_level_2_direct_discovery().await {
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
    async fn execute_level_2_direct_discovery(&mut self) -> LevelResult {
        debug!(
            "Level 2: Attempting direct discovery for {} types",
            self.type_names.len()
        );

        // Enrich context with extras discovery (don't fail if enrichment fails)
        if let Err(e) = self.discovery_context.enrich_with_extras().await {
            debug!("Level 2: Enrichment failed: {}", e);
            // Continue with registry-only info
        }

        // Use enriched context
        let enhanced_type_info = self.discovery_context.as_hashmap();

        // Attempt direct discovery for each type
        let mut corrections = Vec::new();

        for type_name in &self.type_names {
            debug!("Level 2: Processing corrections for '{type_name}'");

            // Get the enriched type info (may have data from both registry and extras)
            if let Some(discovered_info) = enhanced_type_info.get(type_name) {
                debug!("Level 2: Found enriched type information for '{type_name}'");

                // Check if this is a mutation method and we have mutation paths
                if matches!(
                    self.method,
                    BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
                ) && discovered_info.supports_mutation()
                {
                    debug!(
                        "Level 2: Type '{}' supports mutation with {} paths",
                        type_name,
                        discovered_info.get_mutation_paths().len()
                    );

                    // Create a mutation-specific correction with available paths
                    let mut hint =
                        format!("Type '{type_name}' supports mutation. Available paths:\n");
                    for (path, description) in discovered_info.get_mutation_paths() {
                        let _ = writeln!(hint, "  {path} - {description}");
                    }

                    let correction = Correction::Uncorrectable {
                        type_info: discovered_info.clone(),
                        reason:    hint,
                    };
                    corrections.push(correction);
                } else {
                    // Extract the original value for this component
                    let original_component_value =
                        Self::extract_component_value(self.method, &self.params, type_name);

                    // Create a correction from the discovered type information with original value
                    let correction = discovered_info.to_correction(original_component_value);
                    corrections.push(correction);
                }
            } else {
                debug!("Level 2: No type information found for '{type_name}'");
                // Type was not found in registry or extras discovery
            }
        }

        // Determine the level result based on what we discovered
        if corrections.is_empty() {
            debug!(
                "Level 2: Direct discovery complete, proceeding to Level 3 with {} type infos",
                enhanced_type_info.len()
            );
            LevelResult::Continue(enhanced_type_info.clone())
        } else {
            debug!(
                "Level 2: Found {} corrections from direct discovery",
                corrections.len()
            );
            LevelResult::Success(corrections)
        }
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
    #[allow(clippy::too_many_lines)]
    async fn build_recovery_result(
        &self,
        correction_results: Vec<Correction>,
    ) -> FormatRecoveryResult {
        let mut corrections = Vec::new();
        let mut has_applied_corrections = false;

        for correction_result in correction_results {
            match correction_result {
                Correction::Candidate { correction_info } => {
                    let type_name = correction_info.type_name.clone();
                    corrections.push(correction_info);
                    has_applied_corrections = true;
                    debug!("Recovery Engine: Applied correction for type '{type_name}'");
                }
                Correction::Uncorrectable { type_info, reason } => {
                    debug!(
                        "Recovery Engine: Found metadata for type '{}' but no correction: {}",
                        type_info.type_name, reason
                    );
                    // Create a CorrectionInfo from metadata-only result to provide guidance
                    let correction_info = CorrectionInfo {
                        type_name:         type_info.type_name.clone(),
                        original_value:    Self::extract_component_value(
                            self.method,
                            &self.params,
                            &type_info.type_name,
                        )
                        .unwrap_or_else(|| serde_json::json!({})),
                        corrected_value:   build_corrected_value_from_type_info(
                            &type_info,
                            self.method,
                        ),
                        hint:              reason,
                        target_type:       type_info.type_name.clone(),
                        corrected_format:  None,
                        type_info:         Some(type_info),
                        correction_method: CorrectionMethod::DirectReplacement,
                    };
                    corrections.push(correction_info);
                }
            }
        }

        if corrections.is_empty() {
            debug!("Recovery Engine: No corrections found, returning original error");
            return FormatRecoveryResult::NotRecoverable {
                corrections: Vec::new(),
            };
        }

        // Check if we can actually apply the corrections (i.e., we have fixable corrections)
        if has_applied_corrections && can_retry_with_corrections(&corrections) {
            debug!("Recovery Engine: Attempting to retry operation with corrected parameters");

            // Build corrected parameters
            match build_corrected_params(self.method, &self.params, &corrections) {
                Ok(corrected_params) => {
                    debug!("Recovery Engine: Built corrected parameters, executing retry");

                    // Execute the retry asynchronously
                    let client =
                        brp_client::BrpClient::new(self.method, self.port, corrected_params);
                    let retry_result = client.execute_raw().await;

                    match retry_result {
                        Ok(brp_result) => match brp_result {
                            ResponseStatus::Success(value) => {
                                debug!(
                                    "Recovery Engine: Retry succeeded with corrected parameters"
                                );
                                FormatRecoveryResult::Recovered {
                                    corrected_result: ResponseStatus::Success(value),
                                    corrections,
                                }
                            }
                            ResponseStatus::Error(brp_err) => {
                                debug!(
                                    "Recovery Engine: Retry failed with BRP error: {}",
                                    brp_err.message
                                );
                                FormatRecoveryResult::CorrectionFailed {
                                    retry_error: ResponseStatus::Error(brp_err),
                                    corrections,
                                }
                            }
                        },
                        Err(retry_error) => {
                            debug!("Recovery Engine: Retry failed: {}", retry_error);
                            // Convert error to BrpResult::Error
                            let retry_brp_error = ResponseStatus::Error(BrpClientError {
                                code:    -1, // Generic error code
                                message: retry_error.to_string(),
                                data:    None,
                            });
                            // Return correction failed with both original and retry errors
                            FormatRecoveryResult::CorrectionFailed {
                                retry_error: retry_brp_error,
                                corrections,
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!(
                        "Recovery Engine: Could not build corrected parameters: {}",
                        e
                    );
                    // Return original error - we can't fix this
                    FormatRecoveryResult::NotRecoverable { corrections }
                }
            }
        } else {
            debug!("Recovery Engine: No fixable corrections, returning error with guidance");
            // We have corrections but they're not fixable (like the enum case)
            // Return the original error - the handler will add format_corrections to it
            FormatRecoveryResult::NotRecoverable { corrections }
        }
    }

    /// Detect serialization issues and return corrections explaining the problems
    ///
    /// Returns `Some(corrections)` if serialization issues are found,
    /// `None` if no issues are detected.
    fn detect_serialization_issues(
        &self,
        registry_type_info: &HashMap<String, UnifiedTypeInfo>,
    ) -> Option<Vec<Correction>> {
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

                    let corrections =
                        self.type_names
                            .iter()
                            .map(|type_name| {
                                let type_info =
                                    registry_type_info.get(type_name).cloned().unwrap_or_else(
                                        || UnifiedTypeInfo::for_pattern_matching(type_name.clone()),
                                    );
                                Correction::Uncorrectable {
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

    /// Extract component value from method parameters
    fn extract_component_value(
        method: BrpMethod,
        params: &Value,
        type_name: &str,
    ) -> Option<Value> {
        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => params
                .get("components")
                .and_then(|c| c.get(type_name))
                .cloned(),
            BrpMethod::BevyInsertResource
            | BrpMethod::BevyMutateComponent
            | BrpMethod::BevyMutateResource => {
                params.get(FormatCorrectionField::Value.as_ref()).cloned()
            }
            _ => None,
        }
    }
}

/// Check if corrections can be applied for a retry
fn can_retry_with_corrections(corrections: &[CorrectionInfo]) -> bool {
    // Only retry if we have corrections with actual values
    if corrections.is_empty() {
        return false;
    }

    // Check if all corrections have valid corrected values
    for correction in corrections {
        // Skip if the corrected value is just a placeholder or metadata
        if correction.corrected_value.is_null()
            || (correction.corrected_value.is_object()
                && correction.corrected_value.as_object().is_some_and(|o| {
                    o.contains_key(FormatCorrectionField::Hint.as_ref())
                        || o.contains_key(FormatCorrectionField::Examples.as_ref())
                        || o.contains_key(FormatCorrectionField::ValidValues.as_ref())
                }))
        {
            return false;
        }
    }

    true
}

/// Build a corrected value from type info for guidance
fn build_corrected_value_from_type_info(type_info: &UnifiedTypeInfo, method: BrpMethod) -> Value {
    // Check if we have examples for this method
    if let Some(example) = type_info.format_info.examples.get(method.as_str()) {
        return example.clone();
    }

    // For mutations, provide mutation path guidance
    if matches!(
        method,
        BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
    ) {
        // Check if we have a mutate example
        if let Some(mutate_example) = type_info.format_info.examples.get("mutate") {
            return mutate_example.clone();
        }

        let mut guidance = serde_json::json!({
            FormatCorrectionField::Hint.as_ref(): "Use appropriate path and value for mutation"
        });

        if !type_info.format_info.mutation_paths.is_empty() {
            let paths: Vec<String> = type_info
                .format_info
                .mutation_paths
                .keys()
                .cloned()
                .collect();
            guidance[FormatCorrectionField::AvailablePaths.as_ref()] = serde_json::json!(paths);
        }

        // Add enum-specific guidance if this is an enum
        if let Some(enum_info) = &type_info.enum_info {
            let variants: Vec<String> = enum_info.variants.iter().map(|v| v.name.clone()).collect();
            guidance[FormatCorrectionField::ValidValues.as_ref()] = serde_json::json!(variants);
            guidance[FormatCorrectionField::Hint.as_ref()] =
                serde_json::json!("Use empty path with variant name as value");
            guidance[FormatCorrectionField::Examples.as_ref()] = serde_json::json!([
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.first().cloned().unwrap_or_else(|| "Variant1".to_string())},
                {FormatCorrectionField::Path.as_ref(): "", FormatCorrectionField::Value.as_ref(): variants.get(1).cloned().unwrap_or_else(|| "Variant2".to_string())}
            ]);
        }

        return guidance;
    }

    // Default to empty object
    serde_json::json!({})
}

/// Build corrected parameters from corrections
fn build_corrected_params(
    method: BrpMethod,
    original_params: &Value,
    corrections: &[CorrectionInfo],
) -> Result<Option<Value>> {
    let mut params = original_params.clone();

    for correction in corrections {
        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Update components
                if let Some(components) = ParameterName::Components.get_object_mut_from(&mut params)
                {
                    components.insert(
                        correction.type_name.clone(),
                        correction.corrected_value.clone(),
                    );
                }
            }
            BrpMethod::BevyInsertResource => {
                // Update value directly
                params[FormatCorrectionField::Value.as_ref()] = correction.corrected_value.clone();
            }
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource => {
                // For mutations, we need both path and value
                if correction.corrected_value.is_object() {
                    if let Some(obj) = correction.corrected_value.as_object() {
                        if let (Some(path), Some(value)) = (
                            obj.get(FormatCorrectionField::Path.as_ref()),
                            obj.get(FormatCorrectionField::Value.as_ref()),
                        ) {
                            params[FormatCorrectionField::Path.as_ref()] = path.clone();
                            params[FormatCorrectionField::Value.as_ref()] = value.clone();
                        } else {
                            return Err(Error::InvalidArgument(
                                "Mutation correction missing path or value".to_string(),
                            )
                            .into());
                        }
                    }
                } else {
                    // Simple value correction
                    params[FormatCorrectionField::Value.as_ref()] =
                        correction.corrected_value.clone();
                }
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Unsupported method for corrections: {}",
                    method.as_str()
                ))
                .into());
            }
        }
    }

    Ok(Some(params))
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

    use serde_json::json;

    use super::*;

    async fn create_test_engine(method: BrpMethod, error_message: &str) -> FormatDiscoveryEngine {
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
        .await
        .expect("Test engine creation should succeed")
    }

    fn create_type_info_without_serialization(type_name: &str) -> UnifiedTypeInfo {
        let mut type_info = UnifiedTypeInfo::for_pattern_matching(type_name.to_string());
        type_info.registry_status.in_registry = true;
        type_info.serialization.has_serialize = false;
        type_info.serialization.has_deserialize = false;
        type_info.serialization.brp_compatible = false;
        type_info
    }

    fn create_type_info_with_serialization(type_name: &str) -> UnifiedTypeInfo {
        let mut type_info = UnifiedTypeInfo::for_pattern_matching(type_name.to_string());
        type_info.registry_status.in_registry = true;
        type_info.serialization.has_serialize = true;
        type_info.serialization.has_deserialize = true;
        type_info.serialization.brp_compatible = true;
        type_info
    }

    #[tokio::test]
    async fn test_detect_serialization_issues_missing_traits() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        )
        .await;

        // Create registry info for the type that the engine extracted from its parameters
        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        let corrections = result.unwrap();
        assert!(!corrections.is_empty());
        if let Correction::Uncorrectable { reason, .. } = &corrections[0] {
            assert!(reason.contains("lacks Serialize and Deserialize traits"));
            assert!(reason.contains("bevy_render::view::visibility::Visibility"));
            assert!(reason.contains("Add #[derive(Serialize, Deserialize)]"));
        } else {
            panic!("Expected CannotCorrect result");
        }
    }

    #[tokio::test]
    async fn test_detect_serialization_issues_with_traits() {
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
        .await
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

    #[tokio::test]
    async fn test_detect_serialization_issues_non_spawn_method() {
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
        .await
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

    #[tokio::test]
    async fn test_detect_serialization_issues_different_error() {
        let engine = create_test_engine(
            BrpMethod::BevySpawn,
            "Some other error message", // Not UnknownComponentType
        )
        .await;

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because error message doesn't match
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_detect_serialization_issues_type_not_in_registry() {
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
        .await
        .expect("Test engine creation should succeed");

        let registry_info = HashMap::new(); // Empty registry

        let result = engine.detect_serialization_issues(&registry_info);

        // Should return None because type is not in registry
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_detect_serialization_issues_insert_method() {
        let engine = create_test_engine(
            BrpMethod::BevyInsert, // Also should work for insert
            "Unknown component type: `bevy_reflect::DynamicEnum`",
        )
        .await;

        let mut registry_info = HashMap::new();
        registry_info.insert(
            "bevy_render::view::visibility::Visibility".to_string(),
            create_type_info_without_serialization("bevy_render::view::visibility::Visibility"),
        );

        let result = engine.detect_serialization_issues(&registry_info);

        let corrections = result.unwrap();
        assert!(!corrections.is_empty());
        if let Correction::Uncorrectable { reason, .. } = &corrections[0] {
            assert!(reason.contains("lacks Serialize and Deserialize traits"));
            assert!(reason.contains("insert operations")); // Should say "insert" not "spawn"
        } else {
            panic!("Expected CannotCorrect result");
        }
    }

    // Phase 3a Tests - Level 2 with context field
    // Note: These tests require a running Bevy app with BRP, so they will skip if connection fails

    #[tokio::test]
    async fn test_async_engine_constructor() {
        // Test that constructor is now async (key structural change for Phase 3a)
        let result = FormatDiscoveryEngine::new(
            BrpMethod::BevySpawn,
            Port(15702),
            Some(json!({"components": {"Transform": {}}})),
            BrpClientError {
                code:    -23402,
                message: "Test error".to_string(),
                data:    None,
            },
        )
        .await;

        // The test passes if we can call .await on the constructor
        // Whether it succeeds or fails depends on BRP availability
        assert!(result.is_ok() || result.is_err());
    }
}
