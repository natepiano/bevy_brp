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

use super::BrpTypeName;
use super::constants;
use super::constants::ALPHA_MODE_2D_MASK_SIGNATURE_INDEX;
use super::constants::DEFAULT_WRAP_PERIOD_SECS;
use super::constants::EXAMPLE_AFFINE2;
use super::constants::EXAMPLE_AFFINE3A;
use super::constants::EXAMPLE_ALPHA_MODE_2D_MASK;
use super::constants::EXAMPLE_BLOOM_MAX_MIP_DIMENSION;
use super::constants::EXAMPLE_BOOL;
use super::constants::EXAMPLE_CAMERA3D_DEPTH_TEXTURE_USAGES;
use super::constants::EXAMPLE_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS;
use super::constants::EXAMPLE_CHAR;
use super::constants::EXAMPLE_DVEC2;
use super::constants::EXAMPLE_DVEC3;
use super::constants::EXAMPLE_DVEC4;
use super::constants::EXAMPLE_ENTITY_BITS;
use super::constants::EXAMPLE_F32;
use super::constants::EXAMPLE_F64;
use super::constants::EXAMPLE_FIXED_TIMESTEP_NANOS;
use super::constants::EXAMPLE_GLOBAL_TRANSFORM;
use super::constants::EXAMPLE_GLYPH_INDEX;
use super::constants::EXAMPLE_I8;
use super::constants::EXAMPLE_I16;
use super::constants::EXAMPLE_I32;
use super::constants::EXAMPLE_I64;
use super::constants::EXAMPLE_I128;
use super::constants::EXAMPLE_ISIZE;
use super::constants::EXAMPLE_IVEC2;
use super::constants::EXAMPLE_IVEC3;
use super::constants::EXAMPLE_IVEC4;
use super::constants::EXAMPLE_MAT2;
use super::constants::EXAMPLE_MAT3;
use super::constants::EXAMPLE_MAT4;
use super::constants::EXAMPLE_NAME;
use super::constants::EXAMPLE_NON_ZERO_I8;
use super::constants::EXAMPLE_NON_ZERO_I16;
use super::constants::EXAMPLE_NON_ZERO_I32;
use super::constants::EXAMPLE_NON_ZERO_I64;
use super::constants::EXAMPLE_NON_ZERO_I128;
use super::constants::EXAMPLE_NON_ZERO_ISIZE;
use super::constants::EXAMPLE_NON_ZERO_U8;
use super::constants::EXAMPLE_NON_ZERO_U16;
use super::constants::EXAMPLE_NON_ZERO_U32;
use super::constants::EXAMPLE_NON_ZERO_U64;
use super::constants::EXAMPLE_NON_ZERO_U128;
use super::constants::EXAMPLE_NON_ZERO_USIZE;
use super::constants::EXAMPLE_QUAT;
use super::constants::EXAMPLE_STATIC_STR;
use super::constants::EXAMPLE_STRING;
use super::constants::EXAMPLE_U8;
use super::constants::EXAMPLE_U16;
use super::constants::EXAMPLE_U32;
use super::constants::EXAMPLE_U64;
use super::constants::EXAMPLE_U128;
use super::constants::EXAMPLE_UNIT_ARRAY;
use super::constants::EXAMPLE_USIZE;
use super::constants::EXAMPLE_UUID;
use super::constants::EXAMPLE_UVEC2;
use super::constants::EXAMPLE_UVEC3;
use super::constants::EXAMPLE_UVEC4;
use super::constants::EXAMPLE_VEC2;
use super::constants::EXAMPLE_VEC3;
use super::constants::EXAMPLE_VEC4;
use super::constants::EXAMPLE_VIDEO_MODE_BIT_DEPTH;
use super::constants::EXAMPLE_VIDEO_MODE_PHYSICAL_SIZE;
use super::constants::EXAMPLE_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ;
use super::constants::EXAMPLE_VIRTUAL_MAX_DELTA_NANOS;
use super::constants::EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_HEIGHT;
use super::constants::EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_WIDTH;
use super::constants::FIELD_BLOOM_MAX_MIP_DIMENSION;
use super::constants::FIELD_CAMERA_TARGET;
use super::constants::FIELD_CAMERA3D_DEPTH_TEXTURE_USAGES;
use super::constants::FIELD_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS;
use super::constants::FIELD_FIXED_TIMESTEP;
use super::constants::FIELD_GLYPH_ATLAS_LOCATION_GLYPH_INDEX;
use super::constants::FIELD_TIME_WRAP_PERIOD;
use super::constants::FIELD_VIDEO_MODE_BIT_DEPTH;
use super::constants::FIELD_VIDEO_MODE_PHYSICAL_SIZE;
use super::constants::FIELD_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ;
use super::constants::FIELD_VIRTUAL_MAX_DELTA;
use super::constants::FIELD_WINDOW_RESOLUTION_PHYSICAL_HEIGHT;
use super::constants::FIELD_WINDOW_RESOLUTION_PHYSICAL_WIDTH;
use super::constants::SIMPLIFIED_NON_ZERO_I8;
use super::constants::SIMPLIFIED_NON_ZERO_I16;
use super::constants::SIMPLIFIED_NON_ZERO_I32;
use super::constants::SIMPLIFIED_NON_ZERO_I64;
use super::constants::SIMPLIFIED_NON_ZERO_I128;
use super::constants::SIMPLIFIED_NON_ZERO_ISIZE;
use super::constants::SIMPLIFIED_NON_ZERO_U8;
use super::constants::SIMPLIFIED_NON_ZERO_U16;
use super::constants::SIMPLIFIED_NON_ZERO_U32;
use super::constants::SIMPLIFIED_NON_ZERO_U64;
use super::constants::SIMPLIFIED_NON_ZERO_U128;
use super::constants::SIMPLIFIED_NON_ZERO_USIZE;
use super::constants::SIMPLIFIED_UNIT;
use super::constants::SIMPLIFIED_UUID;
use super::constants::SIMPLIFIED_UVEC2;
use super::constants::TYPE_ALLOC_STRING;
use super::constants::TYPE_BEVY_ALPHA_MODE_2D;
use super::constants::TYPE_BEVY_CAMERA;
use super::constants::TYPE_BEVY_CAMERA3D;
use super::constants::TYPE_BEVY_ENTITY;
use super::constants::TYPE_BEVY_FIXED;
use super::constants::TYPE_BEVY_GLOBAL_TRANSFORM;
use super::constants::TYPE_BEVY_GLYPH_ATLAS_LOCATION;
use super::constants::TYPE_BEVY_MAT2;
use super::constants::TYPE_BEVY_MAT3;
use super::constants::TYPE_BEVY_MAT4;
use super::constants::TYPE_BEVY_NAME;
use super::constants::TYPE_BEVY_QUAT;
use super::constants::TYPE_BEVY_RECT;
use super::constants::TYPE_BEVY_TIME_EMPTY_CONTAINER;
use super::constants::TYPE_BEVY_TIME_FIXED_CONTAINER;
use super::constants::TYPE_BEVY_TIME_REAL_CONTAINER;
use super::constants::TYPE_BEVY_TIME_VIRTUAL_CONTAINER;
use super::constants::TYPE_BEVY_VEC2;
use super::constants::TYPE_BEVY_VEC3;
use super::constants::TYPE_BEVY_VEC3A;
use super::constants::TYPE_BEVY_VEC4;
use super::constants::TYPE_BEVY_VIDEO_MODE;
use super::constants::TYPE_BEVY_VIRTUAL;
use super::constants::TYPE_BEVY_WINDOW_RESOLUTION;
use super::constants::TYPE_BLOOM;
use super::constants::TYPE_BOOL;
use super::constants::TYPE_CHAR;
use super::constants::TYPE_CORE_DURATION;
use super::constants::TYPE_CORE_NON_ZERO_I8;
use super::constants::TYPE_CORE_NON_ZERO_I16;
use super::constants::TYPE_CORE_NON_ZERO_I32;
use super::constants::TYPE_CORE_NON_ZERO_I64;
use super::constants::TYPE_CORE_NON_ZERO_I128;
use super::constants::TYPE_CORE_NON_ZERO_ISIZE;
use super::constants::TYPE_CORE_NON_ZERO_U8;
use super::constants::TYPE_CORE_NON_ZERO_U16;
use super::constants::TYPE_CORE_NON_ZERO_U32;
use super::constants::TYPE_CORE_NON_ZERO_U64;
use super::constants::TYPE_CORE_NON_ZERO_U128;
use super::constants::TYPE_CORE_NON_ZERO_USIZE;
use super::constants::TYPE_F32;
use super::constants::TYPE_F64;
use super::constants::TYPE_GLAM_AFFINE2;
use super::constants::TYPE_GLAM_AFFINE3A;
use super::constants::TYPE_GLAM_DVEC2;
use super::constants::TYPE_GLAM_DVEC3;
use super::constants::TYPE_GLAM_DVEC4;
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
use super::constants::TYPE_UNIT;
use super::constants::TYPE_USIZE;
use super::constants::TYPE_UUID;
use super::constants::ZERO_NANOS;
use super::constants::ZERO_SECONDS;
use super::variant_signature::VariantSignature;
use crate::error::Error;
use crate::error::Result;

/// Format knowledge key for matching types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum KnowledgeKey {
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
    pub(super) fn exact(s: impl Into<BrpTypeName>) -> Self { Self::Exact(s.into()) }

    /// Create a struct field match key
    pub(super) fn struct_field(
        struct_type: impl Into<BrpTypeName>,
        field_name: impl Into<String>,
    ) -> Self {
        Self::StructField {
            struct_type: struct_type.into(),
            field_name:  field_name.into(),
        }
    }

    /// Create an enum variant signature match key
    pub(super) fn enum_variant_signature(
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
pub(super) enum TypeKnowledge {
    /// Simple value with just an example
    TeachAndRecurse { example: Value },
    /// Value that should be treated as opaque (no mutation paths)
    TreatAsRootValue {
        example:         Value,
        simplified_type: String,
    },
}

/// Action to take based on type knowledge lookup
///
/// This enum represents the **control flow decisions** that builders should make
/// after consulting the knowledge base, distinct from `TypeKnowledge` which
/// represents the **static facts** stored in the knowledge base.
#[derive(Debug, Clone)]
pub(super) enum KnowledgeAction {
    /// Use this example as the root value - DO NOT recurse into children
    ///
    /// Returned for `TreatAsRootValue` knowledge where the type should be treated
    /// as opaque (e.g., `Duration`, `String`, primitive wrappers).
    CompleteWithExample(Value),

    /// Use this example but CONTINUE recursing into children
    ///
    /// Returned for `TeachAndRecurse` knowledge where we want to override the
    /// example but still expose child mutation paths (e.g., struct field defaults,
    /// enum variant selection).
    UseExampleAndRecurse(Value),

    /// Missing hardcoded knowledge - assemble example from children normally
    Missing,
}

impl TypeKnowledge {
    /// Create a simple knowledge entry with no subfields
    pub(super) const fn new(example: Value) -> Self { Self::TeachAndRecurse { example } }

    /// Create a knowledge entry that should be treated as a simple value
    pub(super) fn as_root_value(example: Value, simplified_type: impl Into<String>) -> Self {
        Self::TreatAsRootValue {
            example,
            simplified_type: simplified_type.into(),
        }
    }

    /// Get the example value for this knowledge
    pub(super) const fn example(&self) -> &Value {
        match self {
            Self::TeachAndRecurse { example } | Self::TreatAsRootValue { example, .. } => example,
        }
    }

    /// Get simplified name for a type if it has `TreatAsRootValue` knowledge
    pub(super) fn get_simplified_name(type_name: &BrpTypeName) -> Option<BrpTypeName> {
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
    /// This is used for generating agent guidance messages that reference `Entity` IDs.
    /// Returns an error if the `Entity` type knowledge is missing or invalid.
    pub(super) fn get_entity_example_value() -> Result<u64> {
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
pub(super) static BRP_TYPE_KNOWLEDGE: LazyLock<HashMap<KnowledgeKey, TypeKnowledge>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // ===== Numeric types =====
        map.insert(
            KnowledgeKey::exact(TYPE_I8),
            TypeKnowledge::as_root_value(json!(EXAMPLE_I8), TYPE_I8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I16),
            TypeKnowledge::as_root_value(json!(EXAMPLE_I16), TYPE_I16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I32),
            TypeKnowledge::as_root_value(json!(EXAMPLE_I32), TYPE_I32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I64),
            TypeKnowledge::as_root_value(json!(EXAMPLE_I64), TYPE_I64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_I128),
            TypeKnowledge::as_root_value(json!(EXAMPLE_I128), TYPE_I128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U8),
            TypeKnowledge::as_root_value(json!(EXAMPLE_U8), TYPE_U8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U16),
            TypeKnowledge::as_root_value(json!(EXAMPLE_U16), TYPE_U16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U32),
            TypeKnowledge::as_root_value(json!(EXAMPLE_U32), TYPE_U32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U64),
            TypeKnowledge::as_root_value(json!(EXAMPLE_U64), TYPE_U64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_U128),
            TypeKnowledge::as_root_value(json!(EXAMPLE_U128), TYPE_U128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F32),
            TypeKnowledge::as_root_value(json!(EXAMPLE_F32), TYPE_F32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_F64),
            TypeKnowledge::as_root_value(json!(EXAMPLE_F64), TYPE_F64),
        );

        // ===== Size types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ISIZE),
            TypeKnowledge::as_root_value(json!(EXAMPLE_ISIZE), TYPE_ISIZE),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_USIZE),
            TypeKnowledge::as_root_value(json!(EXAMPLE_USIZE), TYPE_USIZE),
        );

        // ===== Text types =====
        map.insert(
            KnowledgeKey::exact(TYPE_ALLOC_STRING),
            TypeKnowledge::as_root_value(json!(EXAMPLE_STRING), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STD_STRING),
            TypeKnowledge::as_root_value(json!(EXAMPLE_STRING), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STRING),
            TypeKnowledge::as_root_value(json!(EXAMPLE_STRING), TYPE_STRING),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR_REF),
            TypeKnowledge::as_root_value(json!(EXAMPLE_STATIC_STR), TYPE_STR),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_STR),
            TypeKnowledge::as_root_value(json!(EXAMPLE_STATIC_STR), TYPE_STR),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CHAR),
            TypeKnowledge::as_root_value(json!(EXAMPLE_CHAR), TYPE_CHAR),
        );

        // ===== Boolean =====
        map.insert(
            KnowledgeKey::exact(TYPE_BOOL),
            TypeKnowledge::as_root_value(json!(EXAMPLE_BOOL), TYPE_BOOL),
        );

        // ===== Time types =====
        // Duration - core time type with secs (u64) and nanos (u32) fields
        // Serializes as struct with both fields required
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_DURATION),
            TypeKnowledge::as_root_value(
                constants::duration_value(ZERO_SECONDS, ZERO_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // ===== Unit tuple =====
        // Unit tuple () serializes as empty array [] in BRP mutations
        // required for `bevy_time::time::Time<()>`
        map.insert(
            KnowledgeKey::exact(TYPE_UNIT),
            TypeKnowledge::as_root_value(json!(EXAMPLE_UNIT_ARRAY), SIMPLIFIED_UNIT),
        );

        // ===== UUID =====
        // Standard UUID v4 format string
        map.insert(
            KnowledgeKey::exact(TYPE_UUID),
            TypeKnowledge::as_root_value(json!(EXAMPLE_UUID), SIMPLIFIED_UUID),
        );

        // ===== Bevy math types (these serialize as arrays, not objects!) =====
        // Vec2
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC2),
            TypeKnowledge::new(json!(EXAMPLE_VEC2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC2),
            TypeKnowledge::new(json!(EXAMPLE_VEC2)),
        );

        // Vec3
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3),
            TypeKnowledge::new(json!(EXAMPLE_VEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC3A),
            TypeKnowledge::new(json!(EXAMPLE_VEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3),
            TypeKnowledge::new(json!(EXAMPLE_VEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC3A),
            TypeKnowledge::new(json!(EXAMPLE_VEC3)),
        );

        // Vec4
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_VEC4),
            TypeKnowledge::new(json!(EXAMPLE_VEC4)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_VEC4),
            TypeKnowledge::new(json!(EXAMPLE_VEC4)),
        );

        // Double-precision vectors (f64)
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_DVEC2),
            TypeKnowledge::new(json!(EXAMPLE_DVEC2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_DVEC3),
            TypeKnowledge::new(json!(EXAMPLE_DVEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_DVEC4),
            TypeKnowledge::new(json!(EXAMPLE_DVEC4)),
        );

        // Integer vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC2),
            TypeKnowledge::new(json!(EXAMPLE_IVEC2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC3),
            TypeKnowledge::new(json!(EXAMPLE_IVEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_IVEC4),
            TypeKnowledge::new(json!(EXAMPLE_IVEC4)),
        );

        // Unsigned vectors
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC2),
            TypeKnowledge::new(json!(EXAMPLE_UVEC2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC3),
            TypeKnowledge::new(json!(EXAMPLE_UVEC3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_UVEC4),
            TypeKnowledge::new(json!(EXAMPLE_UVEC4)),
        );

        // Quaternion
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_QUAT),
            TypeKnowledge::new(json!(EXAMPLE_QUAT)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_QUAT),
            TypeKnowledge::new(json!(EXAMPLE_QUAT)),
        );

        // Matrices
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT2),
            TypeKnowledge::new(json!(EXAMPLE_MAT2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT2),
            TypeKnowledge::new(json!(EXAMPLE_MAT2)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT3),
            TypeKnowledge::new(json!(EXAMPLE_MAT3)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT3),
            TypeKnowledge::new(json!(EXAMPLE_MAT3)),
        );
        // Mat3A - Used in GlobalTransform.0.matrix3, expects flat array not nested object
        // The error was: "invalid type: map, expected a sequence of 9 f32values"
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT3A),
            TypeKnowledge::new(json!(EXAMPLE_MAT3)),
        );
        // Mat4 - BRP expects flat array of 16 values, not nested 2D array
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_MAT4),
            TypeKnowledge::new(json!(EXAMPLE_MAT4)),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_MAT4),
            TypeKnowledge::new(json!(EXAMPLE_MAT4)),
        );

        // ===== Bevy math Rect =====
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_RECT),
            TypeKnowledge::new(constants::rect_value()), // Has nested paths via Vec2 fields
        );

        // ===== Bevy ECS types =====
        // Entity - serializes as u64 (entity.to_bits()), not as struct
        // WARNING: This is just an example! For actual BRP operations, use VALID entity IDs
        // obtained from spawn operations or queries. Using invalid entity IDs will cause errors.
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_ENTITY),
            TypeKnowledge::as_root_value(json!(EXAMPLE_ENTITY_BITS), TYPE_BEVY_ENTITY),
        );

        // Name serializes as a plain string, not as a struct with hash/name fields
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_NAME),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NAME), TYPE_STRING),
        );

        // ===== Camera field-specific values =====
        // Provide safe RenderTarget default example to prevent crashes from invalid TextureView
        // handles TextureView variant requires handle to exist in ManualTextureViews
        // resource Window::Primary is always valid and references the default primary
        // window Use TeachAndRecurse to provide safe default while still exposing nested
        // mutation paths
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_CAMERA, FIELD_CAMERA_TARGET),
            TypeKnowledge::new(constants::primary_window_target_value()),
        );

        // ===== Camera3d field-specific values =====
        // Camera3dDepthTextureUsage - wrapper around u32 texture usage flags
        // Valid flags: COPY_SRC=1, COPY_DST=2, TEXTURE_BINDING=4, STORAGE_BINDING=8,
        // RENDER_ATTACHMENT=16 STORAGE_BINDING (8) causes crashes with multisampled
        // textures! Safe combinations: 16 (RENDER_ATTACHMENT only), 20 (RENDER_ATTACHMENT |
        // TEXTURE_BINDING)
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_CAMERA3D, FIELD_CAMERA3D_DEPTH_TEXTURE_USAGES),
            // RENDER_ATTACHMENT | TEXTURE_BINDING - safe combination, treat as opaque u32
            TypeKnowledge::as_root_value(json!(EXAMPLE_CAMERA3D_DEPTH_TEXTURE_USAGES), TYPE_U32),
        );

        // Screen space specular transmission steps - reasonable value to prevent memory issues
        // Default is 1, typical range is 0-4 per transmission.rs example
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_CAMERA3D,
                FIELD_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS,
            ),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS),
                TYPE_USIZE,
            ),
        );

        // ===== Transform types =====
        // GlobalTransform - wraps glam::Affine3A but serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Registry shows nested object but BRP actually expects flat array
        map.insert(
            KnowledgeKey::exact(TYPE_BEVY_GLOBAL_TRANSFORM),
            TypeKnowledge::new(json!(EXAMPLE_GLOBAL_TRANSFORM)), /* Affine matrices don't have
                                                                  * simple component access */
        );

        // Affine2 - Used in UiGlobalTransform.0, serializes as flat array of 6 f32 values
        // Format: [matrix_row1(2), matrix_row2(2), translation(2)]
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_AFFINE2),
            TypeKnowledge::new(json!(EXAMPLE_AFFINE2)),
        );

        // Affine3A - Used as GlobalTransform.0, serializes as flat array of 12 f32 values
        // Format: [matrix_row1(3), matrix_row2(3), matrix_row3(3), translation(3)]
        // Has matrix3 and translation fields but doesn't serialize with field names
        map.insert(
            KnowledgeKey::exact(TYPE_GLAM_AFFINE3A),
            TypeKnowledge::new(json!(EXAMPLE_AFFINE3A)),
        );

        // ===== Asset Handle types =====
        // Handle<T> types - use Weak variant with UUID format for mutations
        // Schema provides non-functional examples, but this format works

        // ===== WindowResolution field-specific values =====
        // Provide reasonable window dimension values to prevent GPU texture size errors
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_WINDOW_RESOLUTION,
                FIELD_WINDOW_RESOLUTION_PHYSICAL_WIDTH,
            ),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_WIDTH),
                TYPE_U32,
            ), // Reasonable window width
        );
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_WINDOW_RESOLUTION,
                FIELD_WINDOW_RESOLUTION_PHYSICAL_HEIGHT,
            ),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_HEIGHT),
                TYPE_U32,
            ), // Reasonable window height
        );

        // ===== GlyphAtlasLocation field-specific values =====
        // Provide safe glyph index to prevent crashes from out-of-bounds atlas access
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_GLYPH_ATLAS_LOCATION,
                FIELD_GLYPH_ATLAS_LOCATION_GLYPH_INDEX,
            ),
            TypeKnowledge::as_root_value(json!(EXAMPLE_GLYPH_INDEX), TYPE_USIZE),
        );

        // ===== VideoMode field-specific values =====
        // Provide realistic video mode values to prevent window system crashes
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_VIDEO_MODE, FIELD_VIDEO_MODE_BIT_DEPTH),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_VIDEO_MODE_BIT_DEPTH),
                TYPE_U16,
            ), // Standard 32-bit color
        );
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_VIDEO_MODE,
                FIELD_VIDEO_MODE_PHYSICAL_SIZE,
            ),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_VIDEO_MODE_PHYSICAL_SIZE),
                SIMPLIFIED_UVEC2,
            ), /* Standard Full HD
                * resolution */
        );
        map.insert(
            KnowledgeKey::struct_field(
                TYPE_BEVY_VIDEO_MODE,
                FIELD_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ,
            ),
            TypeKnowledge::as_root_value(
                json!(EXAMPLE_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ),
                TYPE_U32,
            ), // 60 Hz in millihertz
        );

        // ===== Bloom field-specific values =====
        // Provide safe max_mip_dimension to prevent GPU texture allocation crashes
        // Default is 512, using u32 generic value of 1_000_000 causes rendering pipeline corruption
        map.insert(
            KnowledgeKey::struct_field(TYPE_BLOOM, FIELD_BLOOM_MAX_MIP_DIMENSION),
            TypeKnowledge::as_root_value(json!(EXAMPLE_BLOOM_MAX_MIP_DIMENSION), TYPE_U32),
        );

        // ===== NonZero types =====
        // These types guarantee the value is never zero
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_U8),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_U8), SIMPLIFIED_NON_ZERO_U8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_U16),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_U16), SIMPLIFIED_NON_ZERO_U16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_U32),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_U32), SIMPLIFIED_NON_ZERO_U32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_U64),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_U64), SIMPLIFIED_NON_ZERO_U64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_U128),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_U128), SIMPLIFIED_NON_ZERO_U128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_USIZE),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_USIZE), SIMPLIFIED_NON_ZERO_USIZE),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_I8),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_I8), SIMPLIFIED_NON_ZERO_I8),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_I16),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_I16), SIMPLIFIED_NON_ZERO_I16),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_I32),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_I32), SIMPLIFIED_NON_ZERO_I32),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_I64),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_I64), SIMPLIFIED_NON_ZERO_I64),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_I128),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_I128), SIMPLIFIED_NON_ZERO_I128),
        );
        map.insert(
            KnowledgeKey::exact(TYPE_CORE_NON_ZERO_ISIZE),
            TypeKnowledge::as_root_value(json!(EXAMPLE_NON_ZERO_ISIZE), SIMPLIFIED_NON_ZERO_ISIZE),
        );

        // ===== Time<Fixed> field-specific values =====
        // wrap_period must be non-zero to prevent divide-by-zero in time wrapping calculations
        // Default is 3600 seconds (1 hour) - setting to zero causes panic in
        // run_fixed_main_schedule
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_TIME_FIXED_CONTAINER, FIELD_TIME_WRAP_PERIOD),
            TypeKnowledge::as_root_value(
                constants::duration_value(DEFAULT_WRAP_PERIOD_SECS, ZERO_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // timestep must be non-zero for fixed timestep to function
        // Default is 1/64 second (15625000 nanos) - setting to zero causes divide-by-zero panic
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_FIXED, FIELD_FIXED_TIMESTEP),
            TypeKnowledge::as_root_value(
                constants::duration_value(ZERO_SECONDS, EXAMPLE_FIXED_TIMESTEP_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // ===== Time<Virtual> field-specific values =====
        // wrap_period must be non-zero to prevent divide-by-zero in time wrapping calculations
        // Default is 3600 seconds (1 hour) - setting to zero causes app crash
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_TIME_VIRTUAL_CONTAINER, FIELD_TIME_WRAP_PERIOD),
            TypeKnowledge::as_root_value(
                constants::duration_value(DEFAULT_WRAP_PERIOD_SECS, ZERO_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // max_delta must be non-zero to allow virtual time to advance
        // Default is 250ms (250000000 nanos) - setting to zero prevents time updates
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_VIRTUAL, FIELD_VIRTUAL_MAX_DELTA),
            TypeKnowledge::as_root_value(
                constants::duration_value(ZERO_SECONDS, EXAMPLE_VIRTUAL_MAX_DELTA_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // ===== Time<Real> field-specific values =====
        // wrap_period must be non-zero to prevent divide-by-zero in time wrapping calculations
        // Default is 3600 seconds (1 hour) - setting to zero causes app crash
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_TIME_REAL_CONTAINER, FIELD_TIME_WRAP_PERIOD),
            TypeKnowledge::as_root_value(
                constants::duration_value(DEFAULT_WRAP_PERIOD_SECS, ZERO_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // ===== Time<()> field-specific values =====
        // wrap_period must be non-zero to prevent divide-by-zero in time wrapping calculations
        // Default is 3600 seconds (1 hour) - setting to zero causes app crash
        map.insert(
            KnowledgeKey::struct_field(TYPE_BEVY_TIME_EMPTY_CONTAINER, FIELD_TIME_WRAP_PERIOD),
            TypeKnowledge::as_root_value(
                constants::duration_value(DEFAULT_WRAP_PERIOD_SECS, ZERO_NANOS),
                TYPE_CORE_DURATION,
            ),
        );

        // ===== AlphaMode2d enum variant signatures =====
        // Mask(f32) variant requires alpha threshold in 0.0-1.0 range
        map.insert(
            KnowledgeKey::enum_variant_signature(
                TYPE_BEVY_ALPHA_MODE_2D,
                VariantSignature::Tuple(vec![BrpTypeName::from(TYPE_F32)]),
                ALPHA_MODE_2D_MASK_SIGNATURE_INDEX,
            ),
            TypeKnowledge::as_root_value(json!(EXAMPLE_ALPHA_MODE_2D_MASK), TYPE_F32),
        );

        map
    });
