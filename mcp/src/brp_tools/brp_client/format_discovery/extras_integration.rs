//! formerly: Integration with `bevy_brp_extras` for direct format discovery
//! Does this still need to exist?
use serde_json::{Value, json};
use tracing::debug;

use super::flow_types::CorrectionResult;
use super::format_correction_fields::FormatCorrectionField;
use super::unified_types::{CorrectionInfo, CorrectionMethod, UnifiedTypeInfo};

/// Create correction from discovered type info
pub fn create_correction_from_discovery(
    mut type_info: UnifiedTypeInfo,
    original_value: Option<Value>,
) -> CorrectionResult {
    // Ensure examples are generated
    type_info.ensure_examples();

    // Check if this is an enum with variants - create enum-specific correction
    if let Some(enum_info) = &type_info.enum_info {
        let variant_names: Vec<String> =
            enum_info.variants.iter().map(|v| v.name.clone()).collect();

        let corrected_format = json!({
            FormatCorrectionField::Hint.as_ref(): "Use empty path with variant name as value",
            FormatCorrectionField::ValidValues.as_ref(): variant_names,
            FormatCorrectionField::Examples.as_ref(): variant_names.iter().take(2).map(|variant| json!({
                FormatCorrectionField::Path.as_ref(): "",
                FormatCorrectionField::Value.as_ref(): variant
            })).collect::<Vec<_>>()
        });

        let correction_info = CorrectionInfo {
            type_name:         type_info.type_name.clone(),
            original_value:    original_value.unwrap_or(json!(null)),
            corrected_value:   corrected_format.clone(),
            corrected_format:  Some(corrected_format),
            hint:              format!(
                "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}",
                type_info
                    .type_name
                    .split("::")
                    .last()
                    .unwrap_or(&type_info.type_name),
                variant_names.join(", ")
            ),
            target_type:       type_info.type_name.clone(),
            type_info:         Some(type_info),
            correction_method: CorrectionMethod::DirectReplacement,
        };

        return CorrectionResult::Corrected { correction_info };
    }

    // Check if we can actually transform the original input
    if let Some(original_value) = original_value {
        debug!(
            "Extras Integration: Attempting to transform original value: {}",
            serde_json::to_string(&original_value).unwrap_or_else(|_| "invalid json".to_string())
        );
        if let Some(transformed_value) = type_info.transform_value(&original_value) {
            debug!(
                "Extras Integration: Successfully transformed value to: {}",
                serde_json::to_string(&transformed_value)
                    .unwrap_or_else(|_| "invalid json".to_string())
            );
            // We can transform the input - return Corrected with actual transformation
            let correction_info = CorrectionInfo {
                type_name:         type_info.type_name.clone(),
                original_value:    original_value.clone(),
                corrected_value:   transformed_value,
                hint:              format!(
                    "Transformed {} format for type '{}' (discovered via bevy_brp_extras)",
                    if original_value.is_object() {
                        "object"
                    } else {
                        "value"
                    },
                    type_info.type_name
                ),
                target_type:       type_info.type_name.clone(),
                corrected_format:  None,
                type_info:         Some(type_info),
                correction_method: CorrectionMethod::ObjectToArray,
            };

            return CorrectionResult::Corrected { correction_info };
        }
        debug!("Extras Integration: transform_value() returned None - cannot transform input");
    } else {
        debug!("Extras Integration: No original value provided for transformation");
    }

    // Cannot transform input - provide guidance with examples
    let reason = if let Some(spawn_example) = type_info.get_example("spawn") {
        format!(
            "Cannot transform input for type '{}'. Use this format: {}",
            type_info.type_name,
            serde_json::to_string(&spawn_example).unwrap_or_else(|_| "correct format".to_string())
        )
    } else {
        format!(
            "Cannot transform input for type '{}'. Type discovered but no format example available.",
            type_info.type_name
        )
    };

    CorrectionResult::CannotCorrect { type_info, reason }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::brp_tools::brp_client::format_discovery::unified_types::{
        DiscoverySource, TypeCategory,
    };

    #[test]
    fn test_create_correction_from_discovery_with_example() {
        let mut type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );
        type_info.type_category = TypeCategory::Struct;
        type_info.format_info.examples.insert(
            "spawn".to_string(),
            json!({
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }),
        );

        let original = json!({"translation": {"x": 0.0, "y": 0.0, "z": 0.0}});
        let result = create_correction_from_discovery(type_info, Some(original.clone()));

        match result {
            CorrectionResult::Corrected { correction_info } => {
                assert_eq!(correction_info.original_value, original);
                assert!(correction_info.corrected_value.get("translation").is_some());
                assert_eq!(
                    correction_info.correction_method,
                    CorrectionMethod::ObjectToArray
                );
            }
            CorrectionResult::CannotCorrect { .. } => {
                unreachable!("Expected Applied correction result")
            }
        }
    }

    #[test]
    fn test_create_correction_from_discovery_metadata_only() {
        let type_info = UnifiedTypeInfo::new(
            "bevy_transform::components::transform::Transform".to_string(),
            DiscoverySource::DirectDiscovery,
        );

        let result = create_correction_from_discovery(type_info, None);

        match result {
            CorrectionResult::CannotCorrect { type_info, reason } => {
                assert_eq!(
                    type_info.type_name,
                    "bevy_transform::components::transform::Transform"
                );
                assert!(reason.contains("no format example"));
            }
            CorrectionResult::Corrected { .. } => {
                unreachable!("Expected MetadataOnly correction result")
            }
        }
    }
}
