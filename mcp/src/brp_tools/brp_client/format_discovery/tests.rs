//! Tests for format discovery functionality

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::detection::{ErrorPattern, analyze_error_pattern};
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
