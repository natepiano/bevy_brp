//! Path parsing logic to convert strings to `FieldAccess` structs

use super::field_mapper;
use super::types::{ComponentType, FieldAccess};

/// Parses a path string like ".LinearRgba.red" into a `FieldAccess` struct
pub fn parse_path_to_field_access(path: &str) -> Option<FieldAccess> {
    // Simple field access (no component type) should be handled by the fallback logic
    // These remain as direct tuple indices (.0, .1, .2)
    if path.starts_with('.') && path.matches('.').count() == 1 {
        return None; // Let the fallback handle these
    }

    // Split the path into parts
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() < 3 || !parts[0].is_empty() {
        return None;
    }

    // Extract component type and field name
    let component_name = parts[1];
    let field_name = parts[2];

    // Parse component type
    let component_type = parse_component_type(component_name)?;

    // Parse field name
    let field = field_mapper::parse_field_name(field_name, component_type)?;

    Some(FieldAccess {
        component_type,
        field,
    })
}

/// Parses a component type name string into a `ComponentType` enum
fn parse_component_type(component_name: &str) -> Option<ComponentType> {
    match component_name {
        // Color types
        "LinearRgba" => Some(ComponentType::LinearRgba),
        "Srgba" => Some(ComponentType::Srgba),
        "Hsla" => Some(ComponentType::Hsla),
        "Hsva" => Some(ComponentType::Hsva),
        "Hwba" => Some(ComponentType::Hwba),
        "Laba" => Some(ComponentType::Laba),
        "Lcha" => Some(ComponentType::Lcha),
        "Oklaba" => Some(ComponentType::Oklaba),
        "Oklcha" => Some(ComponentType::Oklcha),
        "Xyza" => Some(ComponentType::Xyza),

        // Math types - floating point
        "Vec2" => Some(ComponentType::Vec2),
        "Vec3" => Some(ComponentType::Vec3),
        "Vec4" => Some(ComponentType::Vec4),
        "Quat" => Some(ComponentType::Quat),

        // Math types - signed integers
        "IVec2" => Some(ComponentType::IVec2),
        "IVec3" => Some(ComponentType::IVec3),
        "IVec4" => Some(ComponentType::IVec4),

        // Math types - unsigned integers
        "UVec2" => Some(ComponentType::UVec2),
        "UVec3" => Some(ComponentType::UVec3),
        "UVec4" => Some(ComponentType::UVec4),

        // Math types - double precision
        "DVec2" => Some(ComponentType::DVec2),
        "DVec3" => Some(ComponentType::DVec3),
        "DVec4" => Some(ComponentType::DVec4),

        _ => None,
    }
}

/// Checks if a variant name looks like an enum variant (starts with uppercase)
pub fn is_enum_variant(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Parses generic enum variant field access patterns
/// Handles cases where we don't have a specific component type mapping
pub fn parse_generic_enum_field_access(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() < 3 || !parts[0].is_empty() || parts[1].is_empty() || parts[2].is_empty() {
        return None;
    }

    let variant_name = parts[1];
    let field_name = parts[2];

    // Check if the second part looks like an enum variant (starts with uppercase)
    if !is_enum_variant(variant_name) {
        return None;
    }

    // For color enum variants, try to map common field names to indices
    match field_name {
        // Index 0: First position fields
        "red" | "r" | "hue" | "h" | "lightness" | "l" | "x" => Some(".0.0".to_string()),
        // Index 1: Second position fields (including special cases)
        "green" | "g" | "saturation" | "s" | "y" | "whiteness" | "chroma" | "c" => {
            Some(".0.1".to_string())
        }
        // Index 2: Third position fields
        "blue" | "b" | "value" | "v" | "z" | "blackness" => Some(".0.2".to_string()),
        // Index 3: Fourth position fields
        "alpha" | "w" => Some(".0.3".to_string()),
        // Special case for 'a' - could be alpha or Lab 'a' component
        "a" => {
            if variant_name.contains("Lab") {
                Some(".0.1".to_string()) // Lab 'a' component
            } else {
                Some(".0.3".to_string()) // Alpha component
            }
        }
        _ => {
            // Generic enum variant field access -> use tuple index 0 and preserve field path
            if parts.len() > 3 {
                let remaining = parts[2..].join(".");
                Some(format!(".0.{remaining}"))
            } else {
                Some(format!(".0.{field_name}"))
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::brp_tools::format_discovery::types::{ColorField, Field, MathField};

    #[test]
    fn test_parse_path_to_field_access() {
        // Test color path parsing
        let field_access = parse_path_to_field_access(".LinearRgba.red").unwrap();
        assert_eq!(field_access.component_type, ComponentType::LinearRgba);
        assert_eq!(field_access.field, Field::Color(ColorField::Red));

        let field_access = parse_path_to_field_access(".Hsla.saturation").unwrap();
        assert_eq!(field_access.component_type, ComponentType::Hsla);
        assert_eq!(field_access.field, Field::Color(ColorField::Saturation));

        // Test math path parsing
        let field_access = parse_path_to_field_access(".Vec3.x").unwrap();
        assert_eq!(field_access.component_type, ComponentType::Vec3);
        assert_eq!(field_access.field, Field::Math(MathField::X));

        // Test Lab 'a' disambiguation
        let field_access = parse_path_to_field_access(".Laba.a").unwrap();
        assert_eq!(field_access.component_type, ComponentType::Laba);
        assert_eq!(field_access.field, Field::Color(ColorField::A));

        let field_access = parse_path_to_field_access(".LinearRgba.a").unwrap();
        assert_eq!(field_access.component_type, ComponentType::LinearRgba);
        assert_eq!(field_access.field, Field::Color(ColorField::Alpha));
    }

    #[test]
    fn test_parse_component_type() {
        assert_eq!(
            parse_component_type("LinearRgba"),
            Some(ComponentType::LinearRgba)
        );
        assert_eq!(parse_component_type("Vec3"), Some(ComponentType::Vec3));
        assert_eq!(parse_component_type("Quat"), Some(ComponentType::Quat));
        assert_eq!(parse_component_type("InvalidType"), None);
    }

    #[test]
    fn test_is_enum_variant() {
        assert!(is_enum_variant("LinearRgba"));
        assert!(is_enum_variant("SomeVariant"));
        assert!(!is_enum_variant("lowercase"));
        assert!(!is_enum_variant(""));
    }

    #[test]
    fn test_parse_generic_enum_field_access() {
        // Test standard color field mappings
        assert_eq!(
            parse_generic_enum_field_access(".LinearRgba.red"),
            Some(".0.0".to_string())
        );
        assert_eq!(
            parse_generic_enum_field_access(".SomeColor.green"),
            Some(".0.1".to_string())
        );
        assert_eq!(
            parse_generic_enum_field_access(".AnyColor.alpha"),
            Some(".0.3".to_string())
        );

        // Test Lab 'a' disambiguation
        assert_eq!(
            parse_generic_enum_field_access(".SomeLabColor.a"),
            Some(".0.1".to_string())
        );
        assert_eq!(
            parse_generic_enum_field_access(".RegularColor.a"),
            Some(".0.3".to_string())
        );

        // Test generic field access
        assert_eq!(
            parse_generic_enum_field_access(".SomeEnum.custom_field"),
            Some(".0.custom_field".to_string())
        );

        // Test invalid paths
        assert_eq!(parse_generic_enum_field_access(".lowercase.field"), None);
        assert_eq!(parse_generic_enum_field_access(".SomeEnum"), None);
        assert_eq!(parse_generic_enum_field_access("no_dot_prefix"), None);
    }

    #[test]
    fn test_simple_field_paths() {
        use super::super::field_mapper::map_field_to_tuple_index;

        // Test simple field paths return None (handled by fallback)
        assert_eq!(parse_path_to_field_access(".x"), None);
        assert_eq!(parse_path_to_field_access(".y"), None);
        assert_eq!(parse_path_to_field_access(".z"), None);

        // Test the mapping result for compound paths
        let field_access = parse_path_to_field_access(".Vec3.x").unwrap();
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.0");
    }
}
