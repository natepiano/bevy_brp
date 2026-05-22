use serde_json::Map;
use serde_json::Value;

use crate::support::JsonObjectAccess;

// agent guidance constants
/// Base warning message for AI agents about mutation paths
pub(super) const AGENT_GUIDANCE: &str = "The 'mutation_paths' field provides valid 'path' arguments for 'mcp__brp__world_mutate_components' and 'mcp__brp__world_mutate_resources' tools, with example values suitable for testing.";
/// Additional warning when Entity fields are present (with placeholder for entity ID)
pub(super) const ENTITY_WARNING: &str = " CAUTION: This type contains bevy_ecs::entity::Entity fields - you must use valid Entity IDs from the running app to replace the example value '{}'. Invalid Entity values may crash the application.";
/// Guidance for types that failed during processing
pub(super) const ERROR_GUIDANCE: &str = "This type was found in the registry but failed during processing. Check the 'error' field for details. No mutation paths or spawn format are available due to the processing failure.";

// bevy component type constants
pub(super) const BEVY_ASSET_HANDLE_PREFIX: &str = "bevy_asset::handle::Handle<";
pub(super) const TYPE_BEVY_ALPHA_MODE_2D: &str =
    "bevy_sprite_render::mesh2d::material::AlphaMode2d";
pub(super) const TYPE_BEVY_CAMERA: &str = "bevy_camera::camera::Camera";
pub(super) const TYPE_BEVY_CAMERA3D: &str = "bevy_camera::components::Camera3d";
pub(super) const TYPE_BEVY_ENTITY: &str = "bevy_ecs::entity::Entity";
pub(super) const TYPE_BEVY_GLOBAL_TRANSFORM: &str =
    "bevy_transform::components::global_transform::GlobalTransform";
pub(super) const TYPE_BEVY_GLYPH_ATLAS_LOCATION: &str = "bevy_text::glyph::GlyphAtlasLocation";
pub(super) const TYPE_BEVY_NAME: &str = "bevy_ecs::name::Name";
pub(super) const TYPE_BEVY_VIDEO_MODE: &str = "bevy_window::monitor::VideoMode";
pub(super) const TYPE_BEVY_WINDOW_RESOLUTION: &str = "bevy_window::window::WindowResolution";
pub(super) const TYPE_BLOOM: &str = "bevy_post_process::bloom::settings::Bloom";

// bevy math type constants
pub(super) const TYPE_BEVY_MAT2: &str = "bevy_math::mat2::Mat2";
pub(super) const TYPE_BEVY_MAT3: &str = "bevy_math::mat3::Mat3";
pub(super) const TYPE_BEVY_MAT4: &str = "bevy_math::mat4::Mat4";
pub(super) const TYPE_BEVY_QUAT: &str = "bevy_math::quat::Quat";
pub(super) const TYPE_BEVY_RECT: &str = "bevy_math::rects::rect::Rect";
pub(super) const TYPE_BEVY_VEC2: &str = "bevy_math::vec2::Vec2";
pub(super) const TYPE_BEVY_VEC3: &str = "bevy_math::vec3::Vec3";
pub(super) const TYPE_BEVY_VEC3A: &str = "bevy_math::vec3a::Vec3A";
pub(super) const TYPE_BEVY_VEC4: &str = "bevy_math::vec4::Vec4";

// bevy time default values
/// Default value for `Time<_>::wrap_period` (one hour, in seconds). Bevy panics
/// or stalls when this is zero, so the BRP type guide ships a non-zero default.
pub(super) const DEFAULT_WRAP_PERIOD_SECS: u64 = 3600;

// bevy time type constants
pub(super) const TYPE_BEVY_FIXED: &str = "bevy_time::fixed::Fixed";
pub(super) const TYPE_BEVY_TIME_EMPTY_CONTAINER: &str = "bevy_time::time::Time<()>";
pub(super) const TYPE_BEVY_TIME_FIXED_CONTAINER: &str =
    "bevy_time::time::Time<bevy_time::fixed::Fixed>";
pub(super) const TYPE_BEVY_TIME_REAL_CONTAINER: &str =
    "bevy_time::time::Time<bevy_time::real::Real>";
pub(super) const TYPE_BEVY_TIME_VIRTUAL_CONTAINER: &str =
    "bevy_time::time::Time<bevy_time::virt::Virtual>";
pub(super) const TYPE_BEVY_VIRTUAL: &str = "bevy_time::virt::Virtual";

// example generation constants
/// Maximum recursion depth for type example generation to prevent stack overflow
pub(super) const MAX_TYPE_RECURSION_DEPTH: usize = 10;

// glam type constants
pub(super) const TYPE_GLAM_AFFINE2: &str = "glam::Affine2";
pub(super) const TYPE_GLAM_AFFINE3A: &str = "glam::Affine3A";
pub(super) const TYPE_GLAM_DVEC2: &str = "glam::DVec2";
pub(super) const TYPE_GLAM_DVEC3: &str = "glam::DVec3";
pub(super) const TYPE_GLAM_DVEC4: &str = "glam::DVec4";
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

// json fields
pub(super) const DURATION_FIELD_NANOS: &str = "nanos";
pub(super) const DURATION_FIELD_SECS: &str = "secs";
pub(super) const MUTABLE_FIELD: &str = "mutable";
pub(super) const MUTABILITY_MESSAGE_FIELD: &str = "message";
pub(super) const NOT_MUTABLE_FIELD: &str = "not_mutable";
pub(super) const PARTIALLY_MUTABLE_FIELD: &str = "partially_mutable";
pub(super) const WINDOW_TARGET_FIELD: &str = "Window";
pub(super) const WINDOW_TARGET_PRIMARY: &str = "Primary";

// non-zero integer type constants
pub(super) const TYPE_CORE_NON_ZERO_I8: &str = "core::num::NonZeroI8";
pub(super) const TYPE_CORE_NON_ZERO_I16: &str = "core::num::NonZeroI16";
pub(super) const TYPE_CORE_NON_ZERO_I32: &str = "core::num::NonZeroI32";
pub(super) const TYPE_CORE_NON_ZERO_I64: &str = "core::num::NonZeroI64";
pub(super) const TYPE_CORE_NON_ZERO_I128: &str = "core::num::NonZeroI128";
pub(super) const TYPE_CORE_NON_ZERO_ISIZE: &str = "core::num::NonZeroIsize";
pub(super) const TYPE_CORE_NON_ZERO_U8: &str = "core::num::NonZeroU8";
pub(super) const TYPE_CORE_NON_ZERO_U16: &str = "core::num::NonZeroU16";
pub(super) const TYPE_CORE_NON_ZERO_U32: &str = "core::num::NonZeroU32";
pub(super) const TYPE_CORE_NON_ZERO_U64: &str = "core::num::NonZeroU64";
pub(super) const TYPE_CORE_NON_ZERO_U128: &str = "core::num::NonZeroU128";
pub(super) const TYPE_CORE_NON_ZERO_USIZE: &str = "core::num::NonZeroUsize";

// operation guidance constants
/// Guidance for `resource` when type is a Resource
pub(super) const INSERT_RESOURCE_GUIDANCE: &str =
    "The 'example' below can be used to insert this resource.";
/// Template for Component without spawn example (use with `OPERATION_SPAWN`)
pub(super) const NO_COMPONENT_EXAMPLE_TEMPLATE: &str =
    "This component does not have a {} example because the root mutation path is not 'mutable'.";
/// Template for Resource without insert example (use with `OPERATION_INSERT`)
pub(super) const NO_RESOURCE_EXAMPLE_TEMPLATE: &str =
    "This resource does not have an {} example because the root mutation path is not 'mutable'.";
/// Guidance for `spawn` when type is a Component
pub(super) const SPAWN_COMPONENT_GUIDANCE: &str =
    "The 'example' below can be used to spawn this component on an entity.";

// operation name constants
/// Operation word for Resource default guidance
pub(super) const OPERATION_INSERT: &str = "insert";
/// Operation word for Component default guidance
pub(super) const OPERATION_SPAWN: &str = "spawn";

// primitive type constants
pub(super) const TYPE_BOOL: &str = "bool";
pub(super) const TYPE_CHAR: &str = "char";
pub(super) const TYPE_F32: &str = "f32";
pub(super) const TYPE_F64: &str = "f64";
pub(super) const TYPE_UNIT: &str = "()";

// reflection trait constants
/// Reflection trait name for Bevy components
pub(super) const REFLECT_TRAIT_COMPONENT: &str = "Component";
/// Reflection trait name for Default implementation
pub(super) const REFLECT_TRAIT_DEFAULT: &str = "Default";
/// Reflection trait name for Bevy resources
pub(super) const REFLECT_TRAIT_RESOURCE: &str = "Resource";

// signed integer type constants
pub(super) const TYPE_I128: &str = "i128";
pub(super) const TYPE_I16: &str = "i16";
pub(super) const TYPE_I32: &str = "i32";
pub(super) const TYPE_I64: &str = "i64";
pub(super) const TYPE_I8: &str = "i8";
pub(super) const TYPE_ISIZE: &str = "isize";

// string type constants
pub(super) const TYPE_ALLOC_STRING: &str = "alloc::string::String";
pub(super) const TYPE_STD_STRING: &str = "std::string::String";
pub(super) const TYPE_STR: &str = "str";
pub(super) const TYPE_STR_REF: &str = "&str";
pub(super) const TYPE_STRING: &str = "String";

// time type constants
pub(super) const TYPE_CORE_DURATION: &str = "core::time::Duration";

// type knowledge example arrays
pub(super) const EXAMPLE_AFFINE2: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
pub(super) const EXAMPLE_AFFINE3A: [f32; 12] =
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
pub(super) const EXAMPLE_DVEC2: [f64; 2] = [1.0, 2.0];
pub(super) const EXAMPLE_DVEC3: [f64; 3] = [1.0, 2.0, 3.0];
pub(super) const EXAMPLE_DVEC4: [f64; 4] = [1.0, 2.0, 3.0, 4.0];
pub(super) const EXAMPLE_GLOBAL_TRANSFORM: [f32; 12] =
    [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
pub(super) const EXAMPLE_IVEC2: [i64; 2] = [0, 0];
pub(super) const EXAMPLE_IVEC3: [i64; 3] = [0, 0, 0];
pub(super) const EXAMPLE_IVEC4: [i64; 4] = [0, 0, 0, 0];
pub(super) const EXAMPLE_MAT2: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub(super) const EXAMPLE_MAT3: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
pub(super) const EXAMPLE_MAT4: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
pub(super) const EXAMPLE_QUAT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub(super) const EXAMPLE_RECT_MAX: [f32; 2] = [100.0, 100.0];
pub(super) const EXAMPLE_RECT_MIN: [f32; 2] = [0.0, 0.0];
pub(super) const EXAMPLE_UVEC2: [u64; 2] = [0, 0];
pub(super) const EXAMPLE_UVEC3: [u64; 3] = [0, 0, 0];
pub(super) const EXAMPLE_UVEC4: [u64; 4] = [0, 0, 0, 0];
pub(super) const EXAMPLE_VEC2: [f32; 2] = [1.0, 2.0];
pub(super) const EXAMPLE_VEC3: [f32; 3] = [1.0, 2.0, 3.0];
pub(super) const EXAMPLE_VEC4: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
pub(super) const EXAMPLE_VIDEO_MODE_PHYSICAL_SIZE: [u64; 2] = [1920, 1080];

// type knowledge example fields
pub(super) const FIELD_BLOOM_MAX_MIP_DIMENSION: &str = "max_mip_dimension";
pub(super) const FIELD_CAMERA3D_DEPTH_TEXTURE_USAGES: &str = "depth_texture_usages";
pub(super) const FIELD_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS: &str =
    "screen_space_specular_transmission_steps";
pub(super) const FIELD_CAMERA_TARGET: &str = "target";
pub(super) const FIELD_FIXED_TIMESTEP: &str = "timestep";
pub(super) const FIELD_GLYPH_ATLAS_LOCATION_GLYPH_INDEX: &str = "glyph_index";
pub(super) const FIELD_RECT_MAX: &str = "max";
pub(super) const FIELD_RECT_MIN: &str = "min";
pub(super) const FIELD_TIME_WRAP_PERIOD: &str = "wrap_period";
pub(super) const FIELD_VIDEO_MODE_BIT_DEPTH: &str = "bit_depth";
pub(super) const FIELD_VIDEO_MODE_PHYSICAL_SIZE: &str = "physical_size";
pub(super) const FIELD_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ: &str = "refresh_rate_millihertz";
pub(super) const FIELD_VIRTUAL_MAX_DELTA: &str = "max_delta";
pub(super) const FIELD_WINDOW_RESOLUTION_PHYSICAL_HEIGHT: &str = "physical_height";
pub(super) const FIELD_WINDOW_RESOLUTION_PHYSICAL_WIDTH: &str = "physical_width";

// type knowledge example scalars
pub(super) const ALPHA_MODE_2D_MASK_SIGNATURE_INDEX: usize = 0;
pub(super) const EXAMPLE_ALPHA_MODE_2D_MASK: f32 = 0.5;
pub(super) const EXAMPLE_BLOOM_MAX_MIP_DIMENSION: u64 = 512;
pub(super) const EXAMPLE_BOOL: bool = true;
pub(super) const EXAMPLE_CAMERA3D_DEPTH_TEXTURE_USAGES: u64 = 20;
pub(super) const EXAMPLE_CAMERA3D_SCREEN_SPACE_SPECULAR_TRANSMISSION_STEPS: u64 = 1;
pub(super) const EXAMPLE_CHAR: char = 'A';
pub(super) const EXAMPLE_ENTITY_BITS: u64 = 8_589_934_670;
pub(super) const EXAMPLE_F32: f32 = 1.0;
pub(super) const EXAMPLE_F64: f64 = 1.0;
pub(super) const EXAMPLE_FIXED_TIMESTEP_NANOS: u32 = 15_625_000;
pub(super) const EXAMPLE_GLYPH_INDEX: u64 = 5;
pub(super) const EXAMPLE_I128: &str = "123456789012345678901234567890";
pub(super) const EXAMPLE_I16: i64 = 1;
pub(super) const EXAMPLE_I32: i64 = 1;
pub(super) const EXAMPLE_I64: i64 = 1;
pub(super) const EXAMPLE_I8: i64 = 42;
pub(super) const EXAMPLE_ISIZE: i64 = 1;
pub(super) const EXAMPLE_NAME: &str = "Entity Name";
pub(super) const EXAMPLE_NON_ZERO_I128: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_I16: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_I32: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_I64: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_I8: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_ISIZE: i64 = 1;
pub(super) const EXAMPLE_NON_ZERO_U128: u64 = 1;
pub(super) const EXAMPLE_NON_ZERO_U16: u64 = 1;
pub(super) const EXAMPLE_NON_ZERO_U32: u64 = 1;
pub(super) const EXAMPLE_NON_ZERO_U64: u64 = 1;
pub(super) const EXAMPLE_NON_ZERO_U8: u64 = 1;
pub(super) const EXAMPLE_NON_ZERO_USIZE: u64 = 1;
pub(super) const EXAMPLE_STATIC_STR: &str = "static string";
pub(super) const EXAMPLE_STRING: &str = "Hello, World!";
pub(super) const EXAMPLE_U128: &str = "987654321098765432109876543210";
pub(super) const EXAMPLE_U16: u64 = 5000;
pub(super) const EXAMPLE_U32: u32 = 1;
pub(super) const EXAMPLE_U64: u64 = 1;
pub(super) const EXAMPLE_U8: u64 = 128;
pub(super) const EXAMPLE_UNIT_ARRAY: [u64; 0] = [];
pub(super) const EXAMPLE_USIZE: u64 = 2;
pub(super) const EXAMPLE_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub(super) const EXAMPLE_VIDEO_MODE_BIT_DEPTH: u64 = 32;
pub(super) const EXAMPLE_VIDEO_MODE_REFRESH_RATE_MILLIHERTZ: u64 = 60000;
pub(super) const EXAMPLE_VIRTUAL_MAX_DELTA_NANOS: u32 = 250_000_000;
pub(super) const EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_HEIGHT: u64 = 600;
pub(super) const EXAMPLE_WINDOW_RESOLUTION_PHYSICAL_WIDTH: u64 = 800;
pub(super) const ZERO_NANOS: u32 = 0;
pub(super) const ZERO_SECONDS: u64 = 0;

// type knowledge simplified types
pub(super) const SIMPLIFIED_NON_ZERO_I128: &str = "NonZeroI128";
pub(super) const SIMPLIFIED_NON_ZERO_I16: &str = "NonZeroI16";
pub(super) const SIMPLIFIED_NON_ZERO_I32: &str = "NonZeroI32";
pub(super) const SIMPLIFIED_NON_ZERO_I64: &str = "NonZeroI64";
pub(super) const SIMPLIFIED_NON_ZERO_I8: &str = "NonZeroI8";
pub(super) const SIMPLIFIED_NON_ZERO_ISIZE: &str = "NonZeroIsize";
pub(super) const SIMPLIFIED_NON_ZERO_U128: &str = "NonZeroU128";
pub(super) const SIMPLIFIED_NON_ZERO_U16: &str = "NonZeroU16";
pub(super) const SIMPLIFIED_NON_ZERO_U32: &str = "NonZeroU32";
pub(super) const SIMPLIFIED_NON_ZERO_U64: &str = "NonZeroU64";
pub(super) const SIMPLIFIED_NON_ZERO_U8: &str = "NonZeroU8";
pub(super) const SIMPLIFIED_NON_ZERO_USIZE: &str = "NonZeroUsize";
pub(super) const SIMPLIFIED_UNIT: &str = "()";
pub(super) const SIMPLIFIED_UUID: &str = "Uuid";
pub(super) const SIMPLIFIED_UVEC2: &str = "UVec2";

// unsigned integer type constants
pub(super) const TYPE_U128: &str = "u128";
pub(super) const TYPE_U16: &str = "u16";
pub(super) const TYPE_U32: &str = "u32";
pub(super) const TYPE_U64: &str = "u64";
pub(super) const TYPE_U8: &str = "u8";
pub(super) const TYPE_USIZE: &str = "usize";

// uuid type constants
pub(super) const TYPE_UUID: &str = "uuid::Uuid";

pub(super) fn duration_value(seconds: u64, nanoseconds: u32) -> Value {
    let mut duration = Map::new();
    duration.insert_field(DURATION_FIELD_NANOS, nanoseconds);
    duration.insert_field(DURATION_FIELD_SECS, seconds);
    Value::Object(duration)
}

pub(super) fn primary_window_target_value() -> Value {
    let mut target = Map::new();
    target.insert_field(WINDOW_TARGET_FIELD, WINDOW_TARGET_PRIMARY);
    Value::Object(target)
}

pub(super) fn rect_value() -> Value {
    let mut rect = Map::new();
    rect.insert_field(FIELD_RECT_MIN, EXAMPLE_RECT_MIN);
    rect.insert_field(FIELD_RECT_MAX, EXAMPLE_RECT_MAX);
    Value::Object(rect)
}
