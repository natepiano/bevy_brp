// array example constants
/// Default element count used when an array's size cannot be inferred from its type name
pub(super) const DEFAULT_ARRAY_EXAMPLE_LENGTH: usize = 2;

// child descriptors
pub(super) const FIRST_ELEMENT_DESCRIPTOR: &str = "0";

// option type classification
pub(super) const OPTION_PREFIX: &str = "core::option::Option<";
pub(super) const OPTION_SOME_FIELD: &str = "Some";
pub(super) const OPTION_SUFFIX: char = '>';

// response fields
pub(super) const RESPONSE_AGENT_GUIDANCE_FIELD: &str = "agent_guidance";
pub(super) const RESPONSE_EXAMPLE_FIELD: &str = "example";
pub(super) const RESPONSE_EXAMPLES_FIELD: &str = "examples";
pub(super) const RESPONSE_RESOURCE_FIELD: &str = "resource";
pub(super) const RESPONSE_SPAWN_FIELD: &str = "spawn";
