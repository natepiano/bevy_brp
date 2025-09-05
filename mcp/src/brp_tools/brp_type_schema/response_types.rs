//! Public API response types for the `brp_type_schema` tool
//!
//! This module contains the strongly-typed structures that form the public API
//! for type schema discovery results. These types are separate from the internal
//! processing types to provide a clean, stable API contract.

use std::collections::HashMap;
use std::fmt::Display;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString};

use super::constants::{RecursionDepth, SCHEMA_REF_PREFIX};
use super::mutation_knowledge::{BRP_MUTATION_KNOWLEDGE, KnowledgeKey};
use super::mutation_path_builder::TypeKind;
use super::type_info::TypeInfo;
use crate::brp_tools::brp_type_schema::constants::MAX_TYPE_RECURSION_DEPTH;
use crate::string_traits::JsonFieldAccess;

/// Enum for BRP supported operations
/// Each operation has specific requirements based on type registration and traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BrpSupportedOperation {
    /// Get operation - requires type in registry
    Get,
    /// Insert operation - requires Serialize + Deserialize traits
    Insert,
    /// Mutate operation - requires mutable type (struct/tuple)
    Mutate,
    /// Query operation - requires type in registry
    Query,
    /// Spawn operation - requires Serialize + Deserialize traits
    Spawn,
}

impl From<BrpSupportedOperation> for String {
    fn from(op: BrpSupportedOperation) -> Self {
        op.as_ref().to_string()
    }
}

/// A newtype wrapper for BRP type names used as `HashMap` keys
///
/// This type provides documentation and type safety for strings that represent
/// fully-qualified Rust type names (e.g., "`bevy_transform::components::transform::Transform`")
/// when used as keys in type information maps.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct BrpTypeName(String);

impl BrpTypeName {
    /// Create a `BrpTypeName` representing an unknown type
    ///
    /// This is commonly used as a fallback when type information
    /// is not available or cannot be determined.
    pub fn unknown() -> Self {
        Self("unknown".to_string())
    }

    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the type string for comparison
    /// This is an alias for `as_str()` but with clearer intent
    pub fn type_string(&self) -> &str {
        &self.0
    }

    /// Extract the base type name by stripping generic parameters
    /// For example: `Vec<String>` returns `Some("Vec")`
    pub fn base_type(&self) -> Option<&str> {
        self.0.split('<').next()
    }
}

impl From<&str> for BrpTypeName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for BrpTypeName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&String> for BrpTypeName {
    fn from(s: &String) -> Self {
        Self(s.clone())
    }
}

impl From<BrpTypeName> for String {
    fn from(type_name: BrpTypeName) -> Self {
        type_name.0
    }
}

impl From<&BrpTypeName> for String {
    fn from(type_name: &BrpTypeName) -> Self {
        type_name.0.clone()
    }
}

impl std::fmt::Display for BrpTypeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<BrpTypeName> for Value {
    fn from(type_name: BrpTypeName) -> Self {
        Self::String(type_name.0)
    }
}

impl From<&BrpTypeName> for Value {
    fn from(type_name: &BrpTypeName) -> Self {
        Self::String(type_name.0.clone())
    }
}

/// Schema information extracted from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    /// Category of the type (Struct, Enum, etc.) from registry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_kind:   Option<TypeKind>,
    /// Field definitions from the registry schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties:  Option<Value>,
    /// Required fields list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required:    Option<Vec<String>>,
    /// Module path of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    /// Crate name of the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crate_name:  Option<String>,
}

/// Information about a field in an enum struct variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumFieldInfo {
    /// Field name
    pub field_name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_name:  BrpTypeName,
}

/// Enum variant access patterns for building mutation paths
#[derive(Debug, Clone)]
pub enum VariantAccess {
    /// Tuple element access via index (e.g., `.0`, `.1`)
    TupleIndex(usize),
    /// Struct field access via field name (e.g., `.field_name`)
    StructField(String),
}

impl VariantAccess {}

/// Variant signatures for deduplication - same signature means same inner structure
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum VariantSignature {
    /// Unit variants (no data)
    Unit,
    /// Tuple variants with specified types
    Tuple(Vec<BrpTypeName>),
    /// Struct variants with field names and types
    Struct(Vec<(String, BrpTypeName)>),
}

/// Type-safe enum variant information - replaces `EnumVariantInfoOld`
/// This enum makes invalid states impossible to construct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnumVariantInfo {
    /// Unit variant - just the variant name
    Unit(String),
    /// Tuple variant - name and guaranteed tuple types
    Tuple(String, Vec<BrpTypeName>),
    /// Struct variant - name and guaranteed struct fields
    Struct(String, Vec<EnumFieldInfo>),
}

impl EnumVariantInfo {
    /// Get the variant name regardless of variant type
    pub fn name(&self) -> &str {
        match self {
            Self::Unit(name) | Self::Tuple(name, _) | Self::Struct(name, _) => name,
        }
    }

    /// Extract inner types and their access methods from this variant
    /// Returns empty vector for unit variants, tuple indices for tuple variants,
    /// and field names for struct variants
    pub fn inner_types(&self) -> Vec<(BrpTypeName, VariantAccess)> {
        match self {
            Self::Unit(_) => Vec::new(),
            Self::Tuple(_, types) => types
                .iter()
                .enumerate()
                .map(|(index, type_name)| (type_name.clone(), VariantAccess::TupleIndex(index)))
                .collect(),
            Self::Struct(_, fields) => fields
                .iter()
                .map(|field| {
                    (
                        field.type_name.clone(),
                        VariantAccess::StructField(field.field_name.clone()),
                    )
                })
                .collect(),
        }
    }

    /// Get the signature of this variant for deduplication
    /// Unit variants return None, tuple variants return type list,
    /// struct variants return field name/type pairs
    pub fn signature(&self) -> VariantSignature {
        match self {
            Self::Unit(_) => VariantSignature::Unit,
            Self::Tuple(_, types) => VariantSignature::Tuple(types.clone()),
            Self::Struct(_, fields) => {
                let field_sig = fields
                    .iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                VariantSignature::Struct(field_sig)
            }
        }
    }

    /// Constructor that infers variant type from JSON structure
    /// instead of relying on separate enum classification
    pub fn from_schema_variant(
        v: &Value,
        registry: &HashMap<BrpTypeName, Value>,
        depth: usize,
    ) -> Option<Self> {
        let name = extract_variant_name(v)?;

        // Infer variant type from JSON structure, not from string parsing
        if v.is_string() {
            Some(Self::Unit(name))
        } else if let Some(prefix_items) = v
            .get_field(SchemaField::PrefixItems)
            .and_then(Value::as_array)
        {
            let types = extract_tuple_types(prefix_items, registry, depth);
            Some(Self::Tuple(name, types))
        } else if let Some(properties) = v
            .get_field(SchemaField::Properties)
            .and_then(Value::as_object)
        {
            let fields = extract_struct_fields(properties, registry, depth);
            Some(Self::Struct(name, fields))
        } else {
            Some(Self::Unit(name)) // Default fallback
        }
    }

    /// Build example JSON for this enum variant
    pub fn build_example(
        &self,
        registry: &HashMap<BrpTypeName, Value>,
        depth: usize,
        enum_type: Option<&BrpTypeName>,
    ) -> Value {
        match self {
            Self::Unit(name) => {
                // NEW: Check for variant-specific knowledge first
                if let Some(enum_type) = enum_type {
                    let variant_key = KnowledgeKey::enum_variant(enum_type.type_string(), name);

                    if let Some(knowledge) = BRP_MUTATION_KNOWLEDGE.get(&variant_key) {
                        return knowledge.example_value().clone();
                    }
                }
                // Fall back to default Unit variant behavior
                serde_json::json!(name)
            }
            Self::Tuple(name, types) => {
                let tuple_values: Vec<Value> = types
                    .iter()
                    .map(|t| {
                        TypeInfo::build_example_value_for_type_with_depth(
                            t,
                            registry,
                            RecursionDepth::from_usize(depth).increment(),
                        )
                    }) // FIXED: Use depth-aware version with recursion tracking
                    .collect();
                // For single-element tuples (newtype pattern), unwrap the single value
                // For multi-element tuples, use array format
                let content = if tuple_values.len() == 1 {
                    // Safe: we just checked length is 1, so index 0 exists
                    tuple_values[0].clone()
                } else {
                    serde_json::Value::Array(tuple_values)
                };
                serde_json::json!({ name: content })
            }
            Self::Struct(name, fields) => {
                let struct_obj: serde_json::Map<String, Value> = fields
                    .iter()
                    .map(|f| {
                        (
                            f.field_name.clone(),
                            TypeInfo::build_example_value_for_type_with_depth(
                                &f.type_name,
                                registry,
                                RecursionDepth::from_usize(depth).increment(),
                            ), // FIXED: Use depth-aware version with recursion tracking
                        )
                    })
                    .collect();
                serde_json::json!({ name: struct_obj })
            }
        }
    }
}

/// Helper function to extract variant name from schema variant
fn extract_variant_name(v: &Value) -> Option<String> {
    // For unit variants, the value is just a string
    if let Value::String(s) = v {
        return Some(s.clone());
    }

    // For tuple/struct variants, look for the shortPath field
    v.get_field(SchemaField::ShortPath)
        .and_then(Value::as_str)
        .map(String::from)
}

/// Helper function to check if recursion depth exceeds the maximum allowed
fn check_depth_exceeded(depth: usize, operation: &str) -> bool {
    if depth > MAX_TYPE_RECURSION_DEPTH {
        tracing::warn!("Max recursion depth reached while {operation}, using fallback");
        true
    } else {
        false
    }
}

/// Create a fallback type for when depth is exceeded
fn create_fallback_type() -> BrpTypeName {
    BrpTypeName::from("f32")
}

/// Create a fallback field for struct variants when depth is exceeded
fn create_fallback_field() -> EnumFieldInfo {
    EnumFieldInfo {
        field_name: "value".to_string(),
        type_name:  create_fallback_type(),
    }
}

/// Helper function to extract tuple types from prefixItems with depth control
/// This prevents stack overflow when processing deeply nested tuple structures
fn extract_tuple_types(
    prefix_items: &[Value],
    _registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<BrpTypeName> {
    if check_depth_exceeded(depth, "extracting tuple types") {
        return vec![create_fallback_type()];
    }

    prefix_items
        .iter()
        .filter_map(|item| {
            item.get_field(SchemaField::Type)
                .and_then(|t| t.get_field(SchemaField::Ref))
                .and_then(Value::as_str)
                .and_then(|s| s.strip_prefix(SCHEMA_REF_PREFIX))
                .map(BrpTypeName::from)
        })
        .collect()
}

/// Helper function to extract struct fields from properties with depth control
/// This prevents stack overflow when processing deeply nested struct structures
fn extract_struct_fields(
    properties: &serde_json::Map<String, Value>,
    _registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<EnumFieldInfo> {
    if check_depth_exceeded(depth, "extracting struct fields") {
        return vec![create_fallback_field()];
    }

    properties
        .iter()
        .filter_map(|(field_name, field_schema)| {
            SchemaField::extract_field_type(field_schema).map(|type_name| EnumFieldInfo {
                field_name: field_name.clone(),
                type_name,
            })
        })
        .collect()
}

/// Build all enum examples - generates one example per unique variant type signature
pub fn build_all_enum_examples(
    schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
    enum_type: Option<&BrpTypeName>, // ADD enum_type parameter
) -> HashMap<String, Value> {
    let variants = extract_enum_variants(schema, registry, depth);

    // Group variants by their type signature and generate one example per group
    let mut examples = HashMap::new();
    let mut seen_unit = false;
    let mut seen_tuples: HashMap<Vec<BrpTypeName>, String> = HashMap::new();
    let mut seen_structs: HashMap<Vec<(String, BrpTypeName)>, String> = HashMap::new();

    for variant in variants {
        match &variant {
            EnumVariantInfo::Unit(name) => {
                if !seen_unit {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    seen_unit = true;
                }
            }
            EnumVariantInfo::Tuple(name, types) => {
                if !seen_tuples.contains_key(types) {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    seen_tuples.insert(types.clone(), name.clone());
                }
            }
            EnumVariantInfo::Struct(name, fields) => {
                let field_sig: Vec<(String, BrpTypeName)> = fields
                    .iter()
                    .map(|f| (f.field_name.clone(), f.type_name.clone()))
                    .collect();
                if let std::collections::hash_map::Entry::Vacant(e) = seen_structs.entry(field_sig)
                {
                    let example = variant.build_example(registry, depth, enum_type); // Pass both
                    examples.insert(name.clone(), example);
                    e.insert(name.clone());
                }
            }
        }
    }

    examples
}

/// Deduplicate variants by signature, returning first variant of each unique signature
/// This prevents redundant processing when multiple variants have the same type structure
pub fn deduplicate_variant_signatures(variants: Vec<EnumVariantInfo>) -> Vec<EnumVariantInfo> {
    use std::collections::HashSet;

    let mut seen_signatures = HashSet::new();
    let mut unique_variants = Vec::new();

    for variant in variants {
        let signature = variant.signature();
        if seen_signatures.insert(signature) {
            unique_variants.push(variant);
        }
    }

    unique_variants
}

/// Extract enum variants using the new `EnumVariantInfo` enum
pub fn extract_enum_variants(
    type_schema: &Value,
    registry: &HashMap<BrpTypeName, Value>,
    depth: usize,
) -> Vec<EnumVariantInfo> {
    type_schema
        .get_field(SchemaField::OneOf)
        .and_then(Value::as_array)
        .map(|variants| {
            variants
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    EnumVariantInfo::from_schema_variant(v, registry, depth)
                        .or_else(|| {
                            tracing::warn!("Failed to parse enum variant {i} in schema - this is unexpected as BRP should provide valid variants");
                            None
                        })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Math type component names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum MathComponent {
    X,
    Y,
    Z,
    W,
}

impl From<MathComponent> for String {
    fn from(component: MathComponent) -> Self {
        component.as_ref().to_string()
    }
}

impl TryFrom<&str> for MathComponent {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            "w" => Ok(Self::W),
            _ => Err(()),
        }
    }
}

/// Status of whether a mutation path can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Path can be fully mutated
    Mutatable,
    /// Path cannot be mutated (missing traits, unsupported type, etc.)
    NotMutatable,
    /// Path is partially mutatable (some elements mutable, others not)
    PartiallyMutatable,
}

/// Context for a mutation path describing what kind of mutation this is
#[derive(Debug, Clone, Deserialize)]
pub enum MutationPathKind {
    /// Replace the entire value (root mutation with empty path)
    RootValue { type_name: BrpTypeName },
    /// Mutate a field in a struct
    StructField {
        field_name:  String,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in a tuple by index
    /// Applies to tuple elements, enums variants, including generics such as Option<T>
    IndexedElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
    /// Mutate an element in an array
    ArrayElement {
        index:       usize,
        parent_type: BrpTypeName,
    },
}

impl MutationPathKind {
    /// Generate a human-readable description for this mutation
    pub fn description(&self) -> String {
        match self {
            Self::RootValue { type_name } => {
                format!("Replace the entire {type_name} value")
            }
            Self::StructField {
                field_name,
                parent_type,
            } => {
                format!("Mutate the {field_name} field of {parent_type}")
            }
            Self::IndexedElement { index, parent_type } => {
                format!("Mutate element {index} of {parent_type}")
            }
            Self::ArrayElement { index, parent_type } => {
                format!("Mutate element [{index}] of {parent_type}")
            }
        }
    }

    /// Get just the variant name for serialization
    pub const fn variant_name(&self) -> &'static str {
        match self {
            Self::RootValue { .. } => "RootValue",
            Self::StructField { .. } => "StructField",
            Self::IndexedElement { .. } => "TupleElement",
            Self::ArrayElement { .. } => "ArrayElement",
        }
    }
}

impl Display for MutationPathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.variant_name())
    }
}

impl Serialize for MutationPathKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Mutation path information (internal representation)
#[derive(Debug, Clone)]
pub struct MutationPathInternal {
    /// Example value for this path
    pub example:         Value,
    /// Path for mutation, e.g., ".translation.x"
    pub path:            String,
    /// For enum types, list of valid variant names
    pub enum_variants:   Option<Vec<String>>,
    /// Type information for this path
    pub type_name:       BrpTypeName,
    /// Context describing what kind of mutation this is
    pub path_kind:       MutationPathKind,
    /// Status of whether this path can be mutated
    pub mutation_status: MutationStatus,
    /// Error reason if mutation is not possible
    pub error_reason:    Option<String>,
}

/// Path information combining navigation and type metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    /// Context describing what kind of mutation this is (how to navigate to this path)
    pub path_kind: MutationPathKind,
    /// Fully-qualified type name of the field
    #[serde(rename = "type")]
    pub type_name: BrpTypeName,
    /// The kind of type this field contains (Struct, Enum, Array, etc.)
    pub type_kind: TypeKind,
}

/// Information about a mutation path that we serialize to our response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationPath {
    /// Human-readable description of what this path mutates
    pub description:      String,
    /// Combined path navigation and type metadata
    pub path_info:        PathInfo,
    /// Status of whether this path can be mutated
    pub mutation_status:  MutationStatus,
    /// Error reason if mutation is not possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason:     Option<String>,
    /// Example value for mutations (for non-Option types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example:          Option<Value>,
    /// Example value for setting Some variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_some:     Option<Value>,
    /// Example value for setting None variant (Option types only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_none:     Option<Value>,
    /// List of valid enum variants for this field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_variants:    Option<Vec<String>>,
    /// Example values for enum variants (maps variant names to example JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_variants: Option<HashMap<String, Value>>,
    /// Additional note about how to use this mutation path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note:             Option<String>,
}

impl MutationPath {
    /// Create a root value mutation with a simplified type name
    pub fn new_root_value(
        type_name: BrpTypeName,
        example_value: Value,
        simplified_type: String,
    ) -> Self {
        Self {
            description:      format!("Replace the entire {type_name} value"),
            path_info:        PathInfo {
                path_kind: MutationPathKind::RootValue {
                    type_name: type_name.clone(),
                },
                type_name: BrpTypeName::from(simplified_type),
                type_kind: TypeKind::Value, // Root values are treated as Value types
            },
            example:          Some(example_value),
            example_some:     None,
            example_none:     None,
            enum_variants:    None,
            example_variants: None,
            note:             None,
            mutation_status:  MutationStatus::Mutatable,
            error_reason:     None,
        }
    }

    /// Create from internal `MutationPath` with proper formatting logic
    pub fn from_mutation_path(
        path: &MutationPathInternal,
        description: String,
        is_option: bool,
        type_schema: &Value,
        registry: &HashMap<BrpTypeName, Value>,
    ) -> Self {
        if is_option {
            // For Option types, check if we have the special format
            if let Some(some_val) = path.example.get_field(OptionField::Some)
                && let Some(none_val) = path.example.get_field(OptionField::None)
            {
                // Get TypeKind for the field type
                let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
                let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

                return Self {
                    description,
                    path_info: PathInfo {
                        path_kind: path.path_kind.clone(),
                        type_name: path.type_name.clone(),
                        type_kind,
                    },
                    example: None,
                    example_some: Some(some_val.clone()),
                    example_none: Some(none_val.clone()),
                    enum_variants: path.enum_variants.clone(),
                    example_variants: None, // Options don't use enum examples
                    note: Some(
                        "For Option fields: pass the value directly to set Some, null to set None"
                            .to_string(),
                    ),
                    mutation_status: path.mutation_status,
                    error_reason: path.error_reason.clone(),
                };
            }
        }

        // Regular non-Option path
        let example_variants = if path.enum_variants.is_some() {
            // This is an enum type - generate example variants using the new system
            let enum_type = Some(&path.type_name); // Extract enum type from path
            let examples = build_all_enum_examples(type_schema, registry, 0, enum_type); // Pass both
            if examples.is_empty() {
                None
            } else {
                Some(examples)
            }
        } else {
            None
        };

        // Compute enum_variants from example_variants keys (alphabetically sorted)
        let enum_variants = example_variants.as_ref().map(|variants| {
            let mut keys: Vec<String> = variants.keys().cloned().collect();
            keys.sort(); // Alphabetical sorting for consistency
            keys
        });

        // Get TypeKind for the field type
        let field_schema = registry.get(&path.type_name).unwrap_or(&Value::Null);
        let type_kind = TypeKind::from_schema(field_schema, &path.type_name);

        Self {
            description,
            path_info: PathInfo {
                path_kind: path.path_kind.clone(),
                type_name: path.type_name.clone(),
                type_kind,
            },
            example: if path.example.is_null() {
                None
            } else {
                Some(path.example.clone())
            },
            example_some: None,
            example_none: None,
            enum_variants,
            example_variants,
            note: None,
            mutation_status: path.mutation_status,
            error_reason: path.error_reason.clone(),
        }
    }
}

/// Option field keys for JSON representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum OptionField {
    Some,
    None,
}

impl From<OptionField> for String {
    fn from(field: OptionField) -> Self {
        field.as_ref().to_string()
    }
}

/// Bevy reflection trait names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum ReflectTrait {
    Component,
    Resource,
    Serialize,
    Deserialize,
}

/// Registry schema field names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr)]
#[strum(serialize_all = "camelCase")]
pub enum SchemaField {
    CrateName,
    Items,
    Kind,
    ModulePath,
    OneOf,
    PrefixItems,
    Properties,
    #[strum(serialize = "$ref")]
    Ref,
    ReflectTypes,
    Required,
    ShortPath,
    Type,
}

impl SchemaField {
    /// Extract field type from field info JSON
    ///
    /// This extracts the type reference from a field definition in the schema,
    /// handling the standard pattern of type.$ref with #/$defs/ prefix.
    pub fn extract_field_type(field_info: &Value) -> Option<BrpTypeName> {
        field_info
            .get_field(Self::Type)
            .and_then(|t| t.get_field(Self::Ref))
            .and_then(Value::as_str)
            .and_then(|ref_str| ref_str.strip_prefix(SCHEMA_REF_PREFIX))
            .map(BrpTypeName::from)
    }
}

/// response structure
#[derive(Debug, Clone, Serialize)]
pub struct TypeSchemaResponse {
    /// Number of types successfully discovered
    pub discovered_count: usize,
    /// List of type names that were requested
    pub requested_types:  Vec<String>,
    /// Summary statistics for the discovery operation
    pub summary:          TypeSchemaSummary,
    /// Detailed information for each type, keyed by type name
    pub type_info:        HashMap<BrpTypeName, TypeInfo>,
}

/// Summary statistics for the discovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchemaSummary {
    /// Number of types that failed discovery
    pub failed_discoveries:     usize,
    /// Number of types successfully discovered
    pub successful_discoveries: usize,
    /// Total number of types requested
    pub total_requested:        usize,
}
