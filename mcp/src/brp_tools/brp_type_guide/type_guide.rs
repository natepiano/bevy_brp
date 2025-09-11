//! This is the main response structure use to convey type information
//! to the caller
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use super::constants::RecursionDepth;
use super::mutation_path_builder::{
    EnumVariantInfo, MutationPath, MutationPathBuilder, MutationPathInternal, PathKind,
    RecursionContext, TypeKind,
};
use super::response_types::{BrpSupportedOperation, BrpTypeName, ReflectTrait, SchemaInfo};
use crate::json_object::JsonObjectAccess;
use crate::json_schema::SchemaField;

/// this is all of the information we provide about a type
/// we serialize this to our output - and we call it `type_guide`
/// because that's what's on the tin
#[derive(Debug, Clone, Serialize)]
pub struct TypeGuide {
    /// Fully-qualified type name
    pub type_name: BrpTypeName,
    /// Whether the type is registered in the Bevy registry
    pub in_registry: bool,
    /// Whether the type has the Serialize trait
    pub has_serialize: bool,
    /// Whether the type has the Deserialize trait
    pub has_deserialize: bool,
    /// List of BRP operations supported by this type
    pub supported_operations: Vec<BrpSupportedOperation>,
    /// Mutation paths available for this type - using same format as V1
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub mutation_paths: HashMap<String, MutationPath>,
    /// Example values for spawn/insert operations (currently empty to match V1)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub example_values: HashMap<String, Value>,
    /// Example format for spawn/insert operations when supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spawn_format: Option<Value>,
    /// Information about enum variants if this is an enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_info: Option<Vec<EnumVariantInfo>>,
    /// Schema information from the registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_info: Option<SchemaInfo>,
    /// Type information for direct fields (struct fields only, one level deep)
    /// Error message if discovery failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TypeGuide {
    /// Builder method to create ``TypeGuide`` from schema data
    pub fn from_registry_schema(
        brp_type_name: BrpTypeName,
        registry_schema: &Value,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Self {
        // Extract type kind
        let type_kind = TypeKind::from_schema(registry_schema, &brp_type_name);

        // Extract reflection traits
        let reflect_types = Self::extract_reflect_types(registry_schema);

        // Check for serialization traits
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        // Get supported operations
        let supported_operations = Self::get_supported_operations(&reflect_types);

        // Build mutation paths to determine actual mutation capability
        let mutation_paths_vec =
            Self::build_mutation_paths(&brp_type_name, registry_schema, Arc::clone(&registry));
        tracing::error!(
            "AFTER build_mutation_paths: {} returned {} paths",
            brp_type_name,
            mutation_paths_vec.len()
        );

        let mutation_paths = Self::convert_mutation_paths(&mutation_paths_vec, &registry);
        tracing::error!(
            "AFTER convert_mutation_paths: {} converted {} paths",
            brp_type_name,
            mutation_paths.len()
        );

        // Add Mutate operation if any paths are actually mutatable
        let mut supported_operations = supported_operations;
        tracing::error!("BEFORE has_mutatable_paths check: {}", brp_type_name);

        if Self::has_mutatable_paths(&mutation_paths) {
            supported_operations.push(BrpSupportedOperation::Mutate);
        }
        tracing::error!("AFTER has_mutatable_paths check: {}", brp_type_name);

        // Build spawn format from root path mutation example - ONLY for types that support
        // spawn/insert
        tracing::error!("BEFORE extract_spawn_format_from_paths: {}", brp_type_name);
        let spawn_format = if supported_operations.contains(&BrpSupportedOperation::Spawn)
            || supported_operations.contains(&BrpSupportedOperation::Insert)
        {
            Self::extract_spawn_format_from_paths(&mutation_paths)
        } else {
            None
        };
        tracing::error!("AFTER extract_spawn_format_from_paths: {}", brp_type_name);

        // Build enum info if it's an enum
        tracing::error!("BEFORE extract_enum_info: {}", brp_type_name);
        let enum_info = if type_kind == TypeKind::Enum {
            Self::extract_enum_info(registry_schema, &registry)
        } else {
            None
        };
        tracing::error!("AFTER extract_enum_info: {}", brp_type_name);

        // Extract schema info from registry
        tracing::error!("BEFORE extract_schema_info: {}", brp_type_name);
        let schema_info = Self::extract_schema_info(registry_schema);
        tracing::error!("AFTER extract_schema_info: {}", brp_type_name);

        Self {
            type_name: brp_type_name,
            in_registry: true,
            has_serialize,
            has_deserialize,
            supported_operations,
            mutation_paths,
            example_values: HashMap::new(), // V1 always has this empty
            spawn_format,
            enum_info,
            schema_info,
            error: None,
        }
    }

    /// Builder method to create ``TypeGuide`` for type not found in registry
    pub fn not_found_in_registry(type_name: BrpTypeName, error_msg: String) -> Self {
        Self {
            type_name,
            in_registry: false,
            has_serialize: false,
            has_deserialize: false,
            supported_operations: Vec::new(),
            mutation_paths: HashMap::new(),
            example_values: HashMap::new(),
            spawn_format: None,
            enum_info: None,
            schema_info: None,
            error: Some(error_msg),
        }
    }

    // Private helper methods

    /// Extract spawn format from root mutation path
    /// Uses the root path `""` example as the spawn format for consistency
    /// Should only be called for types that support spawn/insert operations
    fn extract_spawn_format_from_paths(
        mutation_paths: &HashMap<String, MutationPath>,
    ) -> Option<Value> {
        mutation_paths.get("").and_then(|root_path| {
            // Handle both the new `example` field and the legacy `examples` array
            root_path.example.as_ref().map_or_else(
                || {
                    root_path
                        .examples
                        .first()
                        .map(|example_group| example_group.example.clone())
                },
                |example| Some(example.clone()),
            )
        })
    }

    /// Check if any mutation paths are mutatable (fully or partially)
    /// This determines if the type supports the Mutate operation
    fn has_mutatable_paths(mutation_paths: &HashMap<String, MutationPath>) -> bool {
        use super::mutation_path_builder::MutationStatus;

        mutation_paths.values().any(|path| {
            matches!(
                path.path_info.mutation_status,
                MutationStatus::Mutatable | MutationStatus::PartiallyMutatable
            )
        })
    }

    /// Build mutation paths for a type using the trait system
    fn build_mutation_paths(
        brp_type_name: &BrpTypeName,
        registry_schema: &Value,
        registry: Arc<HashMap<BrpTypeName, Value>>,
    ) -> Vec<MutationPathInternal> {
        tracing::error!(">>> TOP LEVEL TYPE START: {}", brp_type_name);

        let type_kind = TypeKind::from_schema(registry_schema, brp_type_name);

        // Create root context for the new trait system
        let path_kind = PathKind::new_root_value(brp_type_name.clone());
        let ctx = RecursionContext::new(path_kind, Arc::clone(&registry));

        // Use the new trait dispatch system
        let result = type_kind
            .build_paths(&ctx, RecursionDepth::ZERO)
            .unwrap_or_else(|_| Vec::new());

        tracing::error!(
            "<<< TOP LEVEL TYPE COMPLETE: {} (returned {} paths)",
            brp_type_name,
            result.len()
        );
        result
    }

    /// Convert `Vec<MutationPath>` to `HashMap<String, MutationPathInfo>`
    fn convert_mutation_paths(
        paths: &[MutationPathInternal],
        registry: &HashMap<BrpTypeName, Value>,
    ) -> HashMap<String, MutationPath> {
        let mut result = HashMap::new();

        for path in paths {
            // Create MutationPathInfo from MutationPath
            let path_info = MutationPath::from_mutation_path_internal(path, registry);

            // Keep empty path as empty for root mutations
            // BRP expects empty string for root replacements, not "."
            let key = path.path.clone();

            result.insert(key, path_info);
        }

        result
    }

    /// Extract enum information from schema
    fn extract_enum_info(
        registry_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Option<Vec<EnumVariantInfo>> {
        let one_of = registry_schema
            .get_field(SchemaField::OneOf)
            .and_then(Value::as_array)?;

        let variants: Vec<EnumVariantInfo> = one_of
            .iter()
            .filter_map(|v| EnumVariantInfo::from_schema_variant(v, registry, 0))
            .collect();

        Some(variants)
    }

    /// Extract reflect types from a registry schema
    fn extract_reflect_types(registry_schema: &Value) -> Vec<ReflectTrait> {
        registry_schema
            .get_field(SchemaField::ReflectTypes)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| s.parse::<ReflectTrait>().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Extract schema information from registry schema
    fn extract_schema_info(registry_schema: &Value) -> Option<SchemaInfo> {
        let type_kind = registry_schema
            .get_field(SchemaField::Kind)
            .and_then(Value::as_str)
            .and_then(|s| TypeKind::from_str(s).ok());

        let properties = registry_schema.get_field(SchemaField::Properties).cloned();

        let required = registry_schema
            .get_field(SchemaField::Required)
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect()
            });

        let module_path = registry_schema
            .get_field(SchemaField::ModulePath)
            .and_then(Value::as_str)
            .map(String::from);

        let crate_name = registry_schema
            .get_field(SchemaField::CrateName)
            .and_then(Value::as_str)
            .map(String::from);

        // Only return SchemaInfo if we have at least some information
        if type_kind.is_some()
            || properties.is_some()
            || required.is_some()
            || module_path.is_some()
            || crate_name.is_some()
        {
            Some(SchemaInfo {
                type_kind,
                properties,
                required,
                module_path,
                crate_name,
            })
        } else {
            None
        }
    }

    /// Get supported BRP operations based on reflection traits
    fn get_supported_operations(reflect_types: &[ReflectTrait]) -> Vec<BrpSupportedOperation> {
        let mut operations = vec![BrpSupportedOperation::Query];

        let has_component = reflect_types.contains(&ReflectTrait::Component);
        let has_resource = reflect_types.contains(&ReflectTrait::Resource);
        let has_serialize = reflect_types.contains(&ReflectTrait::Serialize);
        let has_deserialize = reflect_types.contains(&ReflectTrait::Deserialize);

        if has_component {
            operations.push(BrpSupportedOperation::Get);
            if has_serialize && has_deserialize {
                operations.push(BrpSupportedOperation::Spawn);
                operations.push(BrpSupportedOperation::Insert);
            }
        }

        if has_resource && has_serialize && has_deserialize {
            // Resources support Insert but mutation capability is determined dynamically
            // based on actual mutation path analysis in from_schema()
            operations.push(BrpSupportedOperation::Insert);
        }

        operations
    }
}
