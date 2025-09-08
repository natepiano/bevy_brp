// ============================================================================
// SCHEMA CONSTANTS
// ============================================================================

/// JSON Schema reference prefix for type definitions
pub const SCHEMA_REF_PREFIX: &str = "#/$defs/";

// ============================================================================
// EXAMPLE GENERATION CONSTANTS
// ============================================================================

use std::ops::Deref;

/// Default size for generated example arrays when size cannot be parsed
pub const DEFAULT_EXAMPLE_ARRAY_SIZE: usize = 3;

/// Maximum size for generated example arrays to prevent excessive memory usage
pub const MAX_EXAMPLE_ARRAY_SIZE: usize = 10;

/// Maximum recursion depth for type example generation to prevent stack overflow
pub const MAX_TYPE_RECURSION_DEPTH: usize = 10;

/// Type-safe wrapper for recursion depth tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecursionDepth(usize);

impl RecursionDepth {
    pub const ZERO: Self = Self(0);

    pub const fn increment(self) -> Self {
        Self(self.0 + 1)
    }

    pub const fn exceeds_limit(self) -> bool {
        self.0 > MAX_TYPE_RECURSION_DEPTH
    }

    /// Create a new `RecursionDepth` from a usize value
    pub const fn from_usize(depth: usize) -> Self {
        Self(depth)
    }

    /// Get the current depth value for debugging
    pub const fn current(self) -> usize {
        self.0
    }
}

// Allow direct comparison with integers
impl Deref for RecursionDepth {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// TYPE NAME CONSTANTS
// ============================================================================

// Primitive types
pub const TYPE_I8: &str = "i8";
pub const TYPE_I16: &str = "i16";
pub const TYPE_I32: &str = "i32";
pub const TYPE_I64: &str = "i64";
pub const TYPE_I128: &str = "i128";
pub const TYPE_U8: &str = "u8";
pub const TYPE_U16: &str = "u16";
pub const TYPE_U32: &str = "u32";
pub const TYPE_U64: &str = "u64";
pub const TYPE_U128: &str = "u128";
pub const TYPE_F32: &str = "f32";
pub const TYPE_F64: &str = "f64";
pub const TYPE_ISIZE: &str = "isize";
pub const TYPE_USIZE: &str = "usize";
pub const TYPE_BOOL: &str = "bool";
pub const TYPE_CHAR: &str = "char";

// String types
pub const TYPE_ALLOC_STRING: &str = "alloc::string::String";
pub const TYPE_STD_STRING: &str = "std::string::String";
pub const TYPE_STRING: &str = "String";
pub const TYPE_STR_REF: &str = "&str";
pub const TYPE_STR: &str = "str";

// Bevy math types
pub const TYPE_BEVY_VEC2: &str = "bevy_math::vec2::Vec2";
pub const TYPE_BEVY_VEC3: &str = "bevy_math::vec3::Vec3";
pub const TYPE_BEVY_VEC3A: &str = "bevy_math::vec3a::Vec3A";
pub const TYPE_BEVY_VEC4: &str = "bevy_math::vec4::Vec4";
pub const TYPE_BEVY_QUAT: &str = "bevy_math::quat::Quat";
pub const TYPE_BEVY_MAT2: &str = "bevy_math::mat2::Mat2";
pub const TYPE_BEVY_MAT3: &str = "bevy_math::mat3::Mat3";
pub const TYPE_BEVY_MAT4: &str = "bevy_math::mat4::Mat4";
pub const TYPE_BEVY_RECT: &str = "bevy_math::rects::rect::Rect";

// Glam types
pub const TYPE_GLAM_VEC2: &str = "glam::Vec2";
pub const TYPE_GLAM_VEC3: &str = "glam::Vec3";
pub const TYPE_GLAM_VEC3A: &str = "glam::Vec3A";
pub const TYPE_GLAM_VEC4: &str = "glam::Vec4";
pub const TYPE_GLAM_IVEC2: &str = "glam::IVec2";
pub const TYPE_GLAM_IVEC3: &str = "glam::IVec3";
pub const TYPE_GLAM_IVEC4: &str = "glam::IVec4";
pub const TYPE_GLAM_UVEC2: &str = "glam::UVec2";
pub const TYPE_GLAM_UVEC3: &str = "glam::UVec3";
pub const TYPE_GLAM_UVEC4: &str = "glam::UVec4";
pub const TYPE_GLAM_QUAT: &str = "glam::Quat";
pub const TYPE_GLAM_MAT2: &str = "glam::Mat2";
pub const TYPE_GLAM_MAT3: &str = "glam::Mat3";
pub const TYPE_GLAM_MAT4: &str = "glam::Mat4";

// Bevy component types
pub const TYPE_BEVY_COLOR: &str = "bevy_color::color::Color";
pub const TYPE_BEVY_NAME: &str = "bevy_ecs::name::Name";
pub const TYPE_BEVY_IMAGE_HANDLE: &str = "bevy_asset::handle::Handle<bevy_image::image::Image>";
