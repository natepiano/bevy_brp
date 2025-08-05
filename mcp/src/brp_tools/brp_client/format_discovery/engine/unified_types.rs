//! Unified type system for format discovery
//!
//! Single coherent schema replacing fragmented type conversions. Contains all
//! discoverable type information in one place to prevent data loss.
//!
//! Core types: `UnifiedTypeInfo`, `FormatInfo`, `RegistryStatus`, `SerializationSupport`

use std::collections::HashMap;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use super::types::{
    BrpTypeName, Correction, CorrectionInfo, CorrectionMethod, DiscoverySource, EnumInfo,
    EnumVariant, FormatInfo, RegistryStatus, SerializationSupport, TypeCategory,
};
use crate::brp_tools::brp_client::format_discovery::format_correction_fields::FormatCorrectionField;
use crate::tool::BrpMethod;

/// Comprehensive type information unified across all discovery sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTypeInfo {
    /// The fully-qualified type name
    pub type_name:            BrpTypeName,
    /// The original value from parameters
    pub original_value:       Option<Value>,
    /// Registry and reflection information
    pub registry_status:      RegistryStatus,
    /// Serialization support information
    pub serialization:        SerializationSupport,
    /// Format-specific data and examples
    pub format_info:          FormatInfo,
    /// List of supported BRP operations for this type
    pub supported_operations: Vec<String>,
    /// Type category for quick identification
    pub type_category:        TypeCategory,
    /// Enum variant information (only populated for enum types)
    pub enum_info:            Option<EnumInfo>,
    /// Source of this type information for debugging
    pub discovery_source:     DiscoverySource,
}

impl UnifiedTypeInfo {
    /// Create a new `UnifiedTypeInfo` with minimal required information
    /// This is now private - use specialized constructors instead
    fn new(
        type_name: impl Into<BrpTypeName>,
        original_value: Option<Value>,
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
            supported_operations: Vec::new(),
            type_category: TypeCategory::Unknown,
            enum_info: None,
            discovery_source,
        }
    }

    /// Create `UnifiedTypeInfo` for pattern matching error handling
    ///
    /// Used when creating error corrections from pattern analysis.
    /// Automatically generates examples before returning.
    pub fn for_pattern_matching(
        type_name: impl Into<BrpTypeName>,
        original_value: Option<Value>,
    ) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.regenerate_all_examples();
        info
    }

    /// Create `UnifiedTypeInfo` for a specific math type
    ///
    /// Used when pattern matching identifies a math type (Vec2, Vec3, etc).
    /// Sets appropriate type category and generates examples.
    pub fn for_math_type(type_name: impl Into<BrpTypeName>, original_value: Option<Value>) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_category = TypeCategory::MathType;
        info.regenerate_all_examples();
        info
    }

    /// Create `UnifiedTypeInfo` for enum types with variant names
    ///
    /// Used when pattern matching identifies an enum with specific variants.
    /// Sets appropriate type category, enum info, and generates examples.
    pub fn for_enum_type(
        type_name: impl Into<BrpTypeName>,
        variant_names: Vec<String>,
        original_value: Option<Value>,
    ) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_category = TypeCategory::Enum;
        if !variant_names.is_empty() {
            let variants = variant_names
                .into_iter()
                .map(|name| EnumVariant {
                    name,
                    variant_type: "Unit".to_string(),
                })
                .collect();
            info.enum_info = Some(EnumInfo { variants });
        }
        info.regenerate_all_examples();
        info
    }

    /// Create `UnifiedTypeInfo` for Transform types
    ///
    /// Used when pattern matching identifies a Transform component.
    /// Sets appropriate type category, child types, and generates examples.
    pub fn for_transform_type(
        type_name: impl Into<BrpTypeName>,
        original_value: Option<Value>,
    ) -> Self {
        let mut info = Self::new(type_name, original_value, DiscoverySource::PatternMatching);
        info.type_category = TypeCategory::Struct;

        info.regenerate_all_examples();
        info
    }

    /// Enrich this type info with data from `bevy_brp_extras` discovery
    ///
    /// This method is infallible - if extras data is malformed or missing,
    /// the type info remains unchanged. The `discovery_source` is only updated
    /// to `RegistryPlusExtras` if actual enrichment occurs.
    pub fn enrich_from_extras(&mut self, extras_response: &Value) {
        let mut enriched = false;

        // Extract and merge format examples from extras_response
        if let Some(examples) = Self::extract_examples_from_extras(extras_response) {
            // REPLACE: format_info.examples (extras data takes precedence)
            // This matches the old behavior where extras completely replaced registry format info
            self.format_info.examples.extend(examples);
            enriched = true;
        }

        // Extract and merge mutation paths from extras_response
        if let Some(mutation_paths) = Self::extract_mutation_paths_from_extras(extras_response) {
            // REPLACE: format_info.mutation_paths (extras data takes precedence)
            // This matches the old behavior where extras completely replaced registry format info
            self.format_info.mutation_paths.extend(mutation_paths);
            enriched = true;
        }

        // Extract and update type category from extras_response if available
        if let Some(type_category) = extras_response.get("type_category").and_then(Value::as_str) {
            let new_category = Self::parse_type_category(type_category);
            if new_category != TypeCategory::Unknown {
                self.type_category = new_category;
                enriched = true;
            }
        }

        // Extract and update enum_info from extras_response if available
        if let Some(enum_info) = Self::extract_enum_info_from_extras(extras_response) {
            debug!(
                "enrich_from_extras: Found enum_info with {} variants for type '{}'",
                enum_info.variants.len(),
                self.type_name
            );
            self.enum_info = Some(enum_info);
            enriched = true;
        } else {
            debug!(
                "enrich_from_extras: No enum_info found in extras response for type '{}'",
                self.type_name
            );
        }

        // UPDATE: discovery_source to RegistryPlusExtras (only if ANY enrichment occurred)
        if enriched {
            self.discovery_source = DiscoverySource::RegistryPlusExtras;
        }
    }

    /// Extract format examples from `bevy_brp_extras` response
    fn extract_examples_from_extras(extras_response: &Value) -> Option<HashMap<String, Value>> {
        let mut examples = HashMap::new();

        // Look for example_values field in the response
        if let Some(example_values) = extras_response
            .get("example_values")
            .and_then(Value::as_object)
        {
            for (operation, example) in example_values {
                examples.insert(operation.clone(), example.clone());
            }
        }

        // Only return if we found at least one example
        if examples.is_empty() {
            None
        } else {
            Some(examples)
        }
    }

    /// Extract mutation paths from `bevy_brp_extras` response
    fn extract_mutation_paths_from_extras(
        extras_response: &Value,
    ) -> Option<HashMap<String, String>> {
        let mut mutation_paths = HashMap::new();

        // Look for mutation_paths field in the response
        if let Some(paths) = extras_response
            .get("mutation_paths")
            .and_then(Value::as_object)
        {
            for (path, description) in paths {
                if let Some(desc_str) = description.as_str() {
                    mutation_paths.insert(path.clone(), desc_str.to_string());
                }
            }
        }

        // Only return if we found at least one mutation path
        if mutation_paths.is_empty() {
            None
        } else {
            Some(mutation_paths)
        }
    }

    /// Extract enum info from `bevy_brp_extras` response
    fn extract_enum_info_from_extras(extras_response: &Value) -> Option<EnumInfo> {
        debug!(
            "extract_enum_info_from_extras: Processing response: {}",
            serde_json::to_string_pretty(extras_response)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );

        extras_response.get("enum_info").and_then(|enum_obj| {
            enum_obj
                .get("variants")
                .and_then(Value::as_array)
                .map(|variants_array| {
                    let variants = variants_array
                        .iter()
                        .filter_map(|variant| {
                            if let Some(variant_obj) = variant.as_object() {
                                let name = variant_obj.get("name")?.as_str()?.to_string();
                                let variant_type = variant_obj
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Unit")
                                    .to_string();
                                Some(EnumVariant { name, variant_type })
                            } else {
                                None
                            }
                        })
                        .collect();

                    EnumInfo { variants }
                })
        })
    }

    /// Create `UnifiedTypeInfo` from Bevy registry schema
    ///
    /// Extracts registry status, reflection traits, and serialization support.
    /// Automatically generates examples before returning.
    pub fn from_registry_schema(
        type_name: impl Into<BrpTypeName>,
        schema_data: &Value,
        original_value: Option<Value>,
    ) -> Self {
        let type_name = type_name.into();
        // Extract reflect types
        let reflect_types = schema_data
            .get("reflectTypes")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        // Determine serialization support
        let has_serialize = reflect_types.contains(&"Serialize".to_string());
        let has_deserialize = reflect_types.contains(&"Deserialize".to_string());

        let registry_status = RegistryStatus {
            in_registry: true, // If we have schema data, it's in the registry
            has_reflect: reflect_types.contains(&"Default".to_string())
                || !reflect_types.is_empty(),
            type_path:   Some(type_name.as_str().to_string()),
        };

        let serialization = SerializationSupport {
            has_serialize,
            has_deserialize,
            brp_compatible: has_serialize && has_deserialize,
        };

        // Extract type category from schema if available
        let type_category = schema_data
            .get("type")
            .and_then(Value::as_str)
            .map_or(TypeCategory::Unknown, Self::parse_type_category);

        // Extract basic structure information for supported operations
        let supported_operations = if serialization.brp_compatible {
            match type_category {
                TypeCategory::Struct | TypeCategory::TupleStruct => vec![
                    "query".to_string(),
                    "get".to_string(),
                    "spawn".to_string(),
                    "insert".to_string(),
                    "mutate".to_string(),
                ],
                TypeCategory::Enum => vec![
                    "query".to_string(),
                    "get".to_string(),
                    "spawn".to_string(),
                    "insert".to_string(),
                ],
                _ => vec!["query".to_string(), "get".to_string()],
            }
        } else {
            // Without serialization, only reflection-based operations work
            vec!["query".to_string(), "get".to_string()]
        };

        // Extract enum information if this is an enum
        let enum_info = if type_category == TypeCategory::Enum {
            Self::extract_enum_info_from_schema(schema_data)
        } else {
            None
        };

        // Generate mutation paths based on schema structure
        let mutation_paths = Self::generate_mutation_paths_from_schema(schema_data);

        let mut unified_info = Self {
            type_name,
            original_value,
            registry_status,
            serialization,
            format_info: FormatInfo {
                examples: HashMap::new(),
                mutation_paths,
                original_format: None,
                corrected_format: None,
            },
            supported_operations,
            type_category,
            enum_info,
            discovery_source: DiscoverySource::TypeRegistry,
        };

        // Generate examples before returning
        unified_info.regenerate_all_examples();
        unified_info
    }

    /// Check if this type supports mutation operations
    pub fn supports_mutation(&self) -> bool {
        !self.format_info.mutation_paths.is_empty()
    }

    /// Get the mutation paths for this type
    pub const fn get_mutation_paths(&self) -> &HashMap<String, String> {
        &self.format_info.mutation_paths
    }

    /// Get example value for a specific operation
    pub fn get_example(&self, operation: &str) -> Option<&Value> {
        self.format_info.examples.get(operation)
    }

    /// Regenerate all examples based on current type information
    fn regenerate_all_examples(&mut self) {
        // Save any existing examples that we should preserve
        let existing_spawn = self.format_info.examples.get("spawn").cloned();
        let existing_insert = self.format_info.examples.get("insert").cloned();

        // Clear existing examples
        self.format_info.examples.clear();

        // Generate spawn/insert examples based on type category
        if let Some(example) = self.generate_spawn_example() {
            self.format_info
                .examples
                .insert("spawn".to_string(), example.clone());
            self.format_info
                .examples
                .insert("insert".to_string(), example);
        } else if self.type_category == TypeCategory::Component {
            // For Component types, preserve existing examples from discovery
            if let Some(spawn_example) = existing_spawn {
                self.format_info
                    .examples
                    .insert("spawn".to_string(), spawn_example);
            }
            if let Some(insert_example) = existing_insert {
                self.format_info
                    .examples
                    .insert("insert".to_string(), insert_example);
            }
        }

        // Generate mutation examples if we have mutation paths
        if self.supports_mutation() {
            if let Some(example) = self.generate_mutation_example() {
                self.format_info
                    .examples
                    .insert("mutate".to_string(), example);
            }
        }
    }

    /// Convert this type info to a `Correction`
    pub fn to_correction(&self, original_value: Option<Value>) -> Correction {
        debug!(
            "to_correction: Converting type '{}' with enum_info: {}",
            self.type_name,
            if self.enum_info.is_some() {
                "present"
            } else {
                "missing"
            }
        );

        // Check if this is an enum with variants - create enum-specific correction
        if let Some(enum_info) = &self.enum_info {
            let variant_names: Vec<String> =
                enum_info.variants.iter().map(|v| v.name.clone()).collect();

            let corrected_format = serde_json::json!({
                FormatCorrectionField::Hint.as_ref(): "Use empty path with variant name as value",
                FormatCorrectionField::ValidValues.as_ref(): variant_names,
                FormatCorrectionField::Examples.as_ref(): variant_names.iter().take(2).map(|variant| serde_json::json!({
                    FormatCorrectionField::Path.as_ref(): "",
                    FormatCorrectionField::Value.as_ref(): variant
                })).collect::<Vec<_>>()
            });

            let correction_info = CorrectionInfo {
                type_name:         self.type_name.clone(),
                original_value:    original_value.unwrap_or(serde_json::json!(null)),
                corrected_value:   corrected_format.clone(),
                corrected_format:  Some(corrected_format),
                hint:              format!(
                    "Enum '{}' requires empty path for unit variant mutation. Valid variants: {}",
                    self.type_name
                        .as_str()
                        .split("::")
                        .last()
                        .unwrap_or(self.type_name.as_str()),
                    variant_names.join(", ")
                ),
                target_type:       self.type_name.as_str().to_string(),
                type_info:         Some(self.clone()),
                correction_method: CorrectionMethod::DirectReplacement,
            };

            return Correction::Candidate { correction_info };
        }

        // Check if we can actually transform the original input
        if let Some(original_value) = original_value {
            tracing::debug!(
                "Extras Integration: Attempting to transform original value: {}",
                serde_json::to_string(&original_value)
                    .unwrap_or_else(|_| "invalid json".to_string())
            );
            if let Some(transformed_value) = self.transform_value(&original_value) {
                tracing::debug!(
                    "Extras Integration: Successfully transformed value to: {}",
                    serde_json::to_string(&transformed_value)
                        .unwrap_or_else(|_| "invalid json".to_string())
                );
                // We can transform the input - return Corrected with actual transformation
                let correction_info = CorrectionInfo {
                    type_name:         self.type_name.clone(),
                    original_value:    original_value.clone(),
                    corrected_value:   transformed_value,
                    hint:              format!(
                        "Transformed {} format for type '{}' (discovered via bevy_brp_extras)",
                        if original_value.is_object() {
                            "object"
                        } else {
                            "value"
                        },
                        self.type_name
                    ),
                    target_type:       self.type_name.as_str().to_string(),
                    corrected_format:  None,
                    type_info:         Some(self.clone()),
                    correction_method: CorrectionMethod::ObjectToArray,
                };

                return Correction::Candidate { correction_info };
            }
            tracing::debug!(
                "Extras Integration: transform_value() returned None - cannot transform input"
            );
        } else {
            tracing::debug!("Extras Integration: No original value provided for transformation");
        }

        // Cannot transform input - provide guidance with examples
        let reason = self.get_example("spawn").map_or_else(|| format!(
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

    /// Create appropriate correction based on the method and context
    /// Only called from extras discovery so this indicates the `correction_source`
    pub fn to_correction_for_method(&self, method: BrpMethod) -> Correction {
        // Check if this is a mutation method and we have mutation paths
        if matches!(
            method,
            BrpMethod::BevyMutateComponent | BrpMethod::BevyMutateResource
        ) && self.supports_mutation()
        {
            // Create mutation guidance
            let mut hint = format!(
                "Type '{}' supports mutation. Available paths:\n",
                self.type_name
            );
            for (path, description) in self.get_mutation_paths() {
                let _ = writeln!(hint, "  {path} - {description}");
            }

            Correction::Uncorrectable {
                type_info: self.clone(),
                reason:    hint,
            }
        } else {
            // Use existing correction logic with stored original_value
            self.to_correction(self.original_value.clone())
        }
    }

    /// Generate spawn example based on type structure
    fn generate_spawn_example(&self) -> Option<Value> {
        match self.type_category {
            TypeCategory::Struct => self.generate_struct_example(),
            TypeCategory::Enum => self.generate_enum_example(),
            TypeCategory::MathType => self.generate_math_type_example(),
            _ => None,
        }
    }

    /// Generate mutation example with paths
    fn generate_mutation_example(&self) -> Option<Value> {
        if let Some((path, description)) = self.format_info.mutation_paths.iter().next() {
            Some(serde_json::json!({
                "path": path,
                "value": Self::generate_value_for_type(description),
                "description": description
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
                .map(|variant| match variant.variant_type.as_str() {
                    "Unit" => Value::String(variant.name.clone()),
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
        match self.type_category {
            TypeCategory::MathType => self.transform_math_value(value),
            TypeCategory::Struct => self.transform_struct_value(value),
            TypeCategory::Enum => self.transform_enum_value(value),
            _ => {
                tracing::debug!(
                    "No transformation available for type_category={:?} (type='{}')",
                    self.type_category,
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

    /// Parse a type category string to the corresponding enum variant
    fn parse_type_category(category_str: &str) -> TypeCategory {
        match category_str {
            "Struct" => TypeCategory::Struct,
            "TupleStruct" => TypeCategory::TupleStruct,
            "Enum" => TypeCategory::Enum,
            "MathType" => TypeCategory::MathType,
            "Component" => TypeCategory::Component,
            _ => TypeCategory::Unknown,
        }
    }

    /// Extract enum variant information from registry schema
    fn extract_enum_info_from_schema(schema_data: &Value) -> Option<EnumInfo> {
        // Look for the "oneOf" field which contains enum variants
        schema_data
            .get("oneOf")
            .and_then(Value::as_array)
            .and_then(|one_of| {
                let variants: Vec<EnumVariant> = one_of
                    .iter()
                    .filter_map(|variant| {
                        match variant {
                            // Simple string variant (unit variants)
                            Value::String(variant_name) => Some(EnumVariant {
                                name:         variant_name.clone(),
                                variant_type: "Unit".to_string(),
                            }),
                            // Object variant (struct or tuple variants)
                            Value::Object(variant_obj) => {
                                variant_obj.get("shortPath").and_then(Value::as_str).map(
                                    |short_path| EnumVariant {
                                        name:         short_path.to_string(),
                                        variant_type: "Unit".to_string(), /* Most registry enums
                                                                           * are
                                                                           * unit variants */
                                    },
                                )
                            }
                            _ => None,
                        }
                    })
                    .collect();

                if variants.is_empty() {
                    None
                } else {
                    Some(EnumInfo { variants })
                }
            })
    }

    /// Generate mutation paths from registry schema structure
    fn generate_mutation_paths_from_schema(schema_data: &Value) -> HashMap<String, String> {
        let mut paths = HashMap::new();

        // Get the type kind
        let kind = schema_data
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("");

        match kind {
            "TupleStruct" => {
                // For tuple structs, generate paths based on prefixItems
                if let Some(prefix_items) = schema_data.get("prefixItems").and_then(Value::as_array)
                {
                    for (index, item) in prefix_items.iter().enumerate() {
                        // Basic tuple access path
                        paths.insert(
                            format!(".{index}"),
                            format!("Access field {index} of the tuple struct"),
                        );

                        // Check if this field is a Color type
                        if let Some(type_ref) = item
                            .get("type")
                            .and_then(|t| t.get("$ref"))
                            .and_then(Value::as_str)
                        {
                            if type_ref.contains("Color") {
                                // Add common color field paths
                                paths.insert(
                                    format!(".{index}.red"),
                                    "Access the red component (if Color is an enum with named fields)"
                                        .to_string(),
                                );
                                paths.insert(
                                    format!(".{index}.green"),
                                    "Access the green component (if Color is an enum with named fields)".to_string()
                                );
                                paths.insert(
                                    format!(".{index}.blue"),
                                    "Access the blue component (if Color is an enum with named fields)"
                                        .to_string(),
                                );
                                paths.insert(
                                    format!(".{index}.alpha"),
                                    "Access the alpha component (if Color is an enum with named fields)".to_string()
                                );

                                // Also add potential enum variant access
                                paths.insert(
                                    format!(".{index}.0"),
                                    "Access the first field if Color is an enum variant"
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
            }
            "Struct" => {
                // For regular structs, use property names
                if let Some(properties) = schema_data.get("properties").and_then(Value::as_object) {
                    for (field_name, _field_type) in properties {
                        paths.insert(
                            format!(".{field_name}"),
                            format!("Access the '{field_name}' field"),
                        );
                    }
                }
            }
            _ => {
                // For other types (enums, values), mutation typically replaces the whole value
                // NOTE: For enums, we don't add mutation paths here because the enum guidance
                // system in build_corrected_value_from_type_info generates better guidance
                // with valid_values and examples
                if kind == "Enum" {
                    // Skip adding mutation path for enums
                }
            }
        }

        paths
    }
}
