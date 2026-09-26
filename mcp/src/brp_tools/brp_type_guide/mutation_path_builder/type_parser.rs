//! Parser for Rust type paths with support for nested generics
//!
//! This module uses nom to properly parse type paths like:
//! - `Color::Srgba`
//! - `Option<T>::Some`
//! - `Option<Handle<Mesh>>::Some`
//! - `core::option::Option<bevy_asset::handle::Handle<bevy_mesh::mesh::Mesh>>::Some`
//!
//! Generic arguments can also be tuples, arrays, slices, and references:
//! - `core::option::Option<(bevy_ecs::entity::Entity, u8)>::Some`
//! - `core::option::Option<[f32; 4]>::Some`
//! - `core::option::Option<&'static str>::Some`

use nom::IResult;
use nom::Parser;
use nom::branch;
use nom::bytes;
use nom::character;
use nom::combinator;
use nom::multi;
use nom::sequence;

/// A parsed type path with optional variant
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTypePath {
    /// The full type including module path and generics
    /// e.g., "`core::option::Option`<`bevy_asset::handle::Handle`<`bevy_mesh::mesh::Mesh`>>"
    full_type:       String,
    /// The simplified type name with generics but no module paths
    /// e.g., "`Option<Handle<Mesh>>`"
    simplified_type: String,
    /// The variant name if present
    /// e.g., "Some"
    variant:         Option<String>,
}

/// Parse an identifier (alphanumeric + underscore, not starting with digit)
fn identifier(input: &str) -> IResult<&str, &str> {
    bytes::complete::take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}

/// Parse generic arguments recursively
fn generics(input: &str) -> IResult<&str, &str> {
    combinator::recognize(sequence::delimited(
        character::complete::char('<'),
        multi::separated_list0(bytes::complete::tag(", "), type_arg),
        character::complete::char('>'),
    ))
    .parse(input)
}

/// Parse one type argument: a tuple, array, slice, reference, or type path
///
/// `type_path_inner` matches the empty string, so it has to come last.
fn type_arg(input: &str) -> IResult<&str, &str> {
    branch::alt((tuple_type, array_type, reference_type, type_path_inner)).parse(input)
}

/// Parse a tuple type such as `()`, `(A,)`, or `(A, B)`
fn tuple_type(input: &str) -> IResult<&str, &str> {
    combinator::recognize(sequence::delimited(
        character::complete::char('('),
        sequence::pair(
            multi::separated_list0(bytes::complete::tag(", "), type_arg),
            combinator::opt(character::complete::char(',')),
        ),
        character::complete::char(')'),
    ))
    .parse(input)
}

/// Parse an array type such as `[f32; 4]` or a slice type such as `[u8]`
fn array_type(input: &str) -> IResult<&str, &str> {
    combinator::recognize(sequence::delimited(
        character::complete::char('['),
        sequence::pair(
            type_arg,
            combinator::opt(sequence::preceded(
                bytes::complete::tag("; "),
                character::complete::digit1,
            )),
        ),
        character::complete::char(']'),
    ))
    .parse(input)
}

/// Parse a reference prefix: `&`, an optional lifetime such as `'static `, and an optional `mut `
fn reference_prefix(input: &str) -> IResult<&str, &str> {
    combinator::recognize((
        character::complete::char('&'),
        combinator::opt(sequence::terminated(
            sequence::preceded(character::complete::char('\''), identifier),
            character::complete::char(' '),
        )),
        combinator::opt(bytes::complete::tag("mut ")),
    ))
    .parse(input)
}

/// Parse a reference type such as `&'static str` or `&mut T`
fn reference_type(input: &str) -> IResult<&str, &str> {
    combinator::recognize(sequence::pair(reference_prefix, type_arg)).parse(input)
}

/// Internal type path parser (needed because we can't reference `type_path` before it's defined)
fn type_path_inner(input: &str) -> IResult<&str, &str> {
    combinator::recognize(sequence::pair(
        multi::separated_list0(bytes::complete::tag("::"), identifier),
        combinator::opt(generics),
    ))
    .parse(input)
}

/// Parse a complete type path (`module::Type<Generics>`)
fn type_path(input: &str) -> IResult<&str, &str> { type_path_inner(input) }

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
    let (input, variant) =
        combinator::opt(sequence::preceded(bytes::complete::tag("::"), identifier)).parse(input)?;
    Ok((input, (type_part, variant)))
}

/// Simplify a type by removing module paths but keeping generic, tuple, array, and reference
/// structure
fn simplify_type(type_str: &str) -> String {
    // References keep their `&`, lifetime, and `mut` prefix
    // "&'static alloc::string::String" -> "&'static String"
    if let Ok((referent, prefix)) = reference_prefix(type_str) {
        return format!("{prefix}{}", simplify_type(referent));
    }

    // Tuples simplify each element and keep a trailing comma
    // "(bevy_ecs::entity::Entity, u8)" -> "(Entity, u8)", "(bevy_ecs::entity::Entity,)" ->
    // "(Entity,)"
    if let Some(elements) = type_str
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let trailing_comma = if elements.trim_end().ends_with(',') {
            ","
        } else {
            ""
        };
        return format!("({}{trailing_comma})", simplify_list(elements));
    }

    // Arrays simplify the element and keep the length, slices have no length
    // "[bevy_math::Vec2; 4]" -> "[Vec2; 4]", "[bevy_math::Vec2]" -> "[Vec2]"
    if let Some(contents) = type_str
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
    {
        return match split_top_level(contents, ';').as_slice() {
            [element, length] => format!("[{}; {length}]", simplify_type(element)),
            _ => format!("[{}]", simplify_type(contents)),
        };
    }

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
    generics_str
        .strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .map_or_else(
            || generics_str.to_string(),
            |arguments| format!("<{}>", simplify_list(arguments)),
        )
}

/// Simplify each type in a comma-separated list and rejoin them with ", "
fn simplify_list(list: &str) -> String {
    split_top_level(list, ',')
        .into_iter()
        .map(simplify_type)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Split `list` at each `separator` outside `<>`, `()`, and `[]` nesting
///
/// Each piece is trimmed, and the empty final piece a trailing separator leaves is dropped, so
/// `"A,"` and `"A"` both split to `["A"]` and `""` splits to `[]`.
fn split_top_level(list: &str, separator: char) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut depth = 0_usize;
    let mut piece_start = 0;

    for (index, ch) in list.char_indices() {
        match ch {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            _ if ch == separator && depth == 0 => {
                pieces.push(list[piece_start..index].trim());
                piece_start = index + ch.len_utf8();
            },
            _ => {},
        }
    }

    let last_piece = list[piece_start..].trim();
    if !last_piece.is_empty() {
        pieces.push(last_piece);
    }
    pieces
}

/// Parse a complete type path and extract simplified variant name
fn parse_type_with_variant(input: &str) -> Result<ParsedTypePath, String> {
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
        },
        Err(e) => Err(format!("Failed to parse type path: {e:?}")),
    }
}

/// Extract a simplified variant name from a full type path
/// e.g., "`core::option::Option`<`bevy_asset::handle::Handle`<`bevy_mesh::mesh::Mesh`>>`::Some`"
///    -> "`Option<Handle<Mesh>>::Some`"
pub(super) fn extract_simplified_variant_name(type_path: &str) -> String {
    match parse_type_with_variant(type_path) {
        Ok(parsed) => {
            if let Some(variant) = parsed.variant {
                format!("{}::{variant}", parsed.simplified_type)
            } else {
                parsed.simplified_type
            }
        },
        Err(_) => {
            // Fallback: if parsing fails, try simple extraction
            type_path.rfind("::").map_or_else(
                || type_path.to_string(),
                |pos| {
                    let variant = &type_path[pos + 2..];
                    format!("UnknownType::{variant}")
                },
            )
        },
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests should panic on unexpected values"
)]
mod tests {
    use super::extract_simplified_variant_name;
    use super::parse_type_with_variant;

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

    #[test]
    fn test_option_with_tuple_of_paths() {
        let input = "core::option::Option<(bevy_ecs::entity::Entity, bevy_input_focus::gained_and_lost::FocusCause)>::Some";
        let result = parse_type_with_variant(input).unwrap();
        assert_eq!(result.simplified_type, "Option<(Entity, FocusCause)>");
        assert_eq!(result.variant, Some("Some".to_string()));
        assert_eq!(
            extract_simplified_variant_name(input),
            "Option<(Entity, FocusCause)>::Some"
        );
    }

    #[test]
    fn test_option_with_tuple_of_primitives() {
        assert_eq!(
            extract_simplified_variant_name("core::option::Option<(u8, u8)>::Some"),
            "Option<(u8, u8)>::Some"
        );
    }

    #[test]
    fn test_option_with_tuple_of_primitive_and_path() {
        assert_eq!(
            extract_simplified_variant_name(
                "core::option::Option<(bool, bevy_math::rects::rect::Rect)>::Some"
            ),
            "Option<(bool, Rect)>::Some"
        );
    }

    #[test]
    fn test_option_with_single_element_tuple() {
        assert_eq!(
            extract_simplified_variant_name(
                "core::option::Option<(bevy_ecs::entity::Entity,)>::Some"
            ),
            "Option<(Entity,)>::Some"
        );
    }

    #[test]
    fn test_option_with_unit() {
        assert_eq!(
            extract_simplified_variant_name("core::option::Option<()>::None"),
            "Option<()>::None"
        );
    }

    #[test]
    fn test_option_with_array() {
        assert_eq!(
            extract_simplified_variant_name("core::option::Option<[f32; 4]>::Some"),
            "Option<[f32; 4]>::Some"
        );
    }

    #[test]
    fn test_option_with_slice_of_arrays() {
        assert_eq!(
            extract_simplified_variant_name(
                "core::option::Option<&'static [[bevy_math::Vec2; 2]]>::Some"
            ),
            "Option<&'static [[Vec2; 2]]>::Some"
        );
    }

    #[test]
    fn test_option_with_static_reference() {
        assert_eq!(
            extract_simplified_variant_name("core::option::Option<&'static str>::Some"),
            "Option<&'static str>::Some"
        );
    }

    #[test]
    fn test_option_with_mutable_reference() {
        assert_eq!(
            extract_simplified_variant_name(
                "core::option::Option<&mut bevy_ecs::entity::Entity>::Some"
            ),
            "Option<&mut Entity>::Some"
        );
    }

    #[test]
    fn test_option_with_vec_of_tuple_containing_array() {
        assert_eq!(
            extract_simplified_variant_name(
                "core::option::Option<alloc::vec::Vec<(bevy_ecs::entity::Entity, [u8; 2])>>::Some"
            ),
            "Option<Vec<(Entity, [u8; 2])>>::Some"
        );
    }

    #[test]
    fn test_result_with_unit_and_path() {
        assert_eq!(
            extract_simplified_variant_name("core::result::Result<(), alloc::string::String>::Ok"),
            "Result<(), String>::Ok"
        );
    }
}
