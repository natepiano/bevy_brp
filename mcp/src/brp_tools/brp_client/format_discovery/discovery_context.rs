//! Core implementation of `DiscoveryContext` and unified type system
//!
//! This module contains the main logic for the `DiscoveryContext` struct and the
//! unified type system for format discovery. Single coherent schema replacing
//! fragmented type conversions. Contains all discoverable type information in
//! one place to prevent data loss.

use std::collections::HashMap;

use serde_json::Value;
use tracing::debug;

use super::types::{Correction, CorrectionInfo, Operation};
use crate::brp_tools::Port;
use crate::brp_tools::brp_type_schema::{
    BRP_FORMAT_KNOWLEDGE, BrpTypeName, EnumVariantInfo, MathComponent, MutationPath, TypeInfo,
    TypeSchemaEngine,
};
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

pub struct DiscoveryContext {
    /// Complete original parameters from the BRP request
    pub original_params: Value,

    /// Type information keyed by `BrpTypeName`
    pub type_registry: HashMap<BrpTypeName, TypeInfo>,

    /// Operation type (`SpawnInsert` or Mutate)
    pub operation: Operation,

    /// For mutations: the specific path being mutated
    /// For spawn/insert: None (use `BrpTypeName` as path)
    pub mutation_path: Option<String>,
}

impl DiscoveryContext {
    /// Create a new `DiscoveryContext` from BRP method parameters
    /// Uses `TypeSchemaEngine` as single source of truth for type information
    pub async fn new(method: BrpMethod, port: Port, params: &Value) -> Result<Self> {
        // Determine operation type from method
        let operation = Operation::try_from(method)?;

        // Extract type names and mutation path
        let (type_names, mutation_path) = Self::extract_type_names_and_path(method, params)?;

        debug!("using TypeSchemaEngine for type information (single registry fetch)");

        // Get TypeInfo from TypeSchemaEngine (single registry fetch)
        let engine = TypeSchemaEngine::new(port).await?;
        let response = engine.generate_response(&type_names);

        // Build type_registry from TypeInfo
        let mut type_registry = HashMap::new();

        for type_name_str in type_names {
            let type_name: BrpTypeName = type_name_str.into();
            let type_info = response.type_info.get(&type_name).ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "Type '{}' not found in registry. Verify the type name is correct and the Bevy app is running with this component registered.",
                    type_name.as_str()
                ))
            })?;
            type_registry.insert(type_name, type_info.clone());
        }

        Ok(Self {
            original_params: params.clone(),
            type_registry,
            operation,
            mutation_path,
        })
    }

    /// Get all type names as an iterator
    pub fn type_names(&self) -> impl Iterator<Item = &BrpTypeName> {
        self.type_registry.keys()
    }

    /// Extract value for a specific type from original parameters
    pub fn extract_value_for_type(&self, type_name: &BrpTypeName) -> Option<Value> {
        match self.operation {
            Operation::SpawnInsert { .. } => {
                // For spawn/insert, extract from components using type name as key
                self.original_params
                    .get("components")
                    .and_then(|c| c.get(type_name.as_str()))
                    .cloned()
            }
            Operation::Mutate { .. } => {
                // For mutations, return the value field
                self.original_params.get("value").cloned()
            }
        }
    }

    /// Extract type names and mutation path from method parameters
    fn extract_type_names_and_path(
        method: BrpMethod,
        params: &Value,
    ) -> Result<(Vec<String>, Option<String>)> {
        let mut type_names = Vec::new();
        let mut mutation_path = None;

        match method {
            BrpMethod::BevySpawn | BrpMethod::BevyInsert => {
                // Validate components field exists and is an object
                let components = params
                    .get("components")
                    .ok_or_else(|| {
                        Error::InvalidArgument("Missing 'components' field".to_string())
                    })?
                    .as_object()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'components' field must be an object".to_string())
                    })?;

                for type_name in components.keys() {
                    // Validate type name is a valid string
                    if type_name.is_empty() {
                        return Err(Error::InvalidArgument(
                            "Empty type name in components".to_string(),
                        )
                        .into());
                    }
                    type_names.push(type_name.clone());
                }

                if type_names.is_empty() {
                    return Err(Error::InvalidArgument("No components provided".to_string()).into());
                }
            }
            BrpMethod::BevyMutateComponent => {
                let component = params
                    .get("component")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'component' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'component' field must be a string".to_string())
                    })?;

                if component.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty component type name".to_string()).into(),
                    );
                }

                mutation_path = params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                type_names.push(component.to_string());
            }
            BrpMethod::BevyInsertResource | BrpMethod::BevyMutateResource => {
                let resource = params
                    .get("resource")
                    .ok_or_else(|| Error::InvalidArgument("Missing 'resource' field".to_string()))?
                    .as_str()
                    .ok_or_else(|| {
                        Error::InvalidArgument("'resource' field must be a string".to_string())
                    })?;

                if resource.is_empty() {
                    return Err(
                        Error::InvalidArgument("Empty resource type name".to_string()).into(),
                    );
                }

                if method == BrpMethod::BevyMutateResource {
                    mutation_path = params
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                }

                type_names.push(resource.to_string());
            }
            _ => {
                return Err(Error::InvalidArgument(format!(
                    "Method {method:?} does not support type extraction"
                ))
                .into());
            }
        }

        Ok((type_names, mutation_path))
    }

    // Helper methods for working with types

    /// Get type info for a specific type
    pub fn get_type_info(&self, type_name: &BrpTypeName) -> Option<&TypeInfo> {
        self.type_registry.get(type_name)
    }

    /// Check if a type is registered in the Bevy registry
    pub fn in_registry(&self, type_name: &BrpTypeName) -> bool {
        self.get_type_info(type_name)
            .is_some_and(|ti| ti.in_registry)
    }

    /// Check if a type is BRP compatible (has both Serialize and Deserialize traits)
    pub fn is_brp_compatible(&self, type_name: &BrpTypeName) -> bool {
        self.get_type_info(type_name)
            .is_some_and(|ti| ti.has_serialize && ti.has_deserialize)
    }

    /// Get enum information for a type if it's an enum
    pub fn enum_info(&self, type_name: &BrpTypeName) -> Option<&Vec<EnumVariantInfo>> {
        self.get_type_info(type_name)
            .and_then(|ti| ti.enum_info.as_ref())
    }

    /// Get mutation paths for a type
    pub fn mutation_paths(
        &self,
        type_name: &BrpTypeName,
    ) -> Option<&HashMap<String, MutationPath>> {
        self.get_type_info(type_name).map(|ti| &ti.mutation_paths)
    }

    /// Check if a type is a math type using BRP format knowledge
    fn is_math_type(&self, type_name: &BrpTypeName) -> bool {
        self.get_type_info(type_name)
            .is_some_and(TypeInfo::is_math_type)
    }

    /// Get example for a specific operation and type
    pub fn get_example_for_operation(
        &self,
        type_name: &BrpTypeName,
        operation: Operation,
    ) -> Option<&Value> {
        self.get_type_info(type_name)
            .and_then(|ti| match operation {
                Operation::SpawnInsert { .. } => ti.spawn_format.as_ref(),
                Operation::Mutate { .. } => None,
            })
    }

    /// Convert this type info to a `Correction`
    pub fn to_correction(&self, type_name: &BrpTypeName) -> Correction {
        let Some(type_info) = self.get_type_info(type_name) else {
            return Correction::Uncorrectable {
                type_name: type_name.clone(),
                reason:    format!("Type '{}' not found in registry", type_name.as_str()),
            };
        };

        // Check if this is an enum with variants - provide guidance only
        if let Some(enum_info) = &type_info.enum_info {
            return Self::handle_enum_type(type_name, enum_info);
        }

        // Get and validate the original value
        let original_value = match self.extract_and_validate_value(type_name) {
            Ok(value) => value,
            Err(correction) => return correction,
        };

        tracing::debug!(
            "Attempting to transform original value for type '{}'",
            type_name.as_str()
        );

        // Transform the value based on context
        let transformed_value = self.get_transformed_value(type_name, type_info, &original_value);

        // Build the final correction
        self.build_correction_result(type_name, original_value, transformed_value)
    }

    /// Handle enum types by providing variant guidance
    fn handle_enum_type(type_name: &BrpTypeName, enum_info: &[EnumVariantInfo]) -> Correction {
        let variant_names: Vec<String> = enum_info.iter().map(|v| v.variant_name.clone()).collect();

        let example_variant = variant_names.first().map_or("VariantName", String::as_str);

        let reason = format!(
            "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}. Use one of these values directly (e.g., \"{}\")",
            type_name
                .as_str()
                .split("::")
                .last()
                .unwrap_or(type_name.as_str()),
            variant_names.join(", "),
            example_variant
        );

        Correction::Uncorrectable {
            type_name: type_name.clone(),
            reason,
        }
    }

    /// Extract and validate the original value from the request
    fn extract_and_validate_value(
        &self,
        type_name: &BrpTypeName,
    ) -> std::result::Result<Value, Correction> {
        self.extract_value_for_type(type_name).map_or_else(
            || {
                // No value was provided in the request
                debug!(
                    "No value provided for type '{}' in request",
                    type_name.as_str()
                );

                // Get an example of the expected format
                let example = self
                    .get_example_for_operation(type_name, self.operation)
                    .map_or_else(
                        || "correct format (no example available)".to_string(),
                        |ex| {
                            serde_json::to_string(ex).unwrap_or_else(|_| "valid format".to_string())
                        },
                    );

                Err(Correction::Uncorrectable {
                    type_name: type_name.clone(),
                    reason:    format!(
                        "No value provided for type '{}'. Expected format: {}",
                        type_name.as_str(),
                        example
                    ),
                })
            },
            |val| {
                debug!(
                    "Extracted value for type '{}': {}",
                    type_name.as_str(),
                    serde_json::to_string(&val).unwrap_or_else(|_| "invalid json".to_string())
                );
                Ok(val)
            },
        )
    }

    /// Get the transformed value based on mutation path context
    fn get_transformed_value(
        &self,
        type_name: &BrpTypeName,
        type_info: &TypeInfo,
        original_value: &Value,
    ) -> Option<Value> {
        self.mutation_path.as_ref().map_or_else(
            || {
                // No mutation path - use self transformation
                debug!("No mutation path, using type's own transformation");
                self.transform_value(type_name, original_value)
            },
            |path| {
                let field_name = path.trim_start_matches('.');
                type_info.field_type_infos.get(field_name).map_or_else(
                    || {
                        debug!(
                            "Field '{}' not found in field_type_infos, using parent transformation",
                            field_name
                        );
                        self.transform_value(type_name, original_value)
                    },
                    |field_type_info| {
                        debug!(
                            "Using field '{}' TypeInfo '{}' for transformation",
                            field_name, field_type_info.type_name
                        );
                        // For now, use the type's transform on the original value
                        self.transform_value(type_name, original_value)
                    },
                )
            },
        )
    }

    /// Build the final correction result based on transformation success
    fn build_correction_result(
        &self,
        type_name: &BrpTypeName,
        original_value: Value,
        transformed_value: Option<Value>,
    ) -> Correction {
        transformed_value.map_or_else(
            || {
                debug!(
                    "Could not transform value for type '{}' - providing guidance instead",
                    type_name.as_str()
                );

                // Cannot transform input - provide guidance with examples
                let reason = self.get_example_for_operation(type_name, Operation::SpawnInsert {
                    parameter_name: ParameterName::Components,
                }).map_or_else(|| format!(
                        "Cannot transform input for type '{}'. Type discovered but no format example available.",
                        type_name.as_str()
                    ), |spawn_example| format!(
                        "Cannot transform input for type '{}'. Use this format: {}",
                        type_name.as_str(),
                        serde_json::to_string(spawn_example)
                            .unwrap_or_else(|_| "correct format".to_string())
                    ));

                Correction::Uncorrectable {
                    type_name: type_name.clone(),
                    reason,
                }
            },
            |transformed_value| {
                debug!(
                    "Successfully transformed value for type '{}'",
                    type_name.as_str()
                );
                // We can transform the input - return Corrected with actual transformation
                let correction_info = CorrectionInfo {
                    corrected_value: transformed_value,
                    hint: format!(
                        "Transformed {} format for type '{}'",
                        if original_value.is_object() {
                            "object"
                        } else {
                            "value"
                        },
                        type_name.as_str()
                    ),
                    type_name: type_name.clone(),
                    original_value,
                };

                Correction::Candidate { correction_info }
            },
        )
    }

    /// Helper method to transform a value with consistent logging
    fn transform_with_logging<F>(
        &self,
        type_name: &BrpTypeName,
        value: &Value,
        transform_fn: F,
        transform_type: &str,
    ) -> Option<Value>
    where
        F: FnOnce(&Self, &Value) -> Option<Value>,
    {
        let result = transform_fn(self, value);
        if result.is_none() {
            debug!(
                "{} transformation failed for type '{}'",
                transform_type,
                type_name.as_str()
            );
        }
        result
    }

    /// Transform an incorrect value to the correct format
    pub fn transform_value(&self, type_name: &BrpTypeName, value: &Value) -> Option<Value> {
        use crate::brp_tools::brp_type_schema::TypeKind;

        let Some(type_info) = self.get_type_info(type_name) else {
            debug!(
                "Cannot transform: type '{}' not found in registry",
                type_name.as_str()
            );
            return None;
        };

        let type_kind = type_info
            .schema_info
            .as_ref()
            .and_then(|s| s.type_kind.clone())
            .unwrap_or_else(|| {
                if type_info.enum_info.is_some() {
                    TypeKind::Enum
                } else {
                    TypeKind::Struct
                }
            });

        debug!(
            "Attempting to transform value for type '{}' (kind: {:?})",
            type_name.as_str(),
            type_kind
        );

        match type_kind {
            TypeKind::Enum => self.transform_with_logging(
                type_name,
                value,
                |ctx, val| ctx.transform_enum_value(type_name, val),
                "Enum",
            ),
            TypeKind::Struct => {
                if self.is_math_type(type_name) {
                    Self::transform_math_value(type_name, value)
                } else {
                    Self::transform_struct_value(type_name, value)
                }
            }
            _ => {
                debug!(
                    "No transformation available for type_kind={:?} (type='{}')",
                    type_kind,
                    type_name.as_str()
                );
                None
            }
        }
    }

    /// Transform math type values (Vec2, Vec3, Quat, etc.)
    fn transform_math_value(type_name: &BrpTypeName, value: &Value) -> Option<Value> {
        // Only try to transform if value is an object
        let obj = value.as_object()?;

        // Get the format knowledge for this type
        let format_knowledge = BRP_FORMAT_KNOWLEDGE.get(type_name)?;

        // Only math types have subfield_paths
        let subfield_paths = format_knowledge.subfield_paths.as_ref()?;

        // Extract values in the order defined by subfield_paths
        let mut values = Vec::new();
        for (component, _example) in subfield_paths {
            let field_name = match component {
                MathComponent::X => "x",
                MathComponent::Y => "y",
                MathComponent::Z => "z",
                MathComponent::W => "w",
            };

            // Try to get the field value as a number
            let field_value = obj.get(field_name)?.as_f64()?;
            values.push(field_value);
        }

        // Return as array
        Some(serde_json::json!(values))
    }

    /// Transform struct values - handles specific known struct types
    fn transform_struct_value(type_name: &BrpTypeName, value: &Value) -> Option<Value> {
        // Only handle specific Transform types we know about
        match type_name.as_str() {
            "bevy_transform::components::transform::Transform"
            | "bevy_transform::components::global_transform::GlobalTransform" => {
                debug!("Attempting to transform {} component", type_name.as_str());
                Self::transform_bevy_transform_fields(value)
            }
            _ => {
                // We don't attempt to transform unknown struct types
                debug!(
                    "No transformation available for struct type '{}'",
                    type_name.as_str()
                );
                None
            }
        }
    }

    /// Transform the fields of a Transform or `GlobalTransform` component
    fn transform_bevy_transform_fields(value: &Value) -> Option<Value> {
        let obj = value.as_object()?;
        let mut result = serde_json::Map::new();
        let mut transformed_any = false;

        // Define the expected field types for Transform components
        // Both Transform and GlobalTransform have the same fields
        let field_types = [
            ("translation", BrpTypeName::from("glam::Vec3")),
            ("rotation", BrpTypeName::from("glam::Quat")),
            ("scale", BrpTypeName::from("glam::Vec3")),
        ];

        for (field_name, field_value) in obj {
            // Find the expected type for this field
            let transformed_value = field_types
                .iter()
                .find(|(name, _)| name == field_name)
                .and_then(|(_, field_type)| {
                    debug!(
                        "Attempting to transform field '{}' as type '{}'",
                        field_name,
                        field_type.as_str()
                    );
                    Self::transform_math_value(field_type, field_value)
                });

            if let Some(transformed) = transformed_value {
                debug!(
                    "Successfully transformed field '{}' from object to array format",
                    field_name
                );
                result.insert(field_name.clone(), transformed);
                transformed_any = true;
            } else {
                debug!(
                    "Field '{}' does not need transformation or cannot be transformed",
                    field_name
                );
                // Copy field as-is
                result.insert(field_name.clone(), field_value.clone());
            }
        }

        if transformed_any {
            debug!("Transform component had fields that were transformed");
            Some(Value::Object(result))
        } else {
            debug!("No fields in Transform component needed transformation");
            None
        }
    }

    /// Transform enum values
    fn transform_enum_value(&self, type_name: &BrpTypeName, value: &Value) -> Option<Value> {
        let type_info = self.get_type_info(type_name)?;
        if let Some(enum_info) = &type_info.enum_info {
            // Handle string to enum variant conversion
            if let Some(str_val) = value.as_str() {
                // Check if string matches a variant name
                if enum_info.iter().any(|v| v.variant_name == str_val) {
                    // For unit variants, just return the string
                    return Some(Value::String(str_val.to_string()));
                }
            }
        }
        None
    }
}
