//! Tests for format discovery functionality

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use serde_json::json;

use super::detection::{ErrorPattern, analyze_error_pattern};
use super::types::TypeCategory;
use crate::brp_tools::BrpClientError;
use crate::brp_tools::brp_client::constants::BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE;

#[test]
fn test_analyze_error_pattern_tuple_struct_access() {
    let error = BrpClientError {
        code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
        message: "Error accessing element with Field access at path .LinearRgba.red".to_string(),
        data:    None,
    };

    let analysis = analyze_error_pattern(&error);
    assert!(analysis.pattern.is_some());

    assert!(
        matches!(
            analysis.pattern,
            Some(ErrorPattern::TupleStructAccess { .. })
        ),
        "Expected TupleStructAccess pattern, got: {:?}",
        analysis.pattern
    );
    if let Some(ErrorPattern::TupleStructAccess { field_path }) = analysis.pattern {
        assert_eq!(field_path, ".LinearRgba.red");
    }
}

#[test]
fn test_analyze_error_pattern_transform_sequence() {
    let error = BrpClientError {
        code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
        message: "Transform component expected a sequence of 3 f32 values".to_string(),
        data:    None,
    };

    let analysis = analyze_error_pattern(&error);
    assert!(analysis.pattern.is_some());

    assert!(
        matches!(
            analysis.pattern,
            Some(ErrorPattern::TransformSequence { .. })
        ),
        "Expected TransformSequence pattern, got: {:?}",
        analysis.pattern
    );
    if let Some(ErrorPattern::TransformSequence { expected_count }) = analysis.pattern {
        assert_eq!(expected_count, 3);
    }
}

#[test]
fn test_analyze_error_pattern_expected_type() {
    let error = BrpClientError {
        code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
        message: "expected `bevy_ecs::name::Name`".to_string(),
        data:    None,
    };

    let analysis = analyze_error_pattern(&error);
    assert!(analysis.pattern.is_some());

    assert!(
        matches!(analysis.pattern, Some(ErrorPattern::ExpectedType { .. })),
        "Expected ExpectedType pattern, got: {:?}",
        analysis.pattern
    );
    if let Some(ErrorPattern::ExpectedType { expected_type }) = analysis.pattern {
        assert_eq!(expected_type, "bevy_ecs::name::Name");
    }
}

#[test]
fn test_analyze_error_pattern_math_type_array() {
    let error = BrpClientError {
        code:    BRP_ERROR_CODE_UNKNOWN_COMPONENT_TYPE,
        message: "Vec3 expects array format".to_string(),
        data:    None,
    };

    let analysis = analyze_error_pattern(&error);
    assert!(analysis.pattern.is_some());

    assert!(
        matches!(analysis.pattern, Some(ErrorPattern::MathTypeArray { .. })),
        "Expected MathTypeArray pattern, got: {:?}",
        analysis.pattern
    );
    if let Some(ErrorPattern::MathTypeArray { math_type }) = analysis.pattern {
        assert_eq!(math_type, "Vec3");
    }
}

// Integration tests for TypeDiscoveryResponse → UnifiedTypeInfo conversion
// and mutation_paths preservation (Phase 5)

#[test]
fn test_enrich_from_extras_full_enrichment() {
    use super::types::DiscoverySource;
    use super::unified_types::UnifiedTypeInfo;

    // Simulate extras discovery response JSON with full metadata
    let extras_response = json!({
        "type_name": "bevy_transform::components::transform::Transform",
        "in_registry": true,
        "has_serialize": true,
        "has_deserialize": true,
        "type_category": "Struct",
        "supported_operations": ["spawn", "insert", "mutate"],
        "example_values": {
            "spawn": {
                "translation": [1.0, 2.0, 3.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            },
            "insert": {
                "translation": [0.0, 0.0, 0.0],
                "rotation": [0.0, 0.0, 0.0, 1.0],
                "scale": [1.0, 1.0, 1.0]
            }
        },
        "mutation_paths": {
            ".translation.x": "X component of translation vector",
            ".translation.y": "Y component of translation vector",
            ".translation.z": "Z component of translation vector",
            ".rotation.w": "W component of rotation quaternion",
            ".scale.x": "X component of scale vector"
        }
    });

    // Start with a registry-based UnifiedTypeInfo
    let mut info = UnifiedTypeInfo::for_transform_type(
        "bevy_transform::components::transform::Transform".to_string(),
        None,
    );

    // Verify initial state
    assert_eq!(info.discovery_source, DiscoverySource::PatternMatching);
    assert!(info.format_info.examples.is_empty() || info.format_info.examples.len() <= 2);
    let initial_mutation_count = info.format_info.mutation_paths.len();

    // Enrich with extras data
    info.enrich_from_extras(&extras_response);

    // Verify enrichment occurred
    assert_eq!(info.discovery_source, DiscoverySource::RegistryPlusExtras);

    // Verify basic type information is preserved from original
    assert_eq!(
        info.type_name,
        "bevy_transform::components::transform::Transform"
    );
    assert_eq!(info.type_category, TypeCategory::Struct); // for_transform_type creates Struct category

    // Note: Registry status, serialization, and supported_operations are not enriched by extras
    // They remain as set by the original constructor

    // Critical: Verify mutation_paths are enriched from extras (additive merge)
    let final_mutation_count = info.format_info.mutation_paths.len();
    assert!(
        final_mutation_count > initial_mutation_count,
        "Mutation paths should be added from extras"
    );

    // Verify specific mutation paths from extras are added
    assert_eq!(
        info.format_info.mutation_paths.get(".translation.x"),
        Some(&"X component of translation vector".to_string())
    );
    assert_eq!(
        info.format_info.mutation_paths.get(".rotation.w"),
        Some(&"W component of rotation quaternion".to_string())
    );

    // Verify example values are enriched from extras
    assert!(
        !info.format_info.examples.is_empty(),
        "Examples should be preserved"
    );
    assert!(info.format_info.examples.contains_key("spawn"));
    assert!(info.format_info.examples.contains_key("insert"));
}

#[test]
fn test_enrich_from_extras_edge_cases() {
    use super::types::DiscoverySource;
    use super::unified_types::UnifiedTypeInfo;

    // Test 1: Enrichment with empty extras response (no example_values or mutation_paths)
    let minimal_response = json!({
        "type_name": "bevy_ecs::name::Name",
        "in_registry": false,
        "has_serialize": false,
        "has_deserialize": false
    });

    let mut info = UnifiedTypeInfo::for_pattern_matching("bevy_ecs::name::Name".to_string(), None);
    let initial_source = info.discovery_source.clone();

    // This should not enrich anything since no example_values or mutation_paths
    info.enrich_from_extras(&minimal_response);

    // Discovery source should remain unchanged since no enrichment occurred
    assert_eq!(info.discovery_source, initial_source);
    assert_eq!(info.type_name, "bevy_ecs::name::Name");

    // Test 2: Enrichment with complex nested mutation paths
    let complex_response = json!({
        "type_name": "custom::ComplexType",
        "in_registry": true,
        "has_serialize": true,
        "has_deserialize": true,
        "example_values": {
            "spawn": {"value": 42}
        },
        "mutation_paths": {
            ".outer.inner.deep.value": "Deeply nested value",
            ".array[0].field": "First array element field",
            ".variant.SomeVariant.data": "Enum variant data",
            ".map['key'].nested": "Map value nested field"
        }
    });

    let mut info = UnifiedTypeInfo::for_pattern_matching("custom::ComplexType".to_string(), None);
    info.enrich_from_extras(&complex_response);

    // Verify enrichment occurred
    assert_eq!(info.discovery_source, DiscoverySource::RegistryPlusExtras);

    // Verify all complex mutation paths are added
    assert_eq!(info.format_info.mutation_paths.len(), 4);
    assert!(
        info.format_info
            .mutation_paths
            .contains_key(".outer.inner.deep.value")
    );
    assert!(
        info.format_info
            .mutation_paths
            .contains_key(".array[0].field")
    );
    assert!(
        info.format_info
            .mutation_paths
            .contains_key(".variant.SomeVariant.data")
    );
    assert!(
        info.format_info
            .mutation_paths
            .contains_key(".map['key'].nested")
    );

    // Verify example was added
    assert!(info.format_info.examples.contains_key("spawn"));
}

#[test]
fn test_registry_schema_to_unified_type_info_conversion() {
    use super::unified_types::UnifiedTypeInfo;

    // Simulate registry schema response
    let schema_data = json!({
        "typePath": "bevy_render::color::Color",
        "shortPath": "Color",
        "reflectTypes": ["Component", "Serialize", "Deserialize", "Default"],
        "properties": {
            "fields": [
                {
                    "name": "r",
                    "type": "f32",
                    "doc": "Red component"
                },
                {
                    "name": "g",
                    "type": "f32",
                    "doc": "Green component"
                },
                {
                    "name": "b",
                    "type": "f32",
                    "doc": "Blue component"
                },
                {
                    "name": "a",
                    "type": "f32",
                    "doc": "Alpha component"
                }
            ]
        }
    });

    let unified_info =
        UnifiedTypeInfo::from_registry_schema("bevy_render::color::Color", &schema_data, None);

    // Verify type information from registry
    assert_eq!(unified_info.type_name, "bevy_render::color::Color");

    // Verify registry status is correctly set
    assert!(unified_info.registry_status.in_registry);
    assert!(unified_info.registry_status.has_reflect);

    // Verify serialization support from reflect traits
    assert!(unified_info.serialization.has_serialize);
    assert!(unified_info.serialization.has_deserialize);
    assert!(unified_info.serialization.brp_compatible);
}
