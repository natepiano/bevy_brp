// Large response token calculation constants
/// Estimated characters per token for response size calculation
pub(super) const CHARS_PER_TOKEN: usize = 4;
/// Default maximum tokens before saving to file (Claude Code MCP limitation)
/// Using 10,000 as a conservative buffer below the 25,000 hard limit
/// (MCP seems to count tokens differently than our 4 chars/token estimate)
pub(super) const DEFAULT_MAX_RESPONSE_TOKENS: usize = 15_000;
