//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `UnifiedTypeInfo`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;
use std::fmt::Write;

use serde::Serialize;
use serde_json::Value;
use tracing::debug;

use super::types::{Correction, CorrectionInfo, CorrectionMethod, FormatInfo, Operation};
use crate::brp_tools::brp_type_schema::{
    BrpTypeName, EnumVariantInfo, EnumVariantKind, MutationPath, TypeInfo, TypeKind,
};
use crate::tool::{BrpMethod, ParameterName};

/// Comprehensive type information unified across all discovery sources
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedTypeInfo {
    /// Complete type information from registry
    pub type_info:      TypeInfo,
    /// The original value from parameters
    pub original_value: Value,
    /// Format-specific data and examples
    pub format_info:    FormatInfo,
}

impl UnifiedTypeInfo {
    /// Create a new `UnifiedTypeInfo` with minimal required information
    /// This is now private - use specialized constructors instead
    fn new(type_name: impl Into<BrpTypeName>, original_value: Value) -> Self {
        // Create minimal TypeInfo for pattern matching cases
        let type_info = TypeInfo {
            type_name:            type_name.into(),
            spawn_format:         None,
            mutation_paths:       HashMap::new(),
            in_registry:          false,
            has_serialize:        false,
            has_deserialize:      false,
            supported_operations: Vec::new(),
            example_values:       HashMap::new(),
            schema_info:          None,
            enum_info:            None,
            error:                None,
        };

        Self {
            type_info,
            original_value,
            format_info: FormatInfo::default(),
        }
    }

    /// Create `UnifiedTypeInfo` from `TypeInfo` (single source of truth constructor)
    pub fn from_type_info(
        type_info: TypeInfo, // Take ownership instead of reference
        original_value: Value,
        _method: BrpMethod, // No longer needed for examples
    ) -> Self {
        Self {
            type_info,
            original_value,
            format_info: FormatInfo::default(), // Only populate if corrections needed
        }
    }

    /// Create `UnifiedTypeInfo` for enum types with variant names
    ///
    /// Used when pattern matching identifies an enum with specific variants.
    /// Sets appropriate type category, enum info, and generates examples.
    pub fn for_enum_type(
        type_name: impl Into<BrpTypeName>,
        variant_names: Vec<String>,
        original_value: Value,
    ) -> Self {
        let mut info = Self::new(type_name, original_value);

        // Update TypeInfo with enum variants
        if !variant_names.is_empty() {
            use crate::brp_tools::brp_type_schema::EnumVariantInfo;
            let variants = variant_names
                .into_iter()
                .map(|name| EnumVariantInfo {
                    variant_name: name,
                    variant_kind: EnumVariantKind::Unit,
                    fields:       None,
                    tuple_types:  None,
                })
                .collect();
            info.type_info.enum_info = Some(variants);
        }

        // Update schema_info to indicate this is an enum
        if let Some(ref mut schema_info) = info.type_info.schema_info {
            schema_info.type_kind = Some(TypeKind::Enum);
        }

        info
    }

    /// Create `UnifiedTypeInfo` for a specific math type
    ///
    /// Used when pattern matching identifies a math type (Vec2, Vec3, etc).
    /// Sets appropriate type category and generates examples.
    pub fn for_math_type(type_name: impl Into<BrpTypeName>, original_value: Value) -> Self {
        let mut info = Self::new(type_name, original_value);

        // Mark this as a math type by setting an error indicating pattern matching
        info.type_info.error = Some("Identified as math type via pattern matching".to_string());

        info
    }

    /// Create `UnifiedTypeInfo` for Transform types
    ///
    /// Used when pattern matching identifies a Transform component.
    /// Sets appropriate type category, child types, and generates examples.
    pub fn for_transform_type(type_name: impl Into<BrpTypeName>, original_value: Value) -> Self {
        let mut info = Self::new(type_name, original_value);

        // Mark this as a transform type by setting an error indicating pattern matching
        info.type_info.error =
            Some("Identified as transform type via pattern matching".to_string());

        info
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

    /// Check if this type supports mutation operations
    pub fn supports_mutation(&self) -> bool {
        !self.type_info.mutation_paths.is_empty()
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
    /// Only called from extras discovery so this indicates the `correction_source`
    /// We check if its a mutation operation - given we are attempting to recover from an error
    /// we can't predict the correct path to use so we provide guidance in an `Uncorrectable`
    /// Otherwise we continue to create a possible `Candidate`
    pub fn to_correction(&self, operation: Operation) -> Correction {
        // Check if this is a mutation operation and we have mutation paths
        if matches!(operation, Operation::Mutate { .. }) && self.supports_mutation() {
            // Create mutation guidance
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
        } else {
            self.to_spawn_insert_correction()
        }
    }

    /// Convert this type info to a `Correction`
    fn to_spawn_insert_correction(&self) -> Correction {
        debug!(
            "to_correction: Converting type '{}' with enum_info: {}",
            self.type_info.type_name,
            if self.type_info.enum_info.is_some() {
                "present"
            } else {
                "missing"
            }
        );

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
        if let Some(transformed_value) = self.transform_value(&self.original_value) {
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
        // Note: to_correction_internal is only called for SpawnInsert operations (Mutate returns
        // early)
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
