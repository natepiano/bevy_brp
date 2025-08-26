//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `TypeContext`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;
use std::fmt::Write;

use serde::Serialize;
use serde_json::Value;
use tracing::debug;

use super::types::{Correction, CorrectionInfo, CorrectionMethod, Operation};
use crate::brp_tools::brp_type_schema::{
    BrpTypeName, EnumVariantInfo, MutationPath, TypeInfo, TypeKind,
};
use crate::tool::ParameterName;

/// Comprehensive type information unified across all discovery sources
#[derive(Debug, Clone, Serialize)]
pub struct TypeContext {
    /// Complete type information from registry
    pub type_info:      TypeInfo,
    /// The original value from parameters
    pub original_value: Value,
    /// Mutation path for mutation operations (e.g., ".translation")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation_path:  Option<String>,
}

impl TypeContext {
    /// Create `TypeContext` from `TypeInfo` (single source of truth constructor)
    pub fn from_type_info(
        type_info: TypeInfo,
        original_value: Value,
        mutation_path: Option<String>,
    ) -> Self {
        Self {
            type_info,
            original_value,
            mutation_path,
        }
    }

    // Convenience methods for TypeInfo field access

    /// Get the type name
    pub const fn type_name(&self) -> &BrpTypeName {
        &self.type_info.type_name
    }

    /// Check if the type is registered in the Bevy registry
    pub const fn in_registry(&self) -> bool {
        self.type_info.in_registry
    }

    /// Check if the type is BRP compatible (has both Serialize and Deserialize traits)
    pub const fn is_brp_compatible(&self) -> bool {
        self.type_info.has_serialize && self.type_info.has_deserialize
    }

    /// Get enum information if this is an enum type
    pub const fn enum_info(&self) -> Option<&Vec<EnumVariantInfo>> {
        self.type_info.enum_info.as_ref()
    }

    /// Get mutation paths for this type
    pub const fn mutation_paths(&self) -> &HashMap<String, MutationPath> {
        &self.type_info.mutation_paths
    }

    /// Get the mutation paths for this type
    pub const fn get_mutation_paths(&self) -> &HashMap<String, MutationPath> {
        &self.type_info.mutation_paths
    }

    /// Check if this type is a math type using BRP format knowledge
    /// Delegate to `TypeInfo`'s `is_math_type` method
    fn is_math_type(&self) -> bool {
        self.type_info.is_math_type()
    }

    /// Get example for a specific operation
    pub const fn get_example_for_operation(&self, operation: Operation) -> Option<&Value> {
        match operation {
            Operation::SpawnInsert { .. } => self.type_info.spawn_format.as_ref(),
            Operation::Mutate { .. } => None, /* For other operations, check corrected format if
                                               * available */
        }
    }

    /// Create appropriate correction based on the operation and context
    /// We check if its a mutation operation - given we are attempting to recover from an error
    /// we can't predict the correct path to use so we provide guidance in an `Uncorrectable`
    /// Otherwise we continue to create a possible `Candidate`
    pub fn build_correction(&self, operation: Operation) -> Correction {
        debug!(
            "to_correction called for type '{}' with operation: {:?}",
            self.type_info.type_name, operation
        );

        match operation {
            Operation::Mutate { .. } => {
                debug!(
                    "This is a mutation operation for type '{}'",
                    self.type_info.type_name
                );

                // Extract mutation path from original parameters to determine what field is being
                // mutated
                let mutation_path = self.extract_mutation_path();
                debug!(
                    "Extracted mutation path: '{}'",
                    mutation_path.as_deref().unwrap_or("(root)")
                );

                // Check if the specific field being mutated is a math type
                let is_math_field = if let Some(path) = &mutation_path {
                    // Field mutation - check if the field's type is a math type
                    let field_name = path.trim_start_matches('.');
                    if let Some(field_type_info) = self.type_info.field_type_infos.get(field_name) {
                        let is_math = field_type_info.is_math_type();
                        debug!(
                            "Field '{}' type '{}' is_math_type: {}",
                            field_name, field_type_info.type_name, is_math
                        );
                        is_math
                    } else {
                        debug!("Field '{}' not found in field_type_infos", field_name);
                        false
                    }
                } else {
                    // Root mutation - check if this type itself is a math type
                    let is_math = self.is_math_type();
                    debug!(
                        "Root mutation - type '{}' is_math_type: {}",
                        self.type_info.type_name, is_math
                    );
                    is_math
                };

                if is_math_field {
                    debug!("Mutation target is a math type, creating Candidate correction");
                    // This mutation can be auto-corrected - enable retry mode
                    self.to_correction()
                } else {
                    debug!("Mutation target is not a math type, creating Uncorrectable correction");
                    // Create mutation guidance for non-transformable types
                    let mut hint = format!(
                        "Type '{}' supports mutation. Available paths:\n",
                        self.type_info.type_name
                    );
                    for (path, mutation_path) in self.get_mutation_paths() {
                        let _ = writeln!(hint, "  {path} - {}", mutation_path.description);
                    }

                    Correction::Uncorrectable {
                        type_info: self.clone(),
                        reason:    hint,
                    }
                }
            }
            Operation::SpawnInsert { .. } => {
                debug!(
                    "This is a spawn/insert operation for type '{}'",
                    self.type_info.type_name
                );
                self.to_correction()
            }
        }
    }

    /// Get mutation path for mutation operations
    fn extract_mutation_path(&self) -> Option<String> {
        self.mutation_path.clone()
    }

    /// Convert this type info to a `Correction`
    fn to_correction(&self) -> Correction {
        // Check if this is an enum with variants - provide guidance only
        if let Some(enum_info) = &self.type_info.enum_info {
            let variant_names: Vec<String> =
                enum_info.iter().map(|v| v.variant_name.clone()).collect();

            let example_variant = variant_names.first().map_or("VariantName", String::as_str);

            let reason = format!(
                "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}. Use one of these values directly (e.g., \"{}\")",
                self.type_info
                    .type_name
                    .as_str()
                    .split("::")
                    .last()
                    .unwrap_or(self.type_info.type_name.as_str()),
                variant_names.join(", "),
                example_variant
            );

            return Correction::Uncorrectable {
                type_info: self.clone(),
                reason,
            };
        }

        // Check if we can actually transform the original input
        tracing::debug!(
            "Extras Integration: Attempting to transform original value: {}",
            serde_json::to_string(&self.original_value)
                .unwrap_or_else(|_| "invalid json".to_string())
        );

        // For mutations with a path, use the field's TypeInfo for transformation
        let transformed_value = if let Some(path) = &self.mutation_path {
            let field_name = path.trim_start_matches('.');
            if let Some(field_type_info) = self.type_info.field_type_infos.get(field_name) {
                debug!(
                    "Using field '{}' TypeInfo '{}' for transformation",
                    field_name, field_type_info.type_name
                );
                // Create temporary TypeContext for the field to use its transform_value
                let field_unified = TypeContext::from_type_info(
                    field_type_info.clone(),
                    self.original_value.clone(),
                    None, // Field doesn't have its own mutation path
                );
                field_unified.transform_value(&self.original_value)
            } else {
                debug!(
                    "Field '{}' not found in field_type_infos, using parent transformation",
                    field_name
                );
                self.transform_value(&self.original_value)
            }
        } else {
            // No mutation path - use self transformation
            self.transform_value(&self.original_value)
        };

        if let Some(transformed_value) = transformed_value {
            tracing::debug!(
                "Extras Integration: Successfully transformed value to: {}",
                serde_json::to_string(&transformed_value)
                    .unwrap_or_else(|_| "invalid json".to_string())
            );
            // We can transform the input - return Corrected with actual transformation
            let correction_info = CorrectionInfo {
                corrected_value:   transformed_value,
                hint:              format!(
                    "Transformed {} format for type '{}'",
                    if self.original_value.is_object() {
                        "object"
                    } else {
                        "value"
                    },
                    self.type_info.type_name
                ),
                corrected_format:  None,
                type_info:         self.clone(),
                correction_method: CorrectionMethod::ObjectToArray,
            };

            return Correction::Candidate { correction_info };
        }
        tracing::debug!(
            "Extras Integration: transform_value() returned None - cannot transform input"
        );

        // Cannot transform input - provide guidance with examples
        let reason = self.get_example_for_operation(Operation::SpawnInsert {
            parameter_name: ParameterName::Components,
        }).map_or_else(|| format!(
                "Cannot transform input for type '{}'. Type discovered but no format example available.",
                self.type_info.type_name
            ), |spawn_example| format!(
                "Cannot transform input for type '{}'. Use this format: {}",
                self.type_info.type_name,
                serde_json::to_string(spawn_example)
                    .unwrap_or_else(|_| "correct format".to_string())
            ));

        Correction::Uncorrectable {
            type_info: self.clone(),
            reason,
        }
    }

    /// Transform an incorrect value to the correct format
    pub fn transform_value(&self, value: &Value) -> Option<Value> {
        let type_kind = self
            .type_info
            .schema_info
            .as_ref()
            .and_then(|s| s.type_kind.clone())
            .unwrap_or_else(|| {
                if self.type_info.enum_info.is_some() {
                    TypeKind::Enum
                } else {
                    TypeKind::Struct
                }
            });
        match type_kind {
            TypeKind::Enum => self.transform_enum_value(value),
            TypeKind::Struct => {
                if self.is_math_type() {
                    self.transform_math_value(value)
                } else {
                    self.transform_struct_value(value)
                }
            }
            _ => {
                tracing::debug!(
                    "No transformation available for type_kind={:?} (type='{}')",
                    type_kind,
                    self.type_info.type_name
                );
                None
            }
        }
    }

    /// Transform math type values (Vec2, Vec3, Quat, etc.)
    fn transform_math_value(&self, value: &Value) -> Option<Value> {
        // Handle object to array conversion for math types
        value
            .as_object()
            .and_then(|obj| match self.type_info.type_name.as_str() {
                name if name.contains("Vec2") => Self::extract_vec2_from_object(obj),
                name if name.contains("Vec3") => Self::extract_vec3_from_object(obj),
                name if name.contains("Vec4") => Self::extract_vec4_from_object(obj),
                name if name.contains("Quat") => Self::extract_quat_from_object(obj),
                _ => None,
            })
    }

    /// Extract Vec2 array from object
    fn extract_vec2_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y]))
    }

    /// Extract Vec3 array from object
    fn extract_vec3_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        let z = obj.get("z").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y, z]))
    }

    /// Extract Vec4 array from object
    fn extract_vec4_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        let x = obj.get("x").and_then(Value::as_f64)?;
        let y = obj.get("y").and_then(Value::as_f64)?;
        let z = obj.get("z").and_then(Value::as_f64)?;
        let w = obj.get("w").and_then(Value::as_f64)?;
        Some(serde_json::json!([x, y, z, w]))
    }

    /// Extract Quaternion array from object
    fn extract_quat_from_object(obj: &serde_json::Map<String, Value>) -> Option<Value> {
        // Same as Vec4 for quaternions
        Self::extract_vec4_from_object(obj)
    }

    /// Transform struct values - only transform if input is valid and transformable
    fn transform_struct_value(&self, value: &Value) -> Option<Value> {
        // Check if this is a Transform type with child math types that can be transformed
        if self.type_info.type_name.as_str().contains("Transform") {
            // Try to transform object format to array format for math fields
            if let Some(obj) = value.as_object() {
                let mut result = serde_json::Map::new();
                let mut transformed_any = false;

                for (field_name, field_value) in obj {
                    match field_name.as_str() {
                        "translation" | "scale" => {
                            // Try to transform Vec3 object to array
                            if let Some(field_obj) = field_value.as_object() {
                                if let Some(vec3_array) = Self::extract_vec3_from_object(field_obj)
                                {
                                    result.insert(field_name.clone(), vec3_array);
                                    transformed_any = true;
                                } else {
                                    // Cannot transform this field - copy as-is
                                    result.insert(field_name.clone(), field_value.clone());
                                }
                            } else {
                                // Field is not an object, copy as-is
                                result.insert(field_name.clone(), field_value.clone());
                            }
                        }
                        "rotation" => {
                            // Try to transform Quat object to array
                            if let Some(field_obj) = field_value.as_object() {
                                if let Some(quat_array) = Self::extract_quat_from_object(field_obj)
                                {
                                    result.insert(field_name.clone(), quat_array);
                                    transformed_any = true;
                                } else {
                                    // Cannot transform this field - copy as-is
                                    result.insert(field_name.clone(), field_value.clone());
                                }
                            } else {
                                // Field is not an object, copy as-is
                                result.insert(field_name.clone(), field_value.clone());
                            }
                        }
                        _ => {
                            // Copy other fields as-is
                            result.insert(field_name.clone(), field_value.clone());
                        }
                    }
                }

                if transformed_any {
                    return Some(Value::Object(result));
                }
            }
        }

        // For other struct types, return None - no transformation possible
        None
    }

    /// Transform enum values
    fn transform_enum_value(&self, value: &Value) -> Option<Value> {
        if let Some(enum_info) = &self.type_info.enum_info {
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
