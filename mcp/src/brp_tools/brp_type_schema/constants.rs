// ============================================================================
// SCHEMA CONSTANTS
// ============================================================================

/// JSON Schema reference prefix for type definitions
pub const SCHEMA_REF_PREFIX: &str = "#/$defs/";

// ============================================================================
// EXAMPLE GENERATION CONSTANTS
// ============================================================================

/// Default size for generated example arrays when size cannot be parsed
pub const DEFAULT_EXAMPLE_ARRAY_SIZE: usize = 3;

/// Maximum size for generated example arrays to prevent excessive memory usage
pub const MAX_EXAMPLE_ARRAY_SIZE: usize = 10;

/// Maximum recursion depth for type example generation to prevent stack overflow
pub const MAX_TYPE_RECURSION_DEPTH: usize = 10;
