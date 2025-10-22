// ============================================================================
// EXAMPLE GENERATION CONSTANTS
// ============================================================================

/// Maximum recursion depth for type example generation to prevent stack overflow
pub const MAX_TYPE_RECURSION_DEPTH: usize = 10;

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
pub const TYPE_GLAM_MAT3A: &str = "glam::Mat3A";
pub const TYPE_GLAM_MAT4: &str = "glam::Mat4";
pub const TYPE_GLAM_AFFINE2: &str = "glam::Affine2";
pub const TYPE_GLAM_AFFINE3A: &str = "glam::Affine3A";

// Bevy component types
pub const TYPE_BEVY_ENTITY: &str = "bevy_ecs::entity::Entity";
pub const TYPE_BEVY_COLOR: &str = "bevy_color::color::Color";
pub const TYPE_BEVY_COLOR_SRGBA: &str = "bevy_color::srgba::Srgba";
pub const TYPE_BEVY_COLOR_LINEAR_RGBA: &str = "bevy_color::linear_rgba::LinearRgba";
pub const TYPE_BEVY_COLOR_HSLA: &str = "bevy_color::hsla::Hsla";
pub const TYPE_BEVY_COLOR_HSVA: &str = "bevy_color::hsva::Hsva";
pub const TYPE_BEVY_COLOR_HWBA: &str = "bevy_color::hwba::Hwba";
pub const TYPE_BEVY_COLOR_LABA: &str = "bevy_color::laba::Laba";
pub const TYPE_BEVY_COLOR_LCHA: &str = "bevy_color::lcha::Lcha";
pub const TYPE_BEVY_COLOR_OKLABA: &str = "bevy_color::oklaba::Oklaba";
pub const TYPE_BEVY_COLOR_OKLCHA: &str = "bevy_color::oklcha::Oklcha";
pub const TYPE_BEVY_COLOR_XYZA: &str = "bevy_color::xyza::Xyza";
pub const TYPE_BEVY_NAME: &str = "bevy_ecs::name::Name";
pub const TYPE_BLOOM: &str = "bevy_post_process::bloom::settings::Bloom";
pub const TYPE_BEVY_CAMERA: &str = "bevy_camera::camera::Camera";
pub const TYPE_BEVY_RENDER_TARGET: &str = "bevy_camera::camera::RenderTarget";

// ============================================================================
// REFLECTION TRAIT CONSTANTS
// ============================================================================

/// Reflection trait name for Bevy components
pub const REFLECT_TRAIT_COMPONENT: &str = "Component";

/// Reflection trait name for Bevy resources
pub const REFLECT_TRAIT_RESOURCE: &str = "Resource";

/// Reflection trait name for Default implementation
pub const REFLECT_TRAIT_DEFAULT: &str = "Default";

// ============================================================================
// AGENT WARNING MESSAGES
// ============================================================================

/// Base warning message for AI agents about mutation paths
pub const AGENT_GUIDANCE: &str = "The 'mutation_paths' field provides valid 'path' arguments for 'mcp__brp__world_mutate_components' and 'mcp__brp__world_mutate_resources' tools, with example values suitable for testing.";

/// Additional warning when Entity fields are present (with placeholder for entity ID)
pub const ENTITY_WARNING: &str = " CAUTION: This type contains bevy_ecs::entity::Entity fields - you must use valid Entity IDs from the running app to replace the example value '{}'. Invalid Entity values may crash the application.";

/// Guidance for types that failed during processing
pub const ERROR_GUIDANCE: &str = "This type was found in the registry but failed during processing. Check the 'error' field for details. No mutation paths or spawn format are available due to the processing failure.";

/// Guidance appended to root path description for Default trait spawning
pub const DEFAULT_SPAWN_GUIDANCE: &str = " However this type implements Default and accepts empty object {} for spawn, insert, or mutate operations on the root path";

// ============================================================================
// PATH PROCESSING CONSTANTS
// ============================================================================
