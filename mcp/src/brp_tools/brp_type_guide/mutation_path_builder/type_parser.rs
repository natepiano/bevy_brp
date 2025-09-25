//! Parser for Rust type paths with support for nested generics
//!
//! This module uses nom to properly parse type paths like:
//! - `Color::Srgba`
//! - `Option<T>::Some`
//! - `Option<Handle<Mesh>>::Some`
//! - `core::option::Option<bevy_asset::handle::Handle<bevy_mesh::mesh::Mesh>>::Some`

use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::char;
use nom::combinator::{opt, recognize};
use nom::multi::separated_list0;
use nom::sequence::{delimited, pair, preceded};
use nom::{IResult, Parser};

/// A parsed type path with optional variant
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTypePath {
    /// The full type including module path and generics
    /// e.g., "`core::option::Option`<`bevy_asset::handle::Handle`<`bevy_mesh::mesh::Mesh`>>"
    pub full_type:       String,
    /// The simplified type name with generics but no module paths
    /// e.g., "Option<Handle<Mesh>>"
    pub simplified_type: String,
    /// The variant name if present
    /// e.g., "Some"
    pub variant:         Option<String>,
}

/// Parse an identifier (alphanumeric + underscore, not starting with digit)
fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

/// Parse generic arguments recursively
fn generics(input: &str) -> IResult<&str, &str> {
    recognize(delimited(
        char('<'),
        separated_list0(
            tag(", "),
            alt((
                // Type with generics
                recognize(pair(type_path_inner, opt(generics))),
                // Simple type
                type_path_inner,
            )),
        ),
        char('>'),
    ))
    .parse(input)
}

/// Internal type path parser (needed because we can't reference `type_path` before it's defined)
fn type_path_inner(input: &str) -> IResult<&str, &str> {
    recognize(pair(separated_list0(tag("::"), identifier), opt(generics))).parse(input)
}

/// Parse a complete type path (`module::Type`<Generics>)
fn type_path(input: &str) -> IResult<&str, &str> {
    type_path_inner(input)
}

/// Parse the complete type path with optional variant
fn full_type_path(input: &str) -> IResult<&str, (&str, Option<&str>)> {
    // Special case: for simple Type::Variant (single ::), split at the last ::
    // This handles both "Color::Srgba" and "mod::Type::Variant" correctly
    if !input.contains('<') {
        // No generics - count the :: separators
        let separator_count = input.matches("::").count();

        if separator_count == 1 {
            // Simple case like "Color::Srgba" - assume it's Type::Variant
            if let Some(pos) = input.find("::") {
                let type_part = &input[..pos];
                let variant_part = &input[pos + 2..];
                return Ok(("", (type_part, Some(variant_part))));
            }
        } else if separator_count > 1 {
            // Multiple :: - the last one is likely the variant separator
            // Find the position of the last ::
            if let Some(last_pos) = input.rfind("::") {
                let type_part = &input[..last_pos];
                let variant_part = &input[last_pos + 2..];
                // Check if variant_part looks like a variant (starts with uppercase)
                if variant_part.chars().next().is_some_and(char::is_uppercase) {
                    return Ok(("", (type_part, Some(variant_part))));
                }
            }
        }
    }

    // Fall back to the original parsing for complex cases with generics
    let (input, type_part) = type_path(input)?;
    let (input, variant) = opt(preceded(tag("::"), identifier)).parse(input)?;
    Ok((input, (type_part, variant)))
}

/// Simplify a type by removing module paths but keeping generic structure
fn simplify_type(type_str: &str) -> String {
    // Find where generics start (if any)
    type_str.find('<').map_or_else(
        || {
            // No generics - simplify by taking just the type name without module path
            // "extras_plugin::TestVariantChainEnum" -> "TestVariantChainEnum"
            // "std::collections::HashMap" -> "HashMap"
            // "MyType" -> "MyType"
            type_str.rsplit("::").next().unwrap_or(type_str).to_string()
        },
        |generic_start| {
            let base_type = &type_str[..generic_start];
            let generics_part = &type_str[generic_start..];

            // Get just the type name (last segment before generics)
            let type_name = if base_type.contains("::") {
                base_type.rsplit("::").next().unwrap_or(base_type)
            } else {
                base_type
            };

            // Recursively simplify types within generics
            let simplified_generics = simplify_generics(generics_part);

            format!("{type_name}{simplified_generics}")
        },
    )
}

/// Simplify generic parameters recursively
fn simplify_generics(generics_str: &str) -> String {
    if !generics_str.starts_with('<') || !generics_str.ends_with('>') {
        return generics_str.to_string();
    }

    let inner = &generics_str[1..generics_str.len() - 1];
    let mut result = String::from("<");
    let mut depth = 0;
    let mut current_type = String::new();

    for ch in inner.chars() {
        match ch {
            '<' => {
                depth += 1;
                current_type.push(ch);
            }
            '>' => {
                depth -= 1;
                current_type.push(ch);
            }
            ',' if depth == 0 => {
                // End of a type parameter
                if !result.ends_with('<') {
                    result.push_str(", ");
                }
                result.push_str(&simplify_type(current_type.trim()));
                current_type.clear();
            }
            _ => {
                current_type.push(ch);
            }
        }
    }

    // Handle the last type parameter
    if !current_type.trim().is_empty() {
        if !result.ends_with('<') {
            result.push_str(", ");
        }
        result.push_str(&simplify_type(current_type.trim()));
    }

    result.push('>');
    result
}

/// Parse a complete type path and extract simplified variant name
pub fn parse_type_with_variant(input: &str) -> Result<ParsedTypePath, String> {
    match full_type_path(input) {
        Ok((remaining, (type_part, variant))) => {
            if !remaining.is_empty() {
                return Err(format!(
                    "Unexpected characters after type path: {remaining}"
                ));
            }

            let simplified = simplify_type(type_part);

            Ok(ParsedTypePath {
                full_type:       type_part.to_string(),
                simplified_type: simplified,
                variant:         variant.map(ToString::to_string),
            })
        }
        Err(e) => Err(format!("Failed to parse type path: {e:?}")),
    }
}

/// Extract a simplified variant name from a full type path
/// e.g., "`core::option::Option`<`bevy_asset::handle::Handle`<`bevy_mesh::mesh::Mesh`>>`::Some`"
///    -> "Option<Handle<Mesh>>`::Some`"
pub fn extract_simplified_variant_name(type_path: &str) -> String {
    match parse_type_with_variant(type_path) {
        Ok(parsed) => {
            if let Some(variant) = parsed.variant {
                format!("{}::{variant}", parsed.simplified_type)
            } else {
                parsed.simplified_type
            }
        }
        Err(_) => {
            // Fallback: if parsing fails, try simple extraction
            type_path.rfind("::").map_or_else(
                || type_path.to_string(),
                |pos| {
                    let variant = &type_path[pos + 2..];
                    format!("UnknownType::{variant}")
                },
            )
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_enum_variant() {
        let input = "Color::Srgba";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "Color");
        assert_eq!(result.variant, Some("Srgba".to_string()));
        assert_eq!(extract_simplified_variant_name(input), "Color::Srgba");
    }

    #[test]
    fn test_option_with_simple_type() {
        let input = "core::option::Option<i32>::Some";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "Option<i32>");
        assert_eq!(result.variant, Some("Some".to_string()));
        assert_eq!(extract_simplified_variant_name(input), "Option<i32>::Some");
    }

    #[test]
    fn test_deeply_nested_generics() {
        let input = "core::option::Option<bevy_asset::handle::Handle<bevy_mesh::mesh::Mesh>>::Some";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "Option<Handle<Mesh>>");
        assert_eq!(result.variant, Some("Some".to_string()));
        assert_eq!(
            extract_simplified_variant_name(input),
            "Option<Handle<Mesh>>::Some"
        );
    }

    #[test]
    fn test_multiple_generic_params() {
        let input = "std::collections::HashMap<String, Vec<u32>>::new";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "HashMap<String, Vec<u32>>");
        assert_eq!(result.variant, Some("new".to_string()));
    }

    #[test]
    fn test_module_path_enum_variant() {
        let input = "extras_plugin::TestVariantChainEnum::WithMiddleStruct";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "TestVariantChainEnum");
        assert_eq!(result.variant, Some("WithMiddleStruct".to_string()));
        assert_eq!(
            extract_simplified_variant_name(input),
            "TestVariantChainEnum::WithMiddleStruct"
        );
    }

    #[test]
    fn test_module_path_enum_variant_empty() {
        let input = "extras_plugin::TestVariantChainEnum::Empty";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "TestVariantChainEnum");
        assert_eq!(result.variant, Some("Empty".to_string()));
        assert_eq!(
            extract_simplified_variant_name(input),
            "TestVariantChainEnum::Empty"
        );
    }

    #[test]
    fn test_nested_module_path_enum_variant() {
        let input = "extras_plugin::BottomEnum::VariantA";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "BottomEnum");
        assert_eq!(result.variant, Some("VariantA".to_string()));
        assert_eq!(
            extract_simplified_variant_name(input),
            "BottomEnum::VariantA"
        );
    }
}
