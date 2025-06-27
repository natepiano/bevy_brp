//! Error detection and pattern matching logic for format discovery

use serde_json::Value;

use super::constants::{
    ACCESS_ERROR_REGEX, EXPECTED_TYPE_REGEX, MATH_TYPE_ARRAY_REGEX, MISSING_FIELD_REGEX,
    TRANSFORM_SEQUENCE_REGEX, TUPLE_STRUCT_PATH_REGEX, TYPE_MISMATCH_REGEX,
    UNKNOWN_COMPONENT_REGEX, UNKNOWN_COMPONENT_TYPE_REGEX, VARIANT_TYPE_MISMATCH_REGEX,
};
use crate::brp_tools::support::brp_client::{BrpError, BrpResult, execute_brp_method};
use crate::error::{Error, Result};
use crate::tools::BRP_METHOD_REGISTRY_SCHEMA;

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
}

/// Result of error pattern analysis
#[derive(Debug, Clone)]
pub struct ErrorAnalysis {
    pub pattern: Option<ErrorPattern>,
}

/// Result of registry checking for serialization support
#[derive(Debug, Clone)]
pub struct SerializationCheck {
    pub diagnostic_message: String,
}

/// Tier information for debugging
#[derive(Debug, Clone)]
pub struct TierInfo {
    pub tier:      u8,
    pub tier_name: String,
    pub action:    String,
    pub success:   bool,
}

impl TierInfo {
    /// Create a new `TierInfo` instance
    pub fn new(tier: u8, name: &str, action: String) -> Self {
        Self {
            tier,
            tier_name: name.to_string(),
            action,
            success: false,
        }
    }

    /// Mark this tier as successful and update the action message
    pub fn mark_success(&mut self, action: String) {
        self.success = true;
        self.action = action;
    }
}

/// Manager for tracking tier execution during format discovery
pub struct TierManager {
    tier_info: Vec<TierInfo>,
}

impl TierManager {
    /// Create a new `TierManager`
    pub const fn new() -> Self {
        Self {
            tier_info: Vec::new(),
        }
    }

    /// Start a new tier
    pub fn start_tier(&mut self, tier: u8, name: &str, action: String) {
        self.tier_info.push(TierInfo::new(tier, name, action));
    }

    /// Complete the current tier
    pub fn complete_tier(&mut self, success: bool, action: String) {
        if let Some(last) = self.tier_info.last_mut() {
            if success {
                last.mark_success(action);
            } else {
                last.action = action;
            }
        }
    }

    /// Convert into the underlying vector of tier info
    pub fn into_vec(self) -> Vec<TierInfo> {
        self.tier_info
    }
}

/// Consolidated pattern matcher that checks all patterns in a single pass
fn match_all_patterns(message: &str) -> Option<ErrorPattern> {
    // Try patterns in order of specificity/importance

    // 1. Access errors have highest priority
    if let Some(captures) = ACCESS_ERROR_REGEX.captures(message) {
        let access = captures[1].to_string();
        let error_type = captures[2].to_string();
        return Some(ErrorPattern::AccessError { access, error_type });
    }

    // 2. Type mismatch patterns (regular and variant)
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

    // 3. Missing field pattern
    if let Some(captures) = MISSING_FIELD_REGEX.captures(message) {
        let type_name = captures[1].to_string();
        let field_name = captures[2].to_string();
        return Some(ErrorPattern::MissingField {
            field_name,
            type_name,
        });
    }

    // 4. Unknown component pattern
    if let Some(captures) = UNKNOWN_COMPONENT_REGEX.captures(message) {
        let component_path = captures[1].to_string();
        return Some(ErrorPattern::UnknownComponent { component_path });
    }

    // 5. Transform sequence pattern
    if let Some(captures) = TRANSFORM_SEQUENCE_REGEX.captures(message) {
        if let Ok(count) = captures[1].parse::<usize>() {
            return Some(ErrorPattern::TransformSequence {
                expected_count: count,
            });
        }
    }

    // 6. Expected type pattern
    if let Some(captures) = EXPECTED_TYPE_REGEX.captures(message) {
        let expected_type = captures[1].to_string();
        return Some(ErrorPattern::ExpectedType { expected_type });
    }

    // 7. Math type array pattern
    if let Some(captures) = MATH_TYPE_ARRAY_REGEX.captures(message) {
        let math_type = captures[1].to_string();
        return Some(ErrorPattern::MathTypeArray { math_type });
    }

    // 8. Tuple struct path pattern
    if let Some(captures) = TUPLE_STRUCT_PATH_REGEX.captures(message) {
        let field_path = captures[1].to_string();
        return Some(ErrorPattern::TupleStructAccess { field_path });
    }

    // 9. Unknown component type pattern
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

/// Check if a type supports serialization by querying the registry schema
pub async fn check_type_serialization(
    type_name: &str,
    port: Option<u16>,
) -> Result<SerializationCheck> {
    // Query the registry schema for this specific type
    let schema_params = serde_json::json!({
        "with_types": ["Component", "Resource"],
        "with_crates": [extract_crate_name(type_name)]
    });

    let schema_result =
        execute_brp_method(BRP_METHOD_REGISTRY_SCHEMA, Some(schema_params), port).await?;

    match schema_result {
        BrpResult::Success(Some(schema_data)) => analyze_schema_for_type(type_name, &schema_data),
        BrpResult::Success(None) => Ok(SerializationCheck {
            diagnostic_message: format!("No schema data returned for type `{type_name}`"),
        }),
        BrpResult::Error(err) => Ok(SerializationCheck {
            diagnostic_message: format!(
                "Failed to query schema for type `{type_name}`: {}",
                err.message
            ),
        }),
    }
}

/// Extract crate name from a fully-qualified type name
fn extract_crate_name(type_name: &str) -> &str {
    // Extract the first part before :: for crate name
    // e.g., "bevy_transform::components::transform::Transform" -> "bevy_transform"
    type_name.split("::").next().unwrap_or(type_name)
}

/// Analyze schema data to determine serialization support for a type
fn analyze_schema_for_type(type_name: &str, schema_data: &Value) -> Result<SerializationCheck> {
    // Schema response can be either an array (old format) or an object (new format)
    // Try object format first (new format where keys are type names)
    if let Some(schema_obj) = schema_data.as_object() {
        // Direct lookup by type name
        if let Some(schema) = schema_obj.get(type_name) {
            return Ok(analyze_single_type_schema(type_name, schema));
        }
    } else if let Some(schemas) = schema_data.as_array() {
        // Fall back to array format (old format)
        for schema in schemas {
            if let Some(type_path) = schema.get("typePath").and_then(Value::as_str) {
                if type_path == type_name {
                    return Ok(analyze_single_type_schema(type_name, schema));
                }
            }
        }
    } else {
        return Err(error_stack::Report::new(Error::FormatDiscovery(
            "Unexpected schema response format: neither an array nor an object".to_string(),
        )));
    }

    // Type not found in schema
    Ok(SerializationCheck {
        diagnostic_message: format!(
            "Type `{type_name}` not found in registry schema. \
            This type may not be registered with BRP or may not exist."
        ),
    })
}

/// Analyze a single type's schema to check serialization support
fn analyze_single_type_schema(type_name: &str, schema: &Value) -> SerializationCheck {
    // Check its reflect types
    let reflect_types = schema
        .get("reflectTypes")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let has_serialize = reflect_types.contains(&"Serialize".to_string());
    let has_deserialize = reflect_types.contains(&"Deserialize".to_string());

    let diagnostic_message = if !has_serialize || !has_deserialize {
        let missing = if !has_serialize && !has_deserialize {
            "Serialize and Deserialize"
        } else if !has_serialize {
            "Serialize"
        } else {
            "Deserialize"
        };

        format!(
            "Type `{type_name}` cannot be used with BRP because it lacks {missing} trait(s). \
            Available traits: {}. \
            To fix this, the type definition needs both #[derive(Serialize, Deserialize)] \
            AND #[reflect(Serialize, Deserialize)] attributes.",
            reflect_types.join(", ")
        )
    } else {
        format!("Type `{type_name}` has proper serialization support")
    };

    SerializationCheck { diagnostic_message }
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

/// Convert tier information to debug strings
pub fn tier_info_to_debug_strings(tier_info: &[TierInfo]) -> Vec<String> {
    let mut debug_strings = Vec::new();

    if !tier_info.is_empty() {
        debug_strings.push("Tiered Format Discovery Results:".to_string());

        for info in tier_info {
            let status_icon = if info.success { "SUCCESS" } else { "FAILED" };
            debug_strings.push(format!(
                "  {} Tier {}: {} - {}",
                status_icon, info.tier, info.tier_name, info.action
            ));
        }
    }

    debug_strings
}
