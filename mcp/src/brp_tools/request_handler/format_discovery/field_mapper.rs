//! Field to tuple index mapping logic

use super::types::{ColorField, ComponentType, Field, FieldAccess, MathField};

/// Maps a field access to its corresponding tuple index path
pub fn map_field_to_tuple_index(field_access: &FieldAccess) -> String {
    match &field_access.field {
        Field::Color(color_field) => {
            map_color_field_to_index(field_access.component_type, *color_field)
        }
        Field::Math(math_field) => map_math_field_to_index(*math_field),
    }
}

/// Maps color fields to their tuple indices based on the component type
fn map_color_field_to_index(component_type: ComponentType, field: ColorField) -> String {
    // All color types are wrapped in a tuple variant, so they start with .0
    // Then the actual color data is at the next level
    match (component_type, field) {
        // Handle index 0 cases: Lab lightness, RGB red, HSL/HSV/HWB hue, and XYZ x
        (
            ComponentType::Laba
            | ComponentType::Lcha
            | ComponentType::Oklaba
            | ComponentType::Oklcha,
            ColorField::Lightness,
        )
        | (
            ComponentType::LinearRgba | ComponentType::Srgba | ComponentType::Xyza,
            ColorField::Red,
        )
        | (ComponentType::Hsla | ComponentType::Hsva | ComponentType::Hwba, ColorField::Hue) => {
            ".0.0".to_string()
        }

        // Handle index 1 cases: RGB green, HSL/HSV saturation, HWB whiteness, Lab a, LCH chroma,
        // XYZ y
        (
            ComponentType::LinearRgba | ComponentType::Srgba | ComponentType::Xyza,
            ColorField::Green,
        )
        | (ComponentType::Hsla | ComponentType::Hsva, ColorField::Saturation)
        | (ComponentType::Hwba, ColorField::Whiteness)
        | (ComponentType::Laba | ComponentType::Oklaba, ColorField::A)
        | (ComponentType::Lcha | ComponentType::Oklcha, ColorField::Chroma) => ".0.1".to_string(),

        // Handle index 2 cases: HSL lightness, RGB blue, HSV value, HWB blackness, Lab b, LCH hue,
        // XYZ z
        (ComponentType::Hsla, ColorField::Lightness)
        | (
            ComponentType::LinearRgba | ComponentType::Srgba | ComponentType::Xyza,
            ColorField::Blue,
        )
        | (ComponentType::Hsva, ColorField::Value)
        | (ComponentType::Hwba, ColorField::Blackness)
        | (ComponentType::Laba | ComponentType::Oklaba, ColorField::B)
        | (ComponentType::Lcha | ComponentType::Oklcha, ColorField::Hue) => ".0.2".to_string(),

        // Alpha is always index 3 for all color types
        (_, ColorField::Alpha) => ".0.3".to_string(),

        // Fallback for invalid combinations (shouldn't happen in normal use)
        _ => ".invalid".to_string(),
    }
}

/// Maps math fields to their tuple indices
fn map_math_field_to_index(field: MathField) -> String {
    // Note: For compatibility with existing behavior, math types are treated as if
    // they were wrapped in tuple variants (like .0.0 instead of .0)
    // This maintains compatibility with the original implementation
    match field {
        MathField::X => ".0.0".to_string(),
        MathField::Y => ".0.1".to_string(),
        MathField::Z => ".0.2".to_string(),
        MathField::W => ".0.3".to_string(),
    }
}

/// Maps a simple field name to its corresponding field enum
pub fn parse_field_name(field_name: &str, component_type: ComponentType) -> Option<Field> {
    // Handle color types
    if component_type.is_color() {
        match field_name.to_lowercase().as_str() {
            "red" | "r" => Some(Field::Color(ColorField::Red)),
            "green" | "g" => Some(Field::Color(ColorField::Green)),
            "blue" | "b" => Some(Field::Color(ColorField::Blue)),
            "alpha" => Some(Field::Color(ColorField::Alpha)),
            "hue" | "h" => Some(Field::Color(ColorField::Hue)),
            "saturation" | "s" => Some(Field::Color(ColorField::Saturation)),
            "lightness" | "l" => Some(Field::Color(ColorField::Lightness)),
            "value" | "v" => Some(Field::Color(ColorField::Value)),
            "whiteness" | "w" => Some(Field::Color(ColorField::Whiteness)),
            "blackness" => Some(Field::Color(ColorField::Blackness)),
            "chroma" | "c" => Some(Field::Color(ColorField::Chroma)),
            // Special case for 'a' - could be alpha or Lab 'a' component
            "a" => {
                if component_type.is_lab_based() {
                    Some(Field::Color(ColorField::A))
                } else {
                    Some(Field::Color(ColorField::Alpha))
                }
            }
            // Special case for 'b' in color context
            _ if field_name == "b" && component_type.is_lab_based() => {
                Some(Field::Color(ColorField::B))
            }
            _ => None,
        }
    } else {
        // Handle math types
        match field_name.to_lowercase().as_str() {
            "x" => Some(Field::Math(MathField::X)),
            "y" => Some(Field::Math(MathField::Y)),
            "z" => Some(Field::Math(MathField::Z)),
            "w" => Some(Field::Math(MathField::W)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_field_mapping() {
        // Test RGB colors
        let field_access = FieldAccess {
            component_type: ComponentType::LinearRgba,
            field:          Field::Color(ColorField::Red),
        };
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.0");

        let field_access = FieldAccess {
            component_type: ComponentType::Srgba,
            field:          Field::Color(ColorField::Green),
        };
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.1");

        let field_access = FieldAccess {
            component_type: ComponentType::LinearRgba,
            field:          Field::Color(ColorField::Alpha),
        };
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.3");
    }

    #[test]
    fn test_math_field_mapping() {
        let field_access = FieldAccess {
            component_type: ComponentType::Vec3,
            field:          Field::Math(MathField::X),
        };
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.0");

        let field_access = FieldAccess {
            component_type: ComponentType::Vec4,
            field:          Field::Math(MathField::W),
        };
        assert_eq!(map_field_to_tuple_index(&field_access), ".0.3");
    }

    #[test]
    fn test_parse_field_name() {
        // Test color field parsing
        assert_eq!(
            parse_field_name("red", ComponentType::LinearRgba),
            Some(Field::Color(ColorField::Red))
        );
        assert_eq!(
            parse_field_name("r", ComponentType::Srgba),
            Some(Field::Color(ColorField::Red))
        );

        // Test 'a' disambiguation
        assert_eq!(
            parse_field_name("a", ComponentType::LinearRgba),
            Some(Field::Color(ColorField::Alpha))
        );
        assert_eq!(
            parse_field_name("a", ComponentType::Laba),
            Some(Field::Color(ColorField::A))
        );

        // Test math field parsing
        assert_eq!(
            parse_field_name("x", ComponentType::Vec3),
            Some(Field::Math(MathField::X))
        );
        assert_eq!(
            parse_field_name("w", ComponentType::Quat),
            Some(Field::Math(MathField::W))
        );
    }
}
