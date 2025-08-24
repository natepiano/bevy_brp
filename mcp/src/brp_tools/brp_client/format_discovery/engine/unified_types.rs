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

use super::types::{
    Correction, CorrectionInfo, CorrectionMethod, DiscoverySource, EnumInfo, EnumVariant,
    FormatInfo, Operation, RegistryStatus, SerializationSupport,
};
use crate::brp_tools::brp_type_schema::{
    BrpTypeName, EnumVariantKind, MutationPath, TypeInfo, TypeKind,
};
use crate::tool::{BrpMethod, ParameterName};

/// Comprehensive type information unified across all discovery sources
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedTypeInfo {
    /// The fully-qualified type name
    pub type_name:        BrpTypeName,
    /// The original value from parameters
    pub original_value:   Value,
    /// Registry and reflection information
    pub registry_status:  RegistryStatus,
    /// Serialization support information
    pub serialization:    SerializationSupport,
    /// Format-specific data and examples
    pub format_info:      FormatInfo,
    /// Type kind for quick identification
    pub type_kind:        TypeKind,
    /// Enum variant information (only populated for enum types)
    pub enum_info:        Option<EnumInfo>,
    /// Source of this type information for debugging
    pub discovery_source: DiscoverySource,
}

impl UnifiedTypeInfo {
    /// Create a new `UnifiedTypeInfo` with minimal required information
    /// This is now private - use specialized constructors instead
    fn new(
        type_name: impl Into<BrpTypeName>,
        original_value: Value,
        discovery_source: DiscoverySource,
    ) -> Self {
        Self {
            type_name: type_name.into(),
            original_value,
            registry_status: RegistryStatus {
                in_registry: false,
                has_reflect: false,
                type_path:   None,
            },
            serialization: SerializationSupport {
                has_serialize:   false,
                has_deserialize: false,
                brp_compatible:  false,
            },
            format_info: FormatInfo {
                examples:         HashMap::new(),
                mutation_paths:   HashMap::new(),
                original_format:  None,
                corrected_format: None,
            },
            type_kind: TypeKind::Value,
            enum_info: None,
            discovery_source,
        }
    }

    /// Create `UnifiedTypeInfo` from `TypeInfo` (single source of truth constructor)
    pub fn from_type_info(
        type_info: &TypeInfo,
        original_value: Value,
        method: BrpMethod,
    ) -> Result<Self, crate::error::Error> {
        let type_name = type_info.type_name.clone();

        // Determine operation for examples
        let operation = Operation::try_from(method)?;

        // Build examples HashMap
        let mut examples = HashMap::new();
        if let Some(spawn_format) = &type_info.spawn_format {
            // Add spawn_format to examples if this is a SpawnInsert operation
            if matches!(operation, Operation::SpawnInsert { .. }) {
                examples.insert(operation, spawn_format.clone());
            }
        }

        // Use TypeInfo mutation_paths directly
        let mutation_paths = type_info.mutation_paths.clone();

        // Convert enum_info from TypeInfo format to our format
        let enum_info = type_info.enum_info.as_ref().map(|enum_variants| {
            let variants = enum_variants
                .iter()
                .map(|variant_info| EnumVariant {
                    name:         variant_info.variant_name.clone(),
                    variant_kind: variant_info.variant_kind,
                    fields:       variant_info.fields.clone(),
                    tuple_types:  variant_info.tuple_types.clone(),
                })
                .collect();
            EnumInfo { variants }
        });

        // Compute has_reflect from TypeInfo's reflect traits
        let has_reflect = type_info.schema_info.is_some() || type_info.in_registry;

        // Compute brp_compatible as has_serialize && has_deserialize
        let brp_compatible = type_info.has_serialize && type_info.has_deserialize;

        // Determine TypeKind from schema_info or default to Value
        let type_kind = type_info
            .schema_info
            .as_ref()
            .and_then(|schema| schema.type_kind.clone())
            .unwrap_or(TypeKind::Value);

        Ok(Self {
            type_name,
            original_value,
            registry_status: RegistryStatus {
                in_registry: type_info.in_registry,
                has_reflect,
                type_path: None, // Not directly available from TypeInfo
            },
            serialization: SerializationSupport {
                has_serialize: type_info.has_serialize,
                has_deserialize: type_info.has_deserialize,
                brp_compatible,
            },
            format_info: FormatInfo {
                examples,
                mutation_paths,
                original_format: None,
                corrected_format: None,
            },
            type_kind,
            enum_info,
            discovery_source: DiscoverySource::TypeRegistry,
        })
    }

    /// Enrich this type info with data from `bevy_brp_extras` discovery

    /// Create `UnifiedTypeInfo` for enum types with variant names
    ///
    /// Used when pattern matching identifies an enum with specific variants.
    /// Sets appropriate type category, enum info, and generates examples.
    pub fn for_enum_type(
        type_name: impl Into<BrpTypeName>,
        variant_names: Vec<String>,
        original_value: Value,
    ) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_kind = TypeKind::Enum;
        if !variant_names.is_empty() {
            let variants = variant_names
                .into_iter()
                .map(|name| EnumVariant {
                    name,
                    variant_kind: EnumVariantKind::Unit,
                    fields: None,
                    tuple_types: None,
                })
                .collect();
            info.enum_info = Some(EnumInfo { variants });
        }
        info.generate_all_examples();
        info
    }

    /// Create `UnifiedTypeInfo` for a specific math type
    ///
    /// Used when pattern matching identifies a math type (Vec2, Vec3, etc).
    /// Sets appropriate type category and generates examples.
    pub fn for_math_type(type_name: impl Into<BrpTypeName>, original_value: Value) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_kind = TypeKind::Struct;
        info.generate_all_examples();
        info
    }

    /// Create `UnifiedTypeInfo` for Transform types
    ///
    /// Used when pattern matching identifies a Transform component.
    /// Sets appropriate type category, child types, and generates examples.
    pub fn for_transform_type(type_name: impl Into<BrpTypeName>, original_value: Value) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_kind = TypeKind::Struct;

        info.generate_all_examples();
        info
    }

    /// Get the mutation paths for this type
    pub const fn get_mutation_paths(&self) -> &HashMap<String, MutationPath> {
        &self.format_info.mutation_paths
    }

    /// Check if this type is a math type using BRP format knowledge
    /// This replaces the old string-based matching and uses the same logic as
    /// `TypeInfo.is_math_type()`
    fn is_math_type(&self) -> bool {
        use crate::brp_tools::brp_type_schema::BRP_FORMAT_KNOWLEDGE;
        BRP_FORMAT_KNOWLEDGE
            .get(&self.type_name)
            .is_some_and(|knowledge| knowledge.subfield_paths.is_some())
    }

    /// Check if this type supports mutation operations
    pub fn supports_mutation(&self) -> bool {
        !self.format_info.mutation_paths.is_empty()
    }

    /// Get example for a specific operation
    pub fn get_example_for_operation(&self, operation: Operation) -> Option<&Value> {
        self.format_info.examples.get(&operation)
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
                self.type_name
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
            self.type_name,
            if self.enum_info.is_some() {
                "present"
            } else {
                "missing"
            }
        );

        // Check if this is an enum with variants - provide guidance only
        if let Some(enum_info) = &self.enum_info {
            let variant_names: Vec<String> =
                enum_info.variants.iter().map(|v| v.name.clone()).collect();

            let example_variant = variant_names.first().map_or("VariantName", String::as_str);

            let reason = format!(
                "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}. Use one of these values directly (e.g., \"{}\")",
                self.type_name
                    .as_str()
                    .split("::")
                    .last()
                    .unwrap_or(self.type_name.as_str()),
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
                    self.type_name
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
                self.type_name
            ), |spawn_example| format!(
                "Cannot transform input for type '{}'. Use this format: {}",
                self.type_name,
                serde_json::to_string(spawn_example)
                    .unwrap_or_else(|_| "correct format".to_string())
            ));

        Correction::Uncorrectable {
            type_info: self.clone(),
            reason,
        }
    }

    /// Regenerate all examples based on current type information
    fn generate_all_examples(&mut self) {
        // Clear existing examples
        self.format_info.examples.clear();

        // Generate spawn/insert example
        if let Some(example) = self.generate_spawn_insert_example() {
            self.format_info.examples.insert(
                Operation::SpawnInsert {
                    parameter_name: ParameterName::Components,
                },
                example,
            );
        }

        // Generate mutation example if type supports mutation
        if self.supports_mutation()
            && let Some(example) = self.generate_mutation_example()
        {
            self.format_info.examples.insert(
                Operation::Mutate {
                    parameter_name: ParameterName::Component,
                },
                example,
            );
        }
    }
    /// Generate spawn example based on type structure
    fn generate_spawn_insert_example(&self) -> Option<Value> {
        match self.type_kind {
            TypeKind::Enum => self.generate_enum_example(),
            TypeKind::Struct => {
                if self.is_math_type() {
                    self.generate_math_type_example()
                } else {
                    self.generate_struct_example()
                }
            }
            _ => None,
        }
    }

    /// Generate mutation example with paths
    fn generate_mutation_example(&self) -> Option<Value> {
        if let Some((path, mutation_path)) = self.format_info.mutation_paths.iter().next() {
            Some(serde_json::json!({
                "path": path,
                "value": Self::generate_value_for_type(&mutation_path.description),
                "description": mutation_path.description
            }))
        } else {
            None
        }
    }

    /// Generate example for struct types
    fn generate_struct_example(&self) -> Option<Value> {
        // For now, return corrected format if available
        self.format_info.corrected_format.clone()
    }

    /// Generate example for enum types
    fn generate_enum_example(&self) -> Option<Value> {
        self.enum_info.as_ref().and_then(|enum_info| {
            enum_info
                .variants
                .first()
                .map(|variant| match variant.variant_kind {
                    EnumVariantKind::Unit => Value::String(variant.name.clone()),
                    _ => serde_json::json!({
                        variant.name.clone(): {}
                    }),
                })
        })
    }

    /// Generate example for math types (Vec2, Vec3, etc.)
    fn generate_math_type_example(&self) -> Option<Value> {
        match self.type_name.as_str() {
            name if name.contains("Vec2") => Some(serde_json::json!([0.0, 0.0])),
            name if name.contains("Vec3") => Some(serde_json::json!([0.0, 0.0, 0.0])),
            name if name.contains("Vec4") => Some(serde_json::json!([0.0, 0.0, 0.0, 0.0])),
            name if name.contains("Quat") => Some(serde_json::json!([0.0, 0.0, 0.0, 1.0])),
            _ => None,
        }
    }

    /// Generate appropriate value for a type description
    fn generate_value_for_type(type_desc: &str) -> Value {
        match type_desc {
            desc if desc.contains("f32") || desc.contains("float") => Value::from(0.0),
            desc if desc.contains("i32") || desc.contains("int") => Value::from(0),
            desc if desc.contains("bool") => Value::from(false),
            desc if desc.contains("String") => Value::from(""),
            _ => Value::Null,
        }
    }

    /// Transform an incorrect value to the correct format
    pub fn transform_value(&self, value: &Value) -> Option<Value> {
        match self.type_kind {
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
                    self.type_kind,
                    self.type_name
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
            .and_then(|obj| match self.type_name.as_str() {
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
        if self.type_name.as_str().contains("Transform") {
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
        if let Some(enum_info) = &self.enum_info {
            // Handle string to enum variant conversion
            if let Some(str_val) = value.as_str() {
                // Check if string matches a variant name
                if enum_info.variants.iter().any(|v| v.name == str_val) {
                    // For unit variants, just return the string
                    return Some(Value::String(str_val.to_string()));
                }
            }
        }
        None
    }
}
