//! Hardcoded BRP format knowledge
//!
//! This module contains the static knowledge of how types should be serialized
//! for BRP, which often differs from their reflection-based representation.
//! This knowledge is extracted from the extras plugin's examples.rs.

use std::collections::HashMap;
use std::sync::LazyLock;

use error_stack::Report;
use serde_json::Value;
use serde_json::json;

use super::constants::TYPE_ALLOC_STRING;
use super::constants::TYPE_BEVY_CAMERA;
use super::constants::TYPE_BEVY_ENTITY;
use super::constants::TYPE_BEVY_MAT2;
use super::constants::TYPE_BEVY_MAT3;
use super::constants::TYPE_BEVY_MAT4;
use super::constants::TYPE_BEVY_NAME;
use super::constants::TYPE_BEVY_QUAT;
use super::constants::TYPE_BEVY_RECT;
use super::constants::TYPE_BEVY_RENDER_TARGET;
use super::constants::TYPE_BEVY_VEC2;
use super::constants::TYPE_BEVY_VEC3;
use super::constants::TYPE_BEVY_VEC3A;
use super::constants::TYPE_BEVY_VEC4;
use super::constants::TYPE_BLOOM;
use super::constants::TYPE_BOOL;
use super::constants::TYPE_CHAR;
use super::constants::TYPE_CORE_DURATION;
use super::constants::TYPE_F32;
use super::constants::TYPE_F64;
use super::constants::TYPE_GLAM_AFFINE2;
use super::constants::TYPE_GLAM_AFFINE3A;
use super::constants::TYPE_GLAM_IVEC2;
use super::constants::TYPE_GLAM_IVEC3;
use super::constants::TYPE_GLAM_IVEC4;
use super::constants::TYPE_GLAM_MAT2;
use super::constants::TYPE_GLAM_MAT3;
use super::constants::TYPE_GLAM_MAT3A;
use super::constants::TYPE_GLAM_MAT4;
use super::constants::TYPE_GLAM_QUAT;
use super::constants::TYPE_GLAM_UVEC2;
use super::constants::TYPE_GLAM_UVEC3;
use super::constants::TYPE_GLAM_UVEC4;
use super::constants::TYPE_GLAM_VEC2;
use super::constants::TYPE_GLAM_VEC3;
use super::constants::TYPE_GLAM_VEC3A;
use super::constants::TYPE_GLAM_VEC4;
use super::constants::TYPE_I8;
use super::constants::TYPE_I16;
use super::constants::TYPE_I32;
use super::constants::TYPE_I64;
use super::constants::TYPE_I128;
use super::constants::TYPE_ISIZE;
use super::constants::TYPE_STD_STRING;
use super::constants::TYPE_STR;
use super::constants::TYPE_STR_REF;
use super::constants::TYPE_STRING;
use super::constants::TYPE_U8;
use super::constants::TYPE_U16;
use super::constants::TYPE_U32;
use super::constants::TYPE_U64;
use super::constants::TYPE_U128;
use super::constants::TYPE_USIZE;
use super::mutation_path_builder::VariantSignature;
use crate::brp_tools::BrpTypeName;
use crate::error::Error;

/// Format knowledge key for matching types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KnowledgeKey {
    /// Exact type name match (current behavior)
    Exact(BrpTypeName),
    /// Struct field-specific match for providing appropriate field values
    StructField {
        /// e.g., `bevy_window::window::WindowResolution`
        struct_type: BrpTypeName,
        /// e.g., `physical_width`
        field_name:  String,
    },
    /// Match an indexed element within enum variants that share a signature
    EnumVariantSignature {
        enum_type: BrpTypeName,
        signature: VariantSignature,
        index:     usize,
    },
}

impl KnowledgeKey {
    /// Create an exact match key
    pub fn exact(s: impl Into<BrpTypeName>) -> Self {
        Self::Exact(s.into())
    }

    /// Create a struct field match key
    pub fn struct_field(
        struct_type: impl Into<BrpTypeName>,
        field_name: impl Into<String>,
    ) -> Self {
        Self::StructField {
            struct_type: struct_type.into(),
            field_name:  field_name.into(),
        }
    }

    /// Create an enum variant signature match key
    pub fn enum_variant_signature(
        enum_type: impl Into<BrpTypeName>,
        signature: VariantSignature,
        index: usize,
    ) -> Self {
        Self::EnumVariantSignature {
            enum_type: enum_type.into(),
            signature,
            index,
        }
    }
}

/// Hardcoded BRP format knowledge for a type
#[derive(Debug, Clone)]
pub enum TypeKnowledge {
    /// Simple value with just an example
    TeachAndRecurse { example: Value },
    /// Value that should be treated as opaque (no mutation paths)
    TreatAsRootValue {
        example:         Value,
        simplified_type: String,
    },
}

impl TypeKnowledge {
    /// Create a simple knowledge entry with no subfields
    pub const fn new(example: Value) -> Self {
        Self::TeachAndRecurse { example }
    }

    /// Create a knowledge entry that should be treated as a simple value
    pub fn as_root_value(example: Value, simplified_type: impl Into<String>) -> Self {
        Self::TreatAsRootValue {
            example,
            simplified_type: simplified_type.into(),
        }
    }

    /// Get the example value for this knowledge
    pub const fn example(&self) -> &Value {
        match self {
            Self::TeachAndRecurse { example } | Self::TreatAsRootValue { example, .. } => example,
        }
    }

    /// Get simplified name for a type if it has `TreatAsRootValue` knowledge
    pub fn get_simplified_name(type_name: &BrpTypeName) -> Option<BrpTypeName> {
        let knowledge_key = KnowledgeKey::exact(type_name);
        if let Some(Self::TreatAsRootValue {
            simplified_type, ..
        }) = BRP_TYPE_KNOWLEDGE.get(&knowledge_key)
        {
            Some(BrpTypeName::from(simplified_type.clone()))
        } else {
            None
        }
    }

    /// Get the example value for `bevy_ecs::entity::Entity` type from type knowledge
    ///
    /// This is used for generating agent guidance messages that reference Entity IDs.
    /// Returns an error if the Entity type knowledge is missing or invalid.
    pub fn get_entity_example_value() -> crate::error::Result<u64> {
        BRP_TYPE_KNOWLEDGE
            .get(&KnowledgeKey::exact(TYPE_BEVY_ENTITY))
            .and_then(|knowledge| knowledge.example().as_u64())
            .ok_or_else(|| {
                Error::InvalidState(
                    "Entity type knowledge missing or invalid in BRP_TYPE_KNOWLEDGE".to_string(),
                )
            })
            .map_err(Report::new)
    }
}

/// Static map of hardcoded BRP format knowledge
/// This captures the serialization rules that can't be derived from registry
pub static BRP_TYPE_KNOWLEDGE: LazyLock<HashMap<KnowledgeKey, TypeKnowledge>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // ===== Numeric types =====
        map.insert(
            KnowledgeKey::exact(TYPE_I8),
            TypeKnowledge::as_root_value(json!(42), TYPE_I8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I16),
            TypeKnowledge::as_root_value(json!(1), TYPE_I16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I32),
            TypeKnowledge::as_root_value(json!(1), TYPE_I32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I64),
            TypeKnowledge::as_root_value(json!(1), TYPE_I64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I128),
            TypeKnowledge::as_root_value(json!("123456789012345678901234567890"), TYPE_I128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U8),
            TypeKnowledge::as_root_value(json!(128), TYPE_U8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U16),
            TypeKnowledge::as_root_value(json!(5000), TYPE_U16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U32),
            TypeKnowledge::as_root_value(json!(1_u32), TYPE_U32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U64),
            TypeKnowledge::as_root_value(json!(1_u64), TYPE_U64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U128),
            TypeKnowledge::as_root_value(json!("987654321098765432109876543210"), TYPE_U128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F32),
            TypeKnowledge::as_root_value(json!(1.0_f32), TYPE_F32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F64),
            TypeKnowledge::as_root_value(json!(1.0_f64), TYPE_F64),
        );

        // ===== Size types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ISIZE),
            TypeKnowledge::as_root_value(json!(1_i64), TYPE_ISIZE),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_USIZE),
            TypeKnowledge::as_root_value(json!(2_u64), TYPE_USIZE),
        );

        // ===== Text types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ALLOC_STRING),
            TypeKnowledge::as_root_value(json!("Hello, World!"), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STD_STRING),
            TypeKnowledge::as_root_value(json!("Hello, World!"), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STRING),
            TypeKnowledge::as_root_value(json!("Hello, World!"), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR_REF),
            TypeKnowledge::as_root_value(json!("static string"), TYPE_STR),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR),
            TypeKnowledge::as_root_value(json!("static string"), TYPE_STR),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CHAR),
            TypeKnowledge::as_root_value(json!('A'), TYPE_CHAR),
        );

        // ===== Boolean =====
        map.insert(
            KnowledgeKey::exact(TYPE_BOOL),
            TypeKnowledge::as_root_value(json!(true), TYPE_BOOL),
        );

        // ===== Time types =====
        // Duration - core time type with secs (u64) and nanos (u32) fields
        // Serializes as struct with both fields required
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_DURATION),
            TypeKnowledge::as_root_value(json!({"secs": 0, "nanos": 0}), TYPE_CORE_DURATION),
        );

        // ===== Unit tuple =====
        // Unit tuple () serializes as empty array [] in BRP mutations
        // required for `bevy_time::time::Time<()>`
        map.insert(
            KnowledgeKey::exact("()"),
            TypeKnowledge::as_root_value(json!([]), "()"),
        );

        // ===== UUID =====
        // Standard UUID v4 format string
        map.insert(
            KnowledgeKey::exact("uuid::Uuid"),
            TypeKnowledge::as_root_value(json!("550e8400-e29b-41d4-a716-446655440000"), "Uuid"),
        );

        // ===== Bevy math types (these serialize as arrays, not objects!) =====
        // Vec2
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC2),
            TypeKnowledge::new(json!([1.0, 2.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC2),
            TypeKnowledge::new(json!([1.0, 2.0])),
        );

        // Vec3
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3A),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3A),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0])),
        );

        // Vec4
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC4),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0, 4.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC4),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0, 4.0])),
        );

        // Double-precision vectors (f64)
        map.insert(
            KnowledgeKey::exact("glam::DVec2"),
            TypeKnowledge::new(json!([1.0, 2.0])),
        );
        map.insert(
            KnowledgeKey::exact("glam::DVec3"),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0])),
        );
        map.insert(
            KnowledgeKey::exact("glam::DVec4"),
            TypeKnowledge::new(json!([1.0, 2.0, 3.0, 4.0])),
        );

        // Integer vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC2),
            TypeKnowledge::new(json!([0, 0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC3),
            TypeKnowledge::new(json!([0, 0, 0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC4),
            TypeKnowledge::new(json!([0, 0, 0, 0])),
        );

        // Unsigned vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC2),
            TypeKnowledge::new(json!([0, 0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC3),
            TypeKnowledge::new(json!([0, 0, 0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC4),
            TypeKnowledge::new(json!([0, 0, 0, 0])),
        );

        // Quaternion
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_QUAT),
            TypeKnowledge::new(json!([0.0, 0.0, 0.0, 1.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_QUAT),
            TypeKnowledge::new(json!([0.0, 0.0, 0.0, 1.0])),
        );

        // Matrices
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT2),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 1.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT2),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 1.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT3),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT3),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])),
        );
        // Mat3A - Used in GlobalTransform.0.matrix3, expects flat array not nested object
        // The error was: "invalid type: map, expected a sequence of 9 f32values"
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT3A),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0])),
        );
        // Mat4 - BRP expects flat array of 16 values, not nested 2D array
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT4),
            TypeKnowledge::new(json!([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0
            ])),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT4),
            TypeKnowledge::new(json!([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0
            ])),
        );

        // ===== Bevy math Rect =====
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_RECT),
            TypeKnowledge::new(json!({
                "min": [0.0, 0.0],
                "max": [100.0, 100.0]
            })), // Has nested paths via Vec2 fields
        );

        // ===== Bevy ECS types =====
        // Entity - serializes as u64 (entity.to_bits()), not as struct
        // WARNING: This is just an example! For actual BRP operations, use VALID entity IDs
        // obtained from spawn operations or queries. Using invalid entity IDs will cause errors.
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_ENTITY),
            TypeKnowledge::as_root_value(json!(8_589_934_670_u64), TYPE_BEVY_ENTITY),
        );

        // Name serializes as a plain string, not as a struct with hash/name fields
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_NAME),
            TypeKnowledge::as_root_value(json!("Entity Name"), TYPE_STRING),
        );

        // ===== Camera field-specific values =====
        // Provide safe RenderTarget to prevent crashes from invalid TextureView handles
        // TextureView variant requires handle to exist in ManualTextureViews resource
        // Window::Primary is always valid and references the default primary window
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_CAMERA, "target"),
            TypeKnowledge::as_root_value(json!({"Window": "Primary"}), TYPE_BEVY_RENDER_TARGET),
        );

        // ===== Camera3d field-specific values =====
        // Camera3dDepthTextureUsage - wrapper around u32 texture usage flags
        // Valid flags: COPY_SRC=1, COPY_DST=2, TEXTURE_BINDING=4, STORAGE_BINDING=8,
        // RENDER_ATTACHMENT=16 STORAGE_BINDING (8) causes crashes with multisampled
        // textures! Safe combinations: 16 (RENDER_ATTACHMENT only), 20 (RENDER_ATTACHMENT |
        // TEXTURE_BINDING)
        map.insert(
            KnowledgeKey::struct_field("bevy_camera::components::Camera3d", "depth_texture_usages"),
            // RENDER_ATTACHMENT | TEXTURE_BINDING - safe combination, treat as opaque u32
            TypeKnowledge::as_root_value(json!(20), TYPE_U32),
        );

        // Screen space specular transmission steps - reasonable value to prevent memory issues
        // Default is 1, typical range is 0-4 per transmission.rs example
        map.insert(
            KnowledgeKey::struct_field(
                "bevy_camera::components::Camera3d",
                "screen_space_specular_transmission_steps",
            ),
            TypeKnowledge::as_root_value(json!(1), TYPE_USIZE),
        );

        // ===== Transform types =====
        // GlobalTransform - wraps glam::Affine3A but serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Registry shows nested object but BRP actually expects flat array
        map.insert(
            KnowledgeKey::exact("bevy_transform::components::global_transform::GlobalTransform"),
            TypeKnowledge::new(json!([
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0
            ])), // Affine matrices don't have simple component access
        );

        // Affine2 - Used in UiGlobalTransform.0, serializes as flat array of 6 f32 values
        // Format: [matrix_row1(2), matrix_row2(2), translation(2)]
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_AFFINE2),
            TypeKnowledge::new(json!([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])),
        );

        // Affine3A - Used as GlobalTransform.0, serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Has matrix3 and translation fields but doesn't serialize with field names
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_AFFINE3A),
            TypeKnowledge::new(json!([
                1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0
            ])),
        );

        // ===== Asset Handle types =====
        // Handle<T> types - use Weak variant with UUID format for mutations
        // Schema provides non-functional examples, but this format works

        // ===== WindowResolution field-specific values =====
        // Provide reasonable window dimension values to prevent GPU texture size errors
        map.insert(
            KnowledgeKey::struct_field("bevy_window::window::WindowResolution", "physical_width"),
            TypeKnowledge::as_root_value(json!(800), TYPE_U32), // Reasonable window width
        );
        map.insert(
            KnowledgeKey::struct_field("bevy_window::window::WindowResolution", "physical_height"),
            TypeKnowledge::as_root_value(json!(600), TYPE_U32), // Reasonable window height
        );

        // ===== GlyphAtlasLocation field-specific values =====
        // Provide safe glyph index to prevent crashes from out-of-bounds atlas access
        map.insert(
            KnowledgeKey::struct_field("bevy_text::glyph::GlyphAtlasLocation", "glyph_index"),
            TypeKnowledge::as_root_value(json!(5), TYPE_USIZE),
        );

        // ===== VideoMode field-specific values =====
        // Provide realistic video mode values to prevent window system crashes
        map.insert(
            KnowledgeKey::struct_field("bevy_window::monitor::VideoMode", "bit_depth"),
            TypeKnowledge::as_root_value(json!(32), "u16"), // Standard 32-bit color
        );
        map.insert(
            KnowledgeKey::struct_field("bevy_window::monitor::VideoMode", "physical_size"),
            TypeKnowledge::as_root_value(json!([1920, 1080]), "UVec2"), /* Standard Full HD
                                                                         * resolution */
        );
        map.insert(
            KnowledgeKey::struct_field(
                "bevy_window::monitor::VideoMode",
                "refresh_rate_millihertz",
            ),
            TypeKnowledge::as_root_value(json!(60000), TYPE_U32), // 60 Hz in millihertz
        );

        // ===== Bloom field-specific values =====
        // Provide safe max_mip_dimension to prevent GPU texture allocation crashes
        // Default is 512, using u32 generic value of 1_000_000 causes rendering pipeline corruption
        map.insert(
            KnowledgeKey::struct_field(TYPE_BLOOM, "max_mip_dimension"),
            TypeKnowledge::as_root_value(json!(512), TYPE_U32),
        );

        // ===== NonZero types =====
        // These types guarantee the value is never zero
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroU8"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroU8"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroU16"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroU16"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroU32"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroU32"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroU64"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroU64"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroU128"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroU128"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroUsize"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroUsize"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroI8"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroI8"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroI16"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroI16"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroI32"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroI32"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroI64"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroI64"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroI128"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroI128"),
        );
        map.insert(
            KnowledgeKey::exact("core::num::NonZeroIsize"),
            TypeKnowledge::as_root_value(json!(1), "NonZeroIsize"),
        );

        // ===== AlphaMode2d enum variant signatures =====
        // Mask(f32) variant requires alpha threshold in 0.0-1.0 range
        map.insert(
            KnowledgeKey::enum_variant_signature(
                "bevy_sprite_render::mesh2d::material::AlphaMode2d",
                VariantSignature::Tuple(vec![BrpTypeName::from("f32")]),
                0,
            ),
            TypeKnowledge::as_root_value(json!(0.5), TYPE_F32),
        );

        map
    });
