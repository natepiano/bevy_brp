//! Hardcoded BRP format knowledge
//!
//! This module contains the static knowledge of how types should be serialized
//! for BRP, which often differs from their reflection-based representation.
//! This knowledge is extracted from the extras plugin's examples.rs.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::{Value, json};

use super::response_types::BrpTypeName;

use super::constants::{
    TYPE_ALLOC_STRING, TYPE_BEVY_COLOR, TYPE_BEVY_IMAGE_HANDLE, TYPE_BEVY_MAT2, TYPE_BEVY_MAT3,
    TYPE_BEVY_MAT4, TYPE_BEVY_NAME, TYPE_BEVY_QUAT, TYPE_BEVY_RECT, TYPE_BEVY_VEC2, TYPE_BEVY_VEC3,
    TYPE_BEVY_VEC3A, TYPE_BEVY_VEC4, TYPE_BOOL, TYPE_CHAR, TYPE_F32, TYPE_F64, TYPE_GLAM_IVEC2,
    TYPE_GLAM_IVEC3, TYPE_GLAM_IVEC4, TYPE_GLAM_MAT2, TYPE_GLAM_MAT3, TYPE_GLAM_MAT4,
    TYPE_GLAM_QUAT, TYPE_GLAM_UVEC2, TYPE_GLAM_UVEC3, TYPE_GLAM_UVEC4, TYPE_GLAM_VEC2,
    TYPE_GLAM_VEC3, TYPE_GLAM_VEC3A, TYPE_GLAM_VEC4, TYPE_I8, TYPE_I16, TYPE_I32, TYPE_I64,
    TYPE_I128, TYPE_ISIZE, TYPE_STD_STRING, TYPE_STR, TYPE_STR_REF, TYPE_STRING, TYPE_U8, TYPE_U16,
    TYPE_U32, TYPE_U64, TYPE_U128, TYPE_USIZE,
};
use super::response_types::MathComponent;

/// Controls how mutation paths are generated for a type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Knowledge {
    /// Generate mutation paths normally (default behavior)
    Teach,
    /// Treat as a simple value with only root mutation, using the specified type name
    TreatAsValue { simplified_type: String },
}

/// Format knowledge key for matching types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FormatKnowledgeKey {
    /// Exact type name match (current behavior)
    Exact(String),
    /// Generic type match - matches base type ignoring type parameters
    Generic(String),
    /// Enum variant-specific match for context-aware examples
    EnumVariant {
        /// e.g., "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthLoadOp"
        enum_type: String,
        /// e.g., "Clear"
        variant_name: String,
        /// e.g., "Clear(f32)" - matches tuple variants with specific types
        variant_pattern: String,
    },
}

impl FormatKnowledgeKey {
    /// Create an exact match key
    pub fn exact(s: impl Into<String>) -> Self {
        Self::Exact(s.into())
    }

    /// Create a generic match key
    pub fn generic(s: impl Into<String>) -> Self {
        Self::Generic(s.into())
    }

    /// Resolve example value using enum dispatch instead of external conditionals
    pub fn resolve_example_value(&self, type_name: &BrpTypeName) -> Option<Value> {
        match self {
            Self::Exact(exact_type) if exact_type == type_name.as_str() => {
                BRP_FORMAT_KNOWLEDGE.get(self).map(|k| k.example_value.clone())
            }
            Self::Exact(_) => None, // Exact type doesn't match
            Self::Generic(generic_type) => {
                let base_type = type_name.as_str().split('<').next()?;
                if base_type == generic_type {
                    BRP_FORMAT_KNOWLEDGE.get(self).map(|k| k.example_value.clone())
                } else {
                    None
                }
            }
            Self::EnumVariant { .. } => {
                // Context-aware matching logic for enum variants
                BRP_FORMAT_KNOWLEDGE.get(self).map(|k| k.example_value.clone())
            }
        }
    }

    /// Try to resolve example value by iterating through all knowledge keys
    pub fn find_example_value_for_type(type_name: &BrpTypeName) -> Option<Value> {
        // Try exact match first
        if let Some(value) = FormatKnowledgeKey::exact(type_name.as_str()).resolve_example_value(type_name) {
            return Some(value);
        }
        
        // Try generic match by stripping type parameters
        if let Some(generic_type) = type_name.as_str().split('<').next() {
            if let Some(value) = FormatKnowledgeKey::generic(generic_type).resolve_example_value(type_name) {
                return Some(value);
            }
        }
        
        None
    }
}

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub struct BrpFormatKnowledge {
    /// Example value in the correct BRP format
    pub example_value:      Value,
    /// Subfield paths for types that support subfield mutation (e.g., Vec3 has x,y,z)
    /// Each tuple is (`component_name`, `example_value`)
    pub subfield_paths:     Option<Vec<(MathComponent, Value)>>,
    /// Controls mutation path generation behavior
    pub mutation_knowledge: Knowledge,
}

impl BrpFormatKnowledge {
    /// Create a simple knowledge entry with no subfields
    pub const fn simple(example_value: Value) -> Self {
        Self {
            example_value,
            subfield_paths: None,
            mutation_knowledge: Knowledge::Teach,
        }
    }

    /// Create a knowledge entry with math component subfields
    pub const fn with_components(
        example_value: Value,
        components: Vec<(MathComponent, Value)>,
    ) -> Self {
        Self {
            example_value,
            subfield_paths: Some(components),
            mutation_knowledge: Knowledge::Teach,
        }
    }

    /// Create a knowledge entry that should be treated as a simple value
    pub const fn as_value(example_value: Value, simplified_type: String) -> Self {
        Self {
            example_value,
            subfield_paths: None,
            mutation_knowledge: Knowledge::TreatAsValue { simplified_type },
        }
    }
}

/// Static map of hardcoded BRP format knowledge
/// This captures the serialization rules that can't be derived from registry
pub static BRP_FORMAT_KNOWLEDGE: LazyLock<HashMap<FormatKnowledgeKey, BrpFormatKnowledge>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // ===== Numeric types =====
        map.insert(
            FormatKnowledgeKey::exact(TYPE_I8),
            BrpFormatKnowledge::simple(json!(-128)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_I16),
            BrpFormatKnowledge::simple(json!(-32768)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_I32),
            BrpFormatKnowledge::simple(json!(-2_147_483_648)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_I64),
            BrpFormatKnowledge::simple(json!(-9_223_372_036_854_775_808_i64)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_I128),
            BrpFormatKnowledge::simple(json!("-170141183460469231731687303715884105728")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_U8),
            BrpFormatKnowledge::simple(json!(255)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_U16),
            BrpFormatKnowledge::simple(json!(65535)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_U32),
            BrpFormatKnowledge::simple(json!(4_294_967_295_u32)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_U64),
            BrpFormatKnowledge::simple(json!(18_446_744_073_709_551_615_u64)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_U128),
            BrpFormatKnowledge::simple(json!("340282366920938463463374607431768211455")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_F32),
            BrpFormatKnowledge::simple(json!(std::f32::consts::PI)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_F64),
            BrpFormatKnowledge::simple(json!(std::f64::consts::PI)),
        );

        // ===== Size types =====
        map.insert(
            FormatKnowledgeKey::exact(TYPE_ISIZE),
            BrpFormatKnowledge::simple(json!(-9_223_372_036_854_775_808_i64)),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_USIZE),
            BrpFormatKnowledge::simple(json!(18_446_744_073_709_551_615_u64)),
        );

        // ===== Text types =====
        map.insert(
            FormatKnowledgeKey::exact(TYPE_ALLOC_STRING),
            BrpFormatKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_STD_STRING),
            BrpFormatKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_STRING),
            BrpFormatKnowledge::simple(json!("Hello, World!")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_STR_REF),
            BrpFormatKnowledge::simple(json!("static string")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_STR),
            BrpFormatKnowledge::simple(json!("static string")),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_CHAR),
            BrpFormatKnowledge::simple(json!('A')),
        );

        // ===== Boolean =====
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BOOL),
            BrpFormatKnowledge::simple(json!(true)),
        );

        // ===== Bevy math types (these serialize as arrays, not objects!) =====
        // Vec2
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_VEC2),
            BrpFormatKnowledge::with_components(
                json!([1.0, 2.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_VEC2),
            BrpFormatKnowledge::with_components(
                json!([1.0, 2.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                ],
            ),
        );

        // Vec3
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_VEC3),
            BrpFormatKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_VEC3A),
            BrpFormatKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_VEC3),
            BrpFormatKnowledge::with_components(
                json!([1.0, 2.0, 3.0]),
                vec![
                    (MathComponent::X, json!(1.0)),
                    (MathComponent::Y, json!(2.0)),
                    (MathComponent::Z, json!(3.0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_VEC3A),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_BEVY_VEC4),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_GLAM_VEC4),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_GLAM_IVEC2),
            BrpFormatKnowledge::with_components(
                json!([0, 0]),
                vec![(MathComponent::X, json!(0)), (MathComponent::Y, json!(0))],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_IVEC3),
            BrpFormatKnowledge::with_components(
                json!([0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_IVEC4),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_GLAM_UVEC2),
            BrpFormatKnowledge::with_components(
                json!([0, 0]),
                vec![(MathComponent::X, json!(0)), (MathComponent::Y, json!(0))],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_UVEC3),
            BrpFormatKnowledge::with_components(
                json!([0, 0, 0]),
                vec![
                    (MathComponent::X, json!(0)),
                    (MathComponent::Y, json!(0)),
                    (MathComponent::Z, json!(0)),
                ],
            ),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_UVEC4),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_BEVY_QUAT),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_GLAM_QUAT),
            BrpFormatKnowledge::with_components(
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
            FormatKnowledgeKey::exact(TYPE_BEVY_MAT2),
            BrpFormatKnowledge {
                example_value:      json!([[1.0, 0.0], [0.0, 1.0]]),
                subfield_paths:     None, // Matrices don't have simple component access
                mutation_knowledge: Knowledge::Teach,
            },
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_MAT2),
            BrpFormatKnowledge::simple(json!([[1.0, 0.0], [0.0, 1.0]])),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_MAT3),
            BrpFormatKnowledge::simple(json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_MAT3),
            BrpFormatKnowledge::simple(json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_MAT4),
            BrpFormatKnowledge::simple(json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])),
        );
        map.insert(
            FormatKnowledgeKey::exact(TYPE_GLAM_MAT4),
            BrpFormatKnowledge::simple(json!([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ])),
        );

        // ===== Bevy math Rect =====
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_RECT),
            BrpFormatKnowledge {
                example_value:      json!({
                    "min": [0.0, 0.0],
                    "max": [100.0, 100.0]
                }),
                subfield_paths:     None, // Has nested paths via Vec2 fields
                mutation_knowledge: Knowledge::Teach,
            },
        );

        // ===== Bevy color types =====

        // Color enum - tuple variants with flat array of RGBA values
        // Note: BRP mutations expect [r, g, b, a] array, not the struct wrapper
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_COLOR),
            BrpFormatKnowledge::simple(json!({"Srgba": [1.0, 0.0, 0.0, 1.0]})),
        );

        // ===== Collections =====
        map.insert(
            FormatKnowledgeKey::generic("alloc::vec::Vec"),
            BrpFormatKnowledge {
                example_value:      json!([]),
                subfield_paths:     None, // Collections have index access, not component access
                mutation_knowledge: Knowledge::Teach,
            },
        );
        map.insert(
            FormatKnowledgeKey::generic("std::collections::HashMap"),
            BrpFormatKnowledge {
                example_value:      json!({}),
                subfield_paths:     None,
                mutation_knowledge: Knowledge::Teach,
            },
        );
        map.insert(
            FormatKnowledgeKey::generic("std::collections::BTreeMap"),
            BrpFormatKnowledge {
                example_value:      json!({}),
                subfield_paths:     None,
                mutation_knowledge: Knowledge::Teach,
            },
        );

        // ===== Option types =====
        map.insert(
            FormatKnowledgeKey::generic("core::option::Option"),
            BrpFormatKnowledge {
                example_value:      json!(null),
                subfield_paths:     None,
                mutation_knowledge: Knowledge::Teach,
            },
        );

        // ===== Bevy ECS types =====
        // Name serializes as a plain string, not as a struct with hash/name fields
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_NAME),
            BrpFormatKnowledge::as_value(json!("Entity Name"), "String".to_string()),
        );

        // ===== Camera3d depth texture usage =====
        // Camera3dDepthTextureUsage - wrapper around u32 texture usage flags
        // Valid flags: COPY_SRC=1, COPY_DST=2, TEXTURE_BINDING=4, STORAGE_BINDING=8,
        // RENDER_ATTACHMENT=16 STORAGE_BINDING (8) causes crashes with multisampled
        // textures! Safe combinations: 16 (RENDER_ATTACHMENT only), 20 (RENDER_ATTACHMENT |
        // TEXTURE_BINDING)
        map.insert(
            FormatKnowledgeKey::exact(
                "bevy_core_pipeline::core_3d::camera_3d::Camera3dDepthTextureUsage",
            ),
            BrpFormatKnowledge {
                example_value:      json!(20), /* RENDER_ATTACHMENT | TEXTURE_BINDING - safe
                                                * combination */
                subfield_paths:     None,
                mutation_knowledge: Knowledge::Teach,
            },
        );

        // ===== Transform types =====
        // GlobalTransform - wraps glam::Affine3A but serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Registry shows nested object but BRP actually expects flat array
        map.insert(
            FormatKnowledgeKey::exact(
                "bevy_transform::components::global_transform::GlobalTransform",
            ),
            BrpFormatKnowledge {
                example_value:      json!([
                    1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0
                ]),
                subfield_paths:     None, // Affine matrices don't have simple component access
                mutation_knowledge: Knowledge::Teach,
            },
        );

        // ===== Asset Handle types =====
        // Handle<T> types - use Weak variant with UUID format for mutations
        // Schema provides non-functional examples, but this format works
        map.insert(
            FormatKnowledgeKey::exact(TYPE_BEVY_IMAGE_HANDLE),
            BrpFormatKnowledge::simple(
                json!({"Weak": {"Uuid": {"uuid": "12345678-1234-1234-1234-123456789012"}}}),
            ),
        );

        map
    });
