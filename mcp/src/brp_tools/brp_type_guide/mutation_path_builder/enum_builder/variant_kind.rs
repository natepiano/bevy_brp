use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::super::new_types::StructFieldName;
use super::super::new_types::VariantName;
use super::super::type_parser;
use super::variant_signature::VariantSignature;
use crate::brp_tools::brp_type_guide::BrpTypeName;
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// Type-safe enum variant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantKind {
    name:      VariantName,
    signature: VariantSignature,
}

impl VariantKind {
    /// Get the fully qualified variant name (e.g., "`Color::Srgba`")
    pub const fn variant_name(&self) -> &VariantName {
        &self.name
    }

    pub const fn signature(&self) -> &VariantSignature {
        &self.signature
    }

    /// Get just the variant name without the enum prefix (e.g., "Srgba" from "`Color::Srgba`")
    pub fn name(&self) -> &str {
        self.name
            .as_str()
            .rsplit_once("::")
            .map_or_else(|| self.name.as_str(), |(_, name)| name)
    }

    /// Extract variant information from a schema variant
    pub fn from_schema_variant(
        v: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        enum_type: &BrpTypeName,
    ) -> Option<Self> {
        // Handle Unit variants which show up as simple strings
        if let Some(variant_str) = v.as_str() {
            // For simple string variants, we need to construct the full variant name
            // Extract just the type name without module path
            let type_name = enum_type
                .as_str()
                .rsplit("::")
                .next()
                .unwrap_or(enum_type.as_str());

            let qualified_name = format!("{type_name}::{variant_str}");
            return Some(Self {
                name:      VariantName::from(qualified_name),
                signature: VariantSignature::Unit,
            });
        }

        // Extract the fully qualified variant name
        let variant_name = extract_variant_qualified_name(v)?;

        // Check what type of variant this is
        if let Some(signature) = extract_tuple_variant_signature(v, registry) {
            return Some(Self {
                name: variant_name,
                signature,
            });
        }

        if let Some(signature) = extract_struct_variant_signature(v, registry) {
            return Some(Self {
                name: variant_name,
                signature,
            });
        }

        // Unit variant (no fields)
        Some(Self {
            name:      variant_name,
            signature: VariantSignature::Unit,
        })
    }
}

/// Extract tuple variant signature from schema if it matches tuple pattern
fn extract_tuple_variant_signature(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Option<VariantSignature> {
    let prefix_items = v.get_field(SchemaField::PrefixItems)?;
    let prefix_array = prefix_items.as_array()?;

    let tuple_types: Vec<BrpTypeName> = prefix_array
        .iter()
        .filter_map(Value::extract_field_type)
        .collect();

    Some(VariantSignature::Tuple(tuple_types))
}

/// Extract struct variant signature from schema if it matches struct pattern
fn extract_struct_variant_signature(
    v: &Value,
    _registry: &HashMap<BrpTypeName, Value>,
) -> Option<VariantSignature> {
    let properties = v.get_field(SchemaField::Properties)?;
    let props_map = properties.as_object()?;

    let struct_fields: Vec<(StructFieldName, BrpTypeName)> = props_map
        .iter()
        .filter_map(|(field_name, field_schema)| {
            field_schema
                .extract_field_type()
                .map(|type_name| (StructFieldName::from(field_name.clone()), type_name))
        })
        .collect();

    if struct_fields.is_empty() {
        return None;
    }

    Some(VariantSignature::Struct(struct_fields))
}

/// Extract the fully qualified variant name from schema (e.g., "`Color::Srgba`")
fn extract_variant_qualified_name(v: &Value) -> Option<VariantName> {
    // First try to get the type path for the full qualified name
    if let Some(type_path) = v.get_field(SchemaField::TypePath).and_then(Value::as_str) {
        // Use the new parser to handle nested generics properly
        let simplified_name = type_parser::extract_simplified_variant_name(type_path);
        return Some(VariantName::from(simplified_name));
    }

    // Fallback to just the variant name if we can't parse it
    v.get_field(SchemaField::ShortPath)
        .and_then(Value::as_str)
        .map(|s| VariantName::from(s.to_string()))
}
