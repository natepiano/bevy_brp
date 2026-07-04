// error response fields
pub(super) const CALL_INFO_FIELD: &str = "call_info";
pub(super) const ERROR_STATUS: &str = "error";
pub(super) const MESSAGE_FIELD: &str = "message";
pub(super) const STATUS_FIELD: &str = "status";

// large response fields
pub(super) const FILEPATH_FIELD: &str = "filepath";
pub(super) const INSTRUCTIONS_FIELD: &str = "instructions";
pub(super) const LARGE_RESPONSE_INSTRUCTIONS: &str =
    "Use Read tool to examine, Grep to search, or jq commands to filter the data.";
pub(super) const ORIGINAL_SIZE_TOKENS_FIELD: &str = "original_size_tokens";
pub(super) const SAVED_TO_FILE_FIELD: &str = "saved_to_file";

// large response filename constants
pub(super) const LARGE_RESPONSE_FILENAME_REPLACEMENT: &str = "_";
pub(super) const LARGE_RESPONSE_FILENAME_SANITIZE_CHARS: [char; 2] = ['/', ' '];

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

// schema probes
pub(super) const VALUE_TYPE_NAME: &str = "Value";
