//! Hardcoded BRP format knowledge
//!
//! This module contains the static knowledge of how types should be serialized
//! for BRP, which often differs from their reflection-based representation.
//! This knowledge is extracted from the extras plugin's examples.rs.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::{Value, json};

use crate::brp_tools::brp_type_schema::constants::{
    TYPE_ALLOC_STRING, TYPE_BEVY_COLOR, TYPE_BEVY_IMAGE_HANDLE, TYPE_BEVY_MAT2, TYPE_BEVY_MAT3,
    TYPE_BEVY_MAT4, TYPE_BEVY_NAME, TYPE_BEVY_QUAT, TYPE_BEVY_RECT, TYPE_BEVY_VEC2, TYPE_BEVY_VEC3,
    TYPE_BEVY_VEC3A, TYPE_BEVY_VEC4, TYPE_BOOL, TYPE_CHAR, TYPE_F32, TYPE_F64, TYPE_GLAM_IVEC2,
    TYPE_GLAM_IVEC3, TYPE_GLAM_IVEC4, TYPE_GLAM_MAT2, TYPE_GLAM_MAT3, TYPE_GLAM_MAT4,
    TYPE_GLAM_QUAT, TYPE_GLAM_UVEC2, TYPE_GLAM_UVEC3, TYPE_GLAM_UVEC4, TYPE_GLAM_VEC2,
    TYPE_GLAM_VEC3, TYPE_GLAM_VEC3A, TYPE_GLAM_VEC4, TYPE_I8, TYPE_I16, TYPE_I32, TYPE_I64,
    TYPE_I128, TYPE_ISIZE, TYPE_STD_STRING, TYPE_STR, TYPE_STR_REF, TYPE_STRING, TYPE_U8, TYPE_U16,
    TYPE_U32, TYPE_U64, TYPE_U128, TYPE_USIZE,
};
use crate::brp_tools::brp_type_schema::response_types::{BrpTypeName, MathComponent};

/// Controls how mutation paths are generated for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnowledgeGuidance {
    /// Generate mutation paths normally (default behavior)
    Teach,
    /// Treat as a simple value with only root mutation, using the specified type name
    TreatAsValue { simplified_type: String },
}

/// Format knowledge key for matching types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KnowledgeKey {
    /// Exact type name match (current behavior)
    Exact(String),
    /// Generic type match - matches base type ignoring type parameters
    Generic(String),
    /// Newtype tuple variant that unwraps to inner type for mutations
    NewtypeVariant {
        /// e.g., "`bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthLoadOp`"
        enum_type:    String,
        /// e.g., "Clear"
        variant_name: String,
        /// e.g., "f32"
        inner_type:   String,
    },
    /// Struct field-specific match for providing appropriate field values
    StructField {
        /// e.g., `bevy_window::window::WindowResolution`
        struct_type: String,
        /// e.g., `physical_width`
        field_name:  String,
    },
}

impl KnowledgeKey {
    /// Create an exact match key
    pub fn exact(s: impl Into<String>) -> Self {
        Self::Exact(s.into())
    }

    /// Create a generic match key
    pub fn generic(s: impl Into<String>) -> Self {
        Self::Generic(s.into())
    }

    /// Create a newtype variant match key
    pub fn newtype_variant(
        enum_type: impl Into<String>,
        variant_name: impl Into<String>,
        inner_type: impl Into<String>,
    ) -> Self {
        Self::NewtypeVariant {
            enum_type:    enum_type.into(),
            variant_name: variant_name.into(),
            inner_type:   inner_type.into(),
        }
    }

    /// Create a struct field match key
    pub fn struct_field(struct_type: impl Into<String>, field_name: impl Into<String>) -> Self {
        Self::StructField {
            struct_type: struct_type.into(),
            field_name:  field_name.into(),
        }
    }

    /// Resolve example value using enum dispatch instead of external conditionals
    pub fn resolve_example(&self, type_name: &BrpTypeName) -> Option<Value> {
        match self {
            Self::Exact(exact_type) if exact_type == type_name.type_string() => {
                BRP_MUTATION_KNOWLEDGE
                    .get(self)
                    .map(|k| k.example().clone())
            }
            Self::Exact(_) => None, // Exact type doesn't match
            Self::Generic(generic_type) => {
                let base_type = type_name.base_type()?;
                if base_type == generic_type {
                    BRP_MUTATION_KNOWLEDGE
                        .get(self)
                        .map(|k| k.example().clone())
                } else {
                    None
                }
            }
            Self::NewtypeVariant { .. } => {
                // Newtype variant matching logic
                BRP_MUTATION_KNOWLEDGE
                    .get(self)
                    .map(|k| k.example().clone())
            }
            Self::StructField { .. } => {
                // Struct field matching is handled separately in find_example_value_for_field
                None
            }
        }
    }

    /// Try to resolve example value by iterating through all knowledge keys
    pub fn find_example_for_type(type_name: &BrpTypeName) -> Option<Value> {
        let resolvers: Vec<Box<dyn Fn() -> Option<Value>>> = vec![
            Box::new(|| Self::exact(type_name.type_string()).resolve_example(type_name)),
            Box::new(|| {
                type_name
                    .base_type()
                    .and_then(|generic| Self::generic(generic).resolve_example(type_name))
            }),
        ];

        resolvers.iter().find_map(|resolver| resolver())
    }
}

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub enum MutationKnowledge {
    /// Simple value with just an example
    Simple { example: Value },
    /// Math type with array example and named components
    MathType {
        array_example: Value,
        components:    HashMap<MathComponent, Value>,
    },
    /// Value that should be treated as opaque (no mutation paths)
    OpaqueValue {
        example:         Value,
        simplified_type: String,
    },
}

impl MutationKnowledge {
    /// Create a simple knowledge entry with no subfields
    pub const fn simple(example: Value) -> Self {
        Self::Simple { example }
    }

    /// Create a knowledge entry with math component subfields
    pub fn with_components(
        example_value: Value,
        components: impl IntoIterator<Item = (MathComponent, Value)>,
    ) -> Self {
        Self::MathType {
            array_example: example_value,
            components:    components.into_iter().collect(),
        }
    }

    /// Create a knowledge entry that should be treated as a simple value
    pub const fn as_value(example: Value, simplified_type: String) -> Self {
        Self::OpaqueValue {
            example,
            simplified_type,
        }
    }

    /// Get the example value for this knowledge
    pub const fn example(&self) -> &Value {
        match self {
            Self::MathType { array_example, .. } => array_example,
            Self::Simple { example } | Self::OpaqueValue { example, .. } => example,
        }
    }

    /// Get the example for a specific math component
    pub fn get_component_example(&self, component: MathComponent) -> Option<&Value> {
        match self {
            Self::MathType { components, .. } => components.get(&component),
            _ => None,
        }
    }

    /// Get the guidance for this knowledge
    pub fn guidance(&self) -> KnowledgeGuidance {
        match self {
            Self::Simple { .. } | Self::MathType { .. } => KnowledgeGuidance::Teach,
            Self::OpaqueValue {
                simplified_type, ..
            } => KnowledgeGuidance::TreatAsValue {
                simplified_type: simplified_type.clone(),
            },
        }
    }
}

/// Static map of hardcoded BRP format knowledge
/// This captures the serialization rules that can't be derived from registry
pub static BRP_MUTATION_KNOWLEDGE: LazyLock<HashMap<KnowledgeKey, MutationKnowledge>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // ===== Numeric types =====
        map.insert(
            KnowledgeKey::exact(TYPE_I8),
            MutationKnowledge::simple(json!(42)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I16),
            MutationKnowledge::simple(json!(1000)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I32),
            MutationKnowledge::simple(json!(100_000)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I64),
            MutationKnowledge::simple(json!(1_000_000_000_i64)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I128),
            MutationKnowledge::simple(json!("123456789012345678901234567890")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U8),
            MutationKnowledge::simple(json!(128)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U16),
            MutationKnowledge::simple(json!(5000)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U32),
            MutationKnowledge::simple(json!(1_000_000_u32)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U64),
            MutationKnowledge::simple(json!(10_000_000_000_u64)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U128),
            MutationKnowledge::simple(json!("987654321098765432109876543210")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F32),
            MutationKnowledge::simple(json!(std::f32::consts::PI)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F64),
            MutationKnowledge::simple(json!(std::f64::consts::PI)),
        );

        // ===== Size types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ISIZE),
            MutationKnowledge::simple(json!(1_000_000_i64)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_USIZE),
            MutationKnowledge::simple(json!(2_000_000_u64)),
        );

        // ===== Text types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ALLOC_STRING),
            MutationKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STD_STRING),
            MutationKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STRING),
            MutationKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR_REF),
            MutationKnowledge::simple(json!("static string")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR),
            MutationKnowledge::simple(json!("static string")),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CHAR),
            MutationKnowledge::simple(json!('A')),
        );

        // ===== Boolean =====
        map.insert(
            KnowledgeKey::exact(TYPE_BOOL),
            MutationKnowledge::simple(json!(true)),
        );

        // ===== Bevy math types (these serialize as arrays, not objects!) =====
        // Vec2
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC2),
            MutationKnowledge::with_components(
                json!([1.0, 2.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC2),
            MutationKnowledge::with_components(
                json!([1.0, 2.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                ],
            ),
        );

        // Vec3
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3A),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3A),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );

        // Vec4
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC4),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0, 4.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                    (MathComponent::W, json!(4.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC4),
            MutationKnowledge::with_components(
                json!([1.0, 2.0, 3.0, 4.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                    (MathComponent::W, json!(4.0)),
                ],
            ),
        );

        // Integer vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC2),
            MutationKnowledge::with_components(
                json!([0, 0]),
                vec![(MathComponent::X, json!(0)), (MathComponent::Y, json!(0))],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC3),
            MutationKnowledge::with_components(
                json!([0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC4),
            MutationKnowledge::with_components(
                json!([0, 0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                    (MathComponent::W, json!(0)),
                ],
            ),
        );

        // Unsigned vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC2),
            MutationKnowledge::with_components(
                json!([0, 0]),
                vec![(MathComponent::X, json!(0)), (MathComponent::Y, json!(0))],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC3),
            MutationKnowledge::with_components(
                json!([0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC4),
            MutationKnowledge::with_components(
                json!([0, 0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                    (MathComponent::W, json!(0)),
                ],
            ),
        );

        // Quaternion
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_QUAT),
            MutationKnowledge::with_components(
                json!([0.0, 0.0, 0.0, 1.0]),
                vec![
                    (MathComponent::X, json!(0.0)),
                    (MathComponent::Y, json!(0.0)),
                    (MathComponent::Z, json!(0.0)),
                    (MathComponent::W, json!(1.0)),
                ],
            ),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_QUAT),
            MutationKnowledge::with_components(
                json!([0.0, 0.0, 0.0, 1.0]),
                vec![
                    (MathComponent::X, json!(0.0)),
                    (MathComponent::Y, json!(0.0)),
                    (MathComponent::Z, json!(0.0)),
                    (MathComponent::W, json!(1.0)),
                ],
            ),
        );

        // Matrices
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT2),
            MutationKnowledge::simple(json!([[1.0, 0.0], [0.0, 1.0]])), /* Matrices don't have
                                                                         * simple component
                                                                         * access */
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT2),
            MutationKnowledge::simple(json!([[1.0, 0.0], [0.0, 1.0]])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT3),
            MutationKnowledge::simple(json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT3),
            MutationKnowledge::simple(json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT4),
            MutationKnowledge::simple(json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT4),
            MutationKnowledge::simple(json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])),
        );

        // ===== Bevy math Rect =====
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_RECT),
            MutationKnowledge::simple(json!({
                "min": [0.0, 0.0],
                "max": [100.0, 100.0]
            })), // Has nested paths via Vec2 fields
        );

        // ===== Bevy color types =====

        // Color enum - tuple variants with flat array of RGBA values
        // Note: BRP mutations expect [r, g, b, a] array, not the struct wrapper
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_COLOR),
            MutationKnowledge::simple(json!({"Srgba": [1.0, 0.0, 0.0, 1.0]})),
        );

        // ===== Collections =====
        map.insert(
            KnowledgeKey::generic("alloc::vec::Vec"),
            MutationKnowledge::simple(json!([])), /* Collections have index access, not
                                                   * component access */
        );
        map.insert(
            KnowledgeKey::generic("std::collections::BTreeMap"),
            MutationKnowledge::simple(json!({})),
        );

        // ===== Bevy ECS types =====
        // Entity - serializes as u64 (entity.to_bits()), not as struct
        // WARNING: This is just an example! For actual BRP operations, use VALID entity IDs
        // obtained from spawn operations or queries. Using invalid entity IDs will cause errors.
        map.insert(
            KnowledgeKey::exact("bevy_ecs::entity::Entity"),
            MutationKnowledge::as_value(json!(8589934670_u64), "Entity".to_string()),
        );

        // Name serializes as a plain string, not as a struct with hash/name fields
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_NAME),
            MutationKnowledge::as_value(json!("Entity Name"), "String".to_string()),
        );

        // ===== Camera3d depth texture usage =====
        // Camera3dDepthTextureUsage - wrapper around u32 texture usage flags
        // Valid flags: COPY_SRC=1, COPY_DST=2, TEXTURE_BINDING=4, STORAGE_BINDING=8,
        // RENDER_ATTACHMENT=16 STORAGE_BINDING (8) causes crashes with multisampled
        // textures! Safe combinations: 16 (RENDER_ATTACHMENT only), 20 (RENDER_ATTACHMENT |
        // TEXTURE_BINDING)
        map.insert(
            KnowledgeKey::struct_field(
                "bevy_core_pipeline::core_3d::camera_3d::Camera3d",
                "depth_texture_usages",
            ),
            // RENDER_ATTACHMENT | TEXTURE_BINDING - safe combination, treat as opaque u32
            MutationKnowledge::as_value(json!(20), "u32".to_string()),
        );

        // ===== Transform types =====
        // GlobalTransform - wraps glam::Affine3A but serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Registry shows nested object but BRP actually expects flat array
        map.insert(
            KnowledgeKey::exact("bevy_transform::components::global_transform::GlobalTransform"),
            MutationKnowledge::simple(json!([
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0
            ])), // Affine matrices don't have simple component access
        );

        // ===== Asset Handle types =====
        // Handle<T> types - use Weak variant with UUID format for mutations
        // Schema provides non-functional examples, but this format works
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_IMAGE_HANDLE),
            MutationKnowledge::simple(
                json!({"Weak": {"Uuid": {"uuid": "12345678-1234-1234-1234-123456789012"}}}),
            ),
        );

        // ===== Camera3d depth load operation =====
        // Camera3d depth clear value - must be in range [0.0, 1.0] for valid GPU operations
        map.insert(
            KnowledgeKey::newtype_variant(
                "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthLoadOp",
                "Clear",
                "f32",
            ),
            MutationKnowledge::simple(json!(0.5)), // Safe middle value in [0.0, 1.0] range
        );

        // ===== WindowResolution field-specific values =====
        // Provide reasonable window dimension values to prevent GPU texture size errors
        map.insert(
            KnowledgeKey::struct_field("bevy_window::window::WindowResolution", "physical_width"),
            MutationKnowledge::simple(json!(800)), // Reasonable window width
        );
        map.insert(
            KnowledgeKey::struct_field("bevy_window::window::WindowResolution", "physical_height"),
            MutationKnowledge::simple(json!(600)), // Reasonable window height
        );

        map
    });
