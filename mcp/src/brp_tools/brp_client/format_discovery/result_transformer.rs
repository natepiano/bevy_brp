//! Result transformation utilities for format discovery
//!
//! This module handles the transformation of format recovery results into the final
//! client response format, including error enhancement and correction metadata.

use serde_json::Value;

use super::flow_types::FormatRecoveryResult;
use super::format_correction_fields::FormatCorrectionField;
use super::{CorrectionInfo, FormatCorrection, FormatCorrectionStatus};
use crate::brp_tools::brp_client::types::{
    BrpClientError, FormatDiscoveryError, ResponseStatus, ResultStructBrpExt,
};
use crate::error::{Error, Result};

/// Transform format recovery result into typed result
pub fn transform_recovery_result<R>(
    recovery_result: FormatRecoveryResult,
    original_error: &BrpClientError,
) -> Result<R>
where
    R: ResultStructBrpExt<
        Args = (
            Option<Value>,
            Option<Vec<Value>>,
            Option<FormatCorrectionStatus>,
        ),
    >,
{
    match recovery_result {
        FormatRecoveryResult::Recovered {
            corrected_result,
            corrections,
        } => {
            // Successfully recovered with format corrections
            // Extract the success value from corrected_result
            match corrected_result {
                ResponseStatus::Success(value) => {
                    // Convert CorrectionInfo to FormatCorrection if needed
                    let format_corrections = convert_corrections(corrections);
                    R::from_brp_client_response((
                        value,
                        Some(
                            format_corrections
                                .into_iter()
                                .map(|c| format_correction_to_json(&c))
                                .collect(),
                        ),
                        Some(FormatCorrectionStatus::Succeeded),
                    ))
                }
                ResponseStatus::Error(err) => {
                    // Recovery succeeded but result contains error - shouldn't happen
                    Err(Error::tool_call_failed(format!(
                        "Format recovery succeeded but result contains error: {}",
                        err.message
                    ))
                    .into())
                }
            }
        }
        FormatRecoveryResult::NotRecoverable { corrections } => {
            // Format discovery couldn't fix it but has guidance
            let enhanced_error = create_format_discovery_error(
                original_error,
                "Format errors not recoverable but guidance available",
                &corrections,
            );
            Err(enhanced_error.into())
        }
        FormatRecoveryResult::CorrectionFailed {
            retry_error,
            corrections,
        } => {
            // Format discovery tried but the correction failed
            let retry_error_msg = match retry_error {
                ResponseStatus::Error(ref err) => &err.message,
                ResponseStatus::Success(_) => "Unknown error",
            };
            let enhanced_error = create_format_discovery_error(
                original_error,
                &format!("Correction attempted but failed: {retry_error_msg}"),
                &corrections,
            );
            Err(enhanced_error.into())
        }
    }
}

/// Convert `CorrectionInfo` to `FormatCorrection`
pub fn convert_corrections(corrections: Vec<CorrectionInfo>) -> Vec<FormatCorrection> {
    corrections
        .into_iter()
        .map(|info| {
            // Extract rich metadata from type_info if available
            let (supported_operations, mutation_paths, type_category) = info
                .type_info
                .as_ref()
                .map_or((None, None, None), |type_info| {
                    (
                        Some(type_info.supported_operations.clone()),
                        Some(
                            type_info
                                .format_info
                                .mutation_paths
                                .keys()
                                .cloned()
                                .collect(),
                        ),
                        // Convert TypeCategory to string using serde serialization
                        serde_json::to_value(&type_info.type_category)
                            .ok()
                            .and_then(|v| v.as_str().map(ToString::to_string)),
                    )
                });

            FormatCorrection {
                component: info.type_name,
                original_format: info.original_value,
                corrected_format: info.corrected_value,
                hint: info.hint,
                supported_operations,
                mutation_paths,
                type_category,
            }
        })
        .collect()
}

/// Convert a `FormatCorrection` to JSON representation with metadata
pub fn format_correction_to_json(correction: &FormatCorrection) -> Value {
    let mut correction_json = serde_json::json!({
        FormatCorrectionField::Component.as_ref(): correction.component,
        FormatCorrectionField::OriginalFormat.as_ref(): correction.original_format,
        FormatCorrectionField::CorrectedFormat.as_ref(): correction.corrected_format,
        FormatCorrectionField::Hint.as_ref(): correction.hint
    });
    // Add rich metadata fields if available
    if let Some(obj) = correction_json.as_object_mut() {
        if let Some(ops) = &correction.supported_operations {
            obj.insert(
                FormatCorrectionField::SupportedOperations
                    .as_ref()
                    .to_string(),
                serde_json::json!(ops),
            );
        }
        if let Some(paths) = &correction.mutation_paths {
            obj.insert(
                FormatCorrectionField::MutationPaths.as_ref().to_string(),
                serde_json::json!(paths),
            );
        }
        if let Some(cat) = &correction.type_category {
            obj.insert(
                FormatCorrectionField::TypeCategory.as_ref().to_string(),
                serde_json::json!(cat),
            );
        }
    }
    correction_json
}

/// Create enhanced error for format discovery failures
pub fn create_format_discovery_error(
    original_error: &BrpClientError,
    reason: &str,
    corrections: &[CorrectionInfo],
) -> Error {
    // Build format corrections array with metadata
    // Always include the array (even if empty) to meet test expectations
    let format_corrections = Some(
        corrections
            .iter()
            .map(|c| {
                let correction = FormatCorrection {
                    component:            c.type_name.clone(),
                    original_format:      c.original_value.clone(),
                    corrected_format:     c.corrected_value.clone(),
                    hint:                 c.hint.clone(),
                    supported_operations: c
                        .type_info
                        .as_ref()
                        .map(|ti| ti.supported_operations.clone()),
                    mutation_paths:       c.type_info.as_ref().and_then(|ti| {
                        let paths = &ti.format_info.mutation_paths;
                        if paths.is_empty() {
                            None
                        } else {
                            Some(paths.keys().cloned().collect())
                        }
                    }),
                    type_category:        c.type_info.as_ref().map(|ti| {
                        // Use debug format since TypeCategory is not publicly accessible
                        format!("{:?}", ti.type_category)
                    }),
                };
                format_correction_to_json(&correction)
            })
            .collect(),
    );

    // Build hint message from corrections
    let hint = if corrections.is_empty() {
        "No format corrections available. Check that the types have Serialize/Deserialize traits."
            .to_string()
    } else {
        corrections
            .iter()
            .filter_map(|c| {
                if c.hint.is_empty() {
                    None
                } else {
                    Some(format!("- {}: {}", c.type_name, c.hint))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let format_discovery_error = FormatDiscoveryError::new(
        "not_attempted".to_string(),
        if hint.is_empty() {
            "Format discovery found issues but could not provide specific guidance.".to_string()
        } else {
            hint
        },
        format_corrections,
        Some(original_error.code),
        reason.to_string(),
        original_error.message.clone(),
    );

    Error::Structured {
        result: Box::new(format_discovery_error),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Note: We can't directly test convert_corrections because CorrectionInfo creation
    // requires private types. The integration tests in format_discovery.md will verify
    // the full flow. Here we test the JSON serialization which is what the API returns.

    #[test]
    #[allow(clippy::expect_used)]
    fn test_format_correction_to_json() {
        // Test that format_correction_to_json properly includes metadata when available
        let correction_with_metadata = FormatCorrection {
            component:            "bevy_transform::components::transform::Transform".to_string(),
            original_format:      json!({"translation": {"x": 1.0, "y": 2.0, "z": 3.0}}),
            corrected_format:     json!({"translation": [1.0, 2.0, 3.0]}),
            hint:                 "Transformed to array format".to_string(),
            supported_operations: Some(vec!["spawn".to_string(), "insert".to_string()]),
            mutation_paths:       Some(vec![
                ".translation.x".to_string(),
                ".translation.y".to_string(),
            ]),
            type_category:        Some("Component".to_string()),
        };

        let json_output = format_correction_to_json(&correction_with_metadata);

        // Verify all fields are present
        assert_eq!(
            json_output["component"],
            "bevy_transform::components::transform::Transform"
        );
        assert_eq!(json_output["hint"], "Transformed to array format");
        assert_eq!(
            json_output["supported_operations"],
            json!(["spawn", "insert"])
        );
        assert_eq!(
            json_output["mutation_paths"],
            json!([".translation.x", ".translation.y"])
        );
        assert_eq!(json_output["type_category"], "Component");

        // Test without metadata
        let correction_without_metadata = FormatCorrection {
            component:            "TestType".to_string(),
            original_format:      json!({}),
            corrected_format:     json!({}),
            hint:                 "Test".to_string(),
            supported_operations: None,
            mutation_paths:       None,
            type_category:        None,
        };

        let json_output = format_correction_to_json(&correction_without_metadata);

        // Verify metadata fields are not present when None
        let json_obj = json_output
            .as_object()
            .expect("JSON output should be an object");
        assert!(!json_obj.contains_key("supported_operations"));
        assert!(!json_obj.contains_key("mutation_paths"));
        assert!(!json_obj.contains_key("type_category"));
    }
}
