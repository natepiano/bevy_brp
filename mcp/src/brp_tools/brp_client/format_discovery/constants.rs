//! Constants and static regex patterns for format discovery

use std::sync::LazyLock;

use regex::Regex;

/// Macro to define regex patterns with consistent error handling
macro_rules! define_regex {
    ($name:ident, $pattern:expr) => {
        pub static $name: LazyLock<Regex> = LazyLock::new(|| {
            // This regex pattern is known to be valid at compile time
            Regex::new($pattern).unwrap_or_else(|_| {
                // Fallback regex that matches nothing - should never happen
                Regex::new(r"$^").unwrap()
            })
        });
    };
}

/// Field name for "name" property
pub const FIELD_NAME: &str = "name";

/// Field name for "text" property
pub const FIELD_TEXT: &str = "text";

/// Field name for "label" property
pub const FIELD_LABEL: &str = "label";

/// Expected count of f32 values in a Transform sequence
/// Transform contains: translation (Vec3), rotation (Quat), scale (Vec3) = 3 + 4 + 3 = 10 f32
/// values However, the Transform component includes additional fields that bring the total to 12
/// f32 values
pub const TRANSFORM_SEQUENCE_F32_COUNT: usize = 12;

// Static regex patterns for error analysis - Based on exact Bevy error strings
define_regex!(
    TRANSFORM_SEQUENCE_REGEX,
    r"expected a sequence of (\d+) f32 values"
);
define_regex!(
    EXPECTED_TYPE_REGEX,
    r"expected `([a-zA-Z_:]+(?::[a-zA-Z_:]+)*)`"
);
define_regex!(
    ACCESS_ERROR_REGEX,
    r"Error accessing element with `([^`]+)` access(?:\s*\(offset \d+\))?: (.+)"
);
define_regex!(
    TYPE_MISMATCH_REGEX,
    r"Expected ([a-zA-Z0-9_\[\]]+) access to access a ([a-zA-Z0-9_]+), found a ([a-zA-Z0-9_]+) instead\."
);
define_regex!(
    VARIANT_TYPE_MISMATCH_REGEX,
    r"Expected variant ([a-zA-Z0-9_\[\]]+) access to access a ([a-zA-Z0-9_]+) variant, found a ([a-zA-Z0-9_]+) variant instead\."
);
define_regex!(
    MISSING_FIELD_REGEX,
    r#"The ([a-zA-Z0-9_]+) accessed doesn't have (?:an? )?[`"]([^`"]+)[`"] field"#
);
define_regex!(
    UNKNOWN_COMPONENT_REGEX,
    r"Unknown component type: `([^`]+)`"
);
define_regex!(
    TUPLE_STRUCT_PATH_REGEX,
    r#"(?:at path|path)\s+[`"]?([^`"\s]+)[`"]?"#
);
define_regex!(
    MATH_TYPE_ARRAY_REGEX,
    r"(Vec2|Vec3|Vec4|Quat)\s+(?:expects?|requires?|needs?)\s+array"
);
define_regex!(
    ENUM_UNIT_VARIANT_REGEX,
    r"Expected variant field access to access a ([a-zA-Z]+) variant, found a ([a-zA-Z]+) variant instead"
);
define_regex!(
    ENUM_UNIT_VARIANT_ACCESS_ERROR_REGEX,
    r"Error accessing element with `([^`]+)` access(?:\s*\(offset \d+\))?: Expected variant field access to access a ([a-zA-Z]+) variant, found a ([a-zA-Z]+) variant instead"
);
