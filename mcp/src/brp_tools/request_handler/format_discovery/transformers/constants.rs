//! Constants used by format transformers

/// Expected count of f32 values in a Transform sequence
/// Transform contains: translation (Vec3), rotation (Quat), scale (Vec3) = 3 + 4 + 3 = 10 f32
/// values However, the Transform component includes additional fields that bring the total to 12
/// f32 values
pub const TRANSFORM_SEQUENCE_F32_COUNT: usize = 12;
