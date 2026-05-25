// large response token calculation constants
/// Estimated characters per token for response size calculation
pub(super) const CHARS_PER_TOKEN: usize = 4;
/// Default maximum tokens before saving to file.
///
/// This is intentionally below current agent context windows because MCP and
/// model token counting can differ from our 4 chars/token estimate.
pub(super) const DEFAULT_MAX_RESPONSE_TOKENS: usize = 25_000;

// response placeholders
pub(super) const ENTITY_COUNT_PLACEHOLDER: &str = "entity_count";
pub(super) const RESULT_PLACEHOLDER: &str = "result";

// response sentinel values
pub(super) const SKIP_NULL_FIELD_SENTINEL: &str = "__SKIP_NULL_FIELD__";

// response tracking fields
pub(super) const OPTIONAL_PARAMETERS_NOT_PROVIDED_FIELD: &str = "optional_parameters_not_provided";
