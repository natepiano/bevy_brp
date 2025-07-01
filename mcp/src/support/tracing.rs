use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};

use tracing::{Level, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

static CURRENT_LEVEL: AtomicU8 = AtomicU8::new(1); // Default to WARN level (1) for "do no harm"

/// Dynamic tracing filter that can be updated at runtime
#[derive(Clone)]
pub struct DynamicFilter;

impl<S> Layer<S> for DynamicFilter
where
    S: Subscriber,
{
    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let current_level = CURRENT_LEVEL.load(Ordering::Relaxed);
        let level_value = match *metadata.level() {
            Level::ERROR => 0,
            Level::WARN => 1,
            Level::INFO => 2,
            Level::DEBUG => 3,
            Level::TRACE => 4,
        };
        level_value <= current_level
    }
}

/// Represents tracing levels that can be set dynamically
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl FromStr for TracingLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!(
                "Invalid tracing level '{s}'. Valid levels are: error, warn, info, debug, trace"
            )),
        }
    }
}

impl TracingLevel {
    const fn as_u8(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Debug => 3,
            Self::Trace => 4,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

/// Initialize file-based tracing with a fixed filename in temp directory
/// Returns a `WorkerGuard` that must be kept alive for logging to work
pub fn init_file_tracing() -> WorkerGuard {
    let temp_dir = std::env::temp_dir();

    // Create file appender
    let file_appender = tracing_appender::rolling::never(&temp_dir, "bevy_brp_mcp_trace.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Create the subscriber with dynamic filtering
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    let subscriber = Registry::default().with(DynamicFilter).with(file_layer);

    subscriber.init();

    // Don't log anything here - it would create the file and violate "do no harm"
    // The file should only be created when the user explicitly sets a tracing level

    guard
}

/// Set the current tracing level dynamically
pub fn set_tracing_level(level: TracingLevel) {
    CURRENT_LEVEL.store(level.as_u8(), Ordering::Relaxed);
    tracing::info!("Tracing level set to: {}", level.as_str());
}

/// Get the current tracing level
pub fn get_current_tracing_level() -> TracingLevel {
    match CURRENT_LEVEL.load(Ordering::Relaxed) {
        0 => TracingLevel::Error,
        2 => TracingLevel::Info,
        3 => TracingLevel::Debug,
        4 => TracingLevel::Trace,
        _ => TracingLevel::Warn, // Default fallback (handles 1 and any invalid values)
    }
}

/// Get the path to the trace log file
/// Useful for testing and troubleshooting
pub fn get_trace_log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("bevy_brp_mcp_trace.log")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_level_from_str() {
        assert!(matches!(
            TracingLevel::from_str("error"),
            Ok(TracingLevel::Error)
        ));
        assert!(matches!(
            TracingLevel::from_str("ERROR"),
            Ok(TracingLevel::Error)
        ));
        assert!(matches!(
            TracingLevel::from_str("warn"),
            Ok(TracingLevel::Warn)
        ));
        assert!(matches!(
            TracingLevel::from_str("info"),
            Ok(TracingLevel::Info)
        ));
        assert!(matches!(
            TracingLevel::from_str("debug"),
            Ok(TracingLevel::Debug)
        ));
        assert!(matches!(
            TracingLevel::from_str("trace"),
            Ok(TracingLevel::Trace)
        ));

        assert!(TracingLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_tracing_level_as_str() {
        assert_eq!(TracingLevel::Error.as_str(), "error");
        assert_eq!(TracingLevel::Warn.as_str(), "warn");
        assert_eq!(TracingLevel::Info.as_str(), "info");
        assert_eq!(TracingLevel::Debug.as_str(), "debug");
        assert_eq!(TracingLevel::Trace.as_str(), "trace");
    }
}
