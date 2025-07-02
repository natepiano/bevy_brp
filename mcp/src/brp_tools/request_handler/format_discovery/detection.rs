//! Error detection and pattern matching logic for format discovery
//!
//! This module provides simplified error detection focused on the core functionality
//! needed by the new 3-level recovery architecture. Complex tier management has been
//! removed in favor of straightforward pattern matching and registry checks.

use super::constants::{
    ACCESS_ERROR_REGEX, ENUM_UNIT_VARIANT_ACCESS_ERROR_REGEX, ENUM_UNIT_VARIANT_REGEX,
    EXPECTED_TYPE_REGEX, MATH_TYPE_ARRAY_REGEX, MISSING_FIELD_REGEX, TRANSFORM_SEQUENCE_REGEX,
    TUPLE_STRUCT_PATH_REGEX, TYPE_MISMATCH_REGEX, UNKNOWN_COMPONENT_REGEX,
    UNKNOWN_COMPONENT_TYPE_REGEX, VARIANT_TYPE_MISMATCH_REGEX,
};
use crate::brp_tools::support::brp_client::BrpError;

/// Known error patterns that can be deterministically handled
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorPattern {
    /// Transform expects sequence of f32 values (e.g., "expected a sequence of 4 f32 values")
    TransformSequence { expected_count: usize },
    /// Component expects a specific type (e.g., "expected `bevy_ecs::name::Name`")
    ExpectedType { expected_type: String },
    /// Vec3/Quat math types expect array format
    MathTypeArray { math_type: String },
    /// Enum serialization issue - unknown component type
    UnknownComponentType { component_type: String },
    /// Tuple struct access error (e.g., "found a tuple struct instead")
    TupleStructAccess { field_path: String },
    /// Bevy `AccessError`: Error accessing element with X access
    AccessError {
        access:     String,
        error_type: String,
    },
    /// Type mismatch: Expected X access to access Y, found Z instead (includes variant mismatches)
    TypeMismatch {
        expected:   String,
        actual:     String,
        access:     String,
        is_variant: bool,
    },
    /// Missing field in struct/tuple
    MissingField {
        field_name: String,
        type_name:  String,
    },
    /// Unknown component type from BRP
    UnknownComponent { component_path: String },
    /// Enum unit variant mutation error
    EnumUnitVariantMutation {
        expected_variant_type: String,
        actual_variant_type:   String,
    },
    /// Enum unit variant access error with prefix
    EnumUnitVariantAccessError {
        access:                String,
        expected_variant_type: String,
        actual_variant_type:   String,
    },
}

/// Result of error pattern analysis
#[derive(Debug, Clone)]
pub struct ErrorAnalysis {
    pub pattern: Option<ErrorPattern>,
}

/// Consolidated pattern matcher that checks all patterns in a single pass
fn match_all_patterns(message: &str) -> Option<ErrorPattern> {
    // Try patterns in order of specificity/importance

    // 1. Enum unit variant access error (must be checked before general ACCESS_ERROR_REGEX)
    if let Some(captures) = ENUM_UNIT_VARIANT_ACCESS_ERROR_REGEX.captures(message) {
        let access = captures[1].to_string();
        let expected_variant_type = captures[2].to_string();
        let actual_variant_type = captures[3].to_string();
        return Some(ErrorPattern::EnumUnitVariantAccessError {
            access,
            expected_variant_type,
            actual_variant_type,
        });
    }

    // 2. Access errors have high priority
    if let Some(captures) = ACCESS_ERROR_REGEX.captures(message) {
        let access = captures[1].to_string();
        let error_type = captures[2].to_string();
        return Some(ErrorPattern::AccessError { access, error_type });
    }

    // 3. Type mismatch patterns (regular and variant)
    if let Some(captures) = TYPE_MISMATCH_REGEX.captures(message) {
        let access = captures[1].to_string();
        let expected = captures[2].to_string();
        let actual = captures[3].to_string();
        return Some(ErrorPattern::TypeMismatch {
            expected,
            actual,
            access,
            is_variant: false,
        });
    }

    if let Some(captures) = VARIANT_TYPE_MISMATCH_REGEX.captures(message) {
        let access = captures[1].to_string();
        let expected = captures[2].to_string();
        let actual = captures[3].to_string();
        return Some(ErrorPattern::TypeMismatch {
            expected,
            actual,
            access,
            is_variant: true,
        });
    }

    // 4. Enum unit variant mutation pattern
    if let Some(captures) = ENUM_UNIT_VARIANT_REGEX.captures(message) {
        let expected_variant_type = captures[1].to_string();
        let actual_variant_type = captures[2].to_string();
        return Some(ErrorPattern::EnumUnitVariantMutation {
            expected_variant_type,
            actual_variant_type,
        });
    }

    // 5. Missing field pattern
    if let Some(captures) = MISSING_FIELD_REGEX.captures(message) {
        let type_name = captures[1].to_string();
        let field_name = captures[2].to_string();
        return Some(ErrorPattern::MissingField {
            field_name,
            type_name,
        });
    }

    // 6. Unknown component pattern
    if let Some(captures) = UNKNOWN_COMPONENT_REGEX.captures(message) {
        let component_path = captures[1].to_string();
        return Some(ErrorPattern::UnknownComponent { component_path });
    }

    // 7. Transform sequence pattern
    if let Some(captures) = TRANSFORM_SEQUENCE_REGEX.captures(message) {
        if let Ok(count) = captures[1].parse::<usize>() {
            return Some(ErrorPattern::TransformSequence {
                expected_count: count,
            });
        }
    }

    // 8. Expected type pattern
    if let Some(captures) = EXPECTED_TYPE_REGEX.captures(message) {
        let expected_type = captures[1].to_string();
        return Some(ErrorPattern::ExpectedType { expected_type });
    }

    // 9. Math type array pattern
    if let Some(captures) = MATH_TYPE_ARRAY_REGEX.captures(message) {
        let math_type = captures[1].to_string();
        return Some(ErrorPattern::MathTypeArray { math_type });
    }

    // 10. Tuple struct path pattern
    if let Some(captures) = TUPLE_STRUCT_PATH_REGEX.captures(message) {
        let field_path = captures[1].to_string();
        return Some(ErrorPattern::TupleStructAccess { field_path });
    }

    // 11. Unknown component type pattern
    if let Some(captures) = UNKNOWN_COMPONENT_TYPE_REGEX.captures(message) {
        let component_type = captures[1].to_string();
        return Some(ErrorPattern::UnknownComponentType { component_type });
    }

    None
}

/// Analyze error message to identify known patterns using exact regex matching
pub fn analyze_error_pattern(error: &BrpError) -> ErrorAnalysis {
    ErrorAnalysis {
        pattern: match_all_patterns(&error.message),
    }
}

/// Helper function to extract context from errors
pub fn extract_path_from_error_context(error_message: &str) -> Option<String> {
    // Look for patterns like "at path .foo.bar" or "path '.foo.bar'"
    error_message.find("at path ").map_or_else(
        || {
            error_message
                .find("path '")
                .or_else(|| error_message.find("path \""))
                .and_then(|pos| extract_path_from_position(error_message, pos + 6))
        },
        |pos| extract_path_from_position(error_message, pos + 8),
    )
}

#[allow(dead_code)]
fn extract_path_from_position(error_message: &str, start_pos: usize) -> Option<String> {
    let path_start = &error_message[start_pos..];

    // Find the end of the path (stop at quotes, spaces, or end of string)
    let end_chars = [' ', '\'', '"', '\n'];
    let path_end = path_start
        .find(|c| end_chars.contains(&c))
        .unwrap_or(path_start.len());

    let path = &path_start[..path_end];

    // Validate that it looks like a path (starts with . or contains .)
    if path.starts_with('.') || path.contains('.') {
        Some(path.to_string())
    } else {
        None
    }
}

// Phase 4: Legacy tier system completely removed
// Debug information is now handled by the unified recovery engine
// All tier management logic has been replaced by the 3-level recovery system
