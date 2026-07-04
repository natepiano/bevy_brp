// byte formatting
pub(super) const BYTES_PER_UNIT: f64 = 1024.0;
pub(super) const UNITS: &[&str] = &["B", "KB", "MB", "GB"];

// log filenames
pub(super) const LOG_EXTENSION: &str = ".log";
pub(super) const LOG_PREFIX: &str = "bevy_brp_mcp_";
pub(super) const TRACE_LOG_FILENAME: &str = "bevy_brp_mcp_trace.log";

// tracing filter constants
/// Third-party HTTP/transport crate name prefixes whose tracing events are
/// suppressed because they are noise for BRP debugging.
pub(super) const TRACING_FILTERED_TARGET_PREFIXES: &[&str] =
    &["reqwest::", "hyper", "h2::", "rustls::", "want::"];
