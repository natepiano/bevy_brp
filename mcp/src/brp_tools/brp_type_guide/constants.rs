// Agent guidance constants
/// Base warning message for AI agents about mutation paths
pub(super) const AGENT_GUIDANCE: &str = "The 'mutation_paths' field provides valid 'path' arguments for 'mcp__brp__world_mutate_components' and 'mcp__brp__world_mutate_resources' tools, with example values suitable for testing.";
/// Additional warning when Entity fields are present (with placeholder for entity ID)
pub(super) const ENTITY_WARNING: &str = " CAUTION: This type contains bevy_ecs::entity::Entity fields - you must use valid Entity IDs from the running app to replace the example value '{}'. Invalid Entity values may crash the application.";
/// Guidance for types that failed during processing
pub(super) const ERROR_GUIDANCE: &str = "This type was found in the registry but failed during processing. Check the 'error' field for details. No mutation paths or spawn format are available due to the processing failure.";

// Bevy component type constants
pub(super) const TYPE_BEVY_CAMERA: &str = "bevy_camera::camera::Camera";
pub(super) const TYPE_BEVY_ENTITY: &str = "bevy_ecs::entity::Entity";
pub(super) const TYPE_BEVY_NAME: &str = "bevy_ecs::name::Name";
pub(super) const TYPE_BLOOM: &str = "bevy_post_process::bloom::settings::Bloom";

// Bevy math type constants
pub(super) const TYPE_BEVY_MAT2: &str = "bevy_math::mat2::Mat2";
pub(super) const TYPE_BEVY_MAT3: &str = "bevy_math::mat3::Mat3";
pub(super) const TYPE_BEVY_MAT4: &str = "bevy_math::mat4::Mat4";
pub(super) const TYPE_BEVY_QUAT: &str = "bevy_math::quat::Quat";
pub(super) const TYPE_BEVY_RECT: &str = "bevy_math::rects::rect::Rect";
pub(super) const TYPE_BEVY_VEC2: &str = "bevy_math::vec2::Vec2";
pub(super) const TYPE_BEVY_VEC3: &str = "bevy_math::vec3::Vec3";
pub(super) const TYPE_BEVY_VEC3A: &str = "bevy_math::vec3a::Vec3A";
pub(super) const TYPE_BEVY_VEC4: &str = "bevy_math::vec4::Vec4";

// Example generation constants
/// Maximum recursion depth for type example generation to prevent stack overflow
pub(super) const MAX_TYPE_RECURSION_DEPTH: usize = 10;

// Glam type constants
pub(super) const TYPE_GLAM_AFFINE2: &str = "glam::Affine2";
pub(super) const TYPE_GLAM_AFFINE3A: &str = "glam::Affine3A";
pub(super) const TYPE_GLAM_IVEC2: &str = "glam::IVec2";
pub(super) const TYPE_GLAM_IVEC3: &str = "glam::IVec3";
pub(super) const TYPE_GLAM_IVEC4: &str = "glam::IVec4";
pub(super) const TYPE_GLAM_MAT2: &str = "glam::Mat2";
pub(super) const TYPE_GLAM_MAT3: &str = "glam::Mat3";
pub(super) const TYPE_GLAM_MAT3A: &str = "glam::Mat3A";
pub(super) const TYPE_GLAM_MAT4: &str = "glam::Mat4";
pub(super) const TYPE_GLAM_QUAT: &str = "glam::Quat";
pub(super) const TYPE_GLAM_UVEC2: &str = "glam::UVec2";
pub(super) const TYPE_GLAM_UVEC3: &str = "glam::UVec3";
pub(super) const TYPE_GLAM_UVEC4: &str = "glam::UVec4";
pub(super) const TYPE_GLAM_VEC2: &str = "glam::Vec2";
pub(super) const TYPE_GLAM_VEC3: &str = "glam::Vec3";
pub(super) const TYPE_GLAM_VEC3A: &str = "glam::Vec3A";
pub(super) const TYPE_GLAM_VEC4: &str = "glam::Vec4";

// Operation guidance constants
/// Guidance for `resource_example` when type is a Resource
pub(super) const INSERT_RESOURCE_GUIDANCE: &str =
    "The 'example' below can be used to insert this resource.";
/// Template for Component without spawn example (use with `OPERATION_SPAWN`)
pub(super) const NO_COMPONENT_EXAMPLE_TEMPLATE: &str =
    "This component does not have a {} example because the root mutation path is not 'mutable'.";
/// Template for Resource without insert example (use with `OPERATION_INSERT`)
pub(super) const NO_RESOURCE_EXAMPLE_TEMPLATE: &str =
    "This resource does not have an {} example because the root mutation path is not 'mutable'.";
/// Guidance for `spawn_example` when type is a Component
pub(super) const SPAWN_COMPONENT_GUIDANCE: &str =
    "The 'example' below can be used to spawn this component on an entity.";

// Operation name constants
/// Operation word for Resource default guidance
pub(super) const OPERATION_INSERT: &str = "insert";
/// Operation word for Component default guidance
pub(super) const OPERATION_SPAWN: &str = "spawn";

// Primitive type constants
pub(super) const TYPE_BOOL: &str = "bool";
pub(super) const TYPE_CHAR: &str = "char";
pub(super) const TYPE_F32: &str = "f32";
pub(super) const TYPE_F64: &str = "f64";

// Reflection trait constants
/// Reflection trait name for Bevy components
pub(super) const REFLECT_TRAIT_COMPONENT: &str = "Component";
/// Reflection trait name for Default implementation
pub(super) const REFLECT_TRAIT_DEFAULT: &str = "Default";
/// Reflection trait name for Bevy resources
pub(super) const REFLECT_TRAIT_RESOURCE: &str = "Resource";

// Signed integer type constants
pub(super) const TYPE_I128: &str = "i128";
pub(super) const TYPE_I16: &str = "i16";
pub(super) const TYPE_I32: &str = "i32";
pub(super) const TYPE_I64: &str = "i64";
pub(super) const TYPE_I8: &str = "i8";
pub(super) const TYPE_ISIZE: &str = "isize";

// String type constants
pub(super) const TYPE_ALLOC_STRING: &str = "alloc::string::String";
pub(super) const TYPE_STD_STRING: &str = "std::string::String";
pub(super) const TYPE_STR: &str = "str";
pub(super) const TYPE_STR_REF: &str = "&str";
pub(super) const TYPE_STRING: &str = "String";

// Time type constants
pub(super) const TYPE_CORE_DURATION: &str = "core::time::Duration";

// Unsigned integer type constants
pub(super) const TYPE_U128: &str = "u128";
pub(super) const TYPE_U16: &str = "u16";
pub(super) const TYPE_U32: &str = "u32";
pub(super) const TYPE_U64: &str = "u64";
pub(super) const TYPE_U8: &str = "u8";
pub(super) const TYPE_USIZE: &str = "usize";
