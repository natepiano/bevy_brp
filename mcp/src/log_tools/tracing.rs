use std::str::FromStr;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;

use tracing::Level;
use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use super::lazy_file_writer::LazyFileWriter;

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
        // Suppress third-party HTTP connection logs that are noise for BRP debugging
        let target = metadata.target();
        if target.starts_with("reqwest::")
            || target.starts_with("hyper")
            || target.starts_with("h2::")
            || target.starts_with("rustls::")
            || target.starts_with("want::")
        {
            return false;
        }

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
    #[cfg(feature = "mcp-debug")]
    const fn as_u8(self) -> u8 {
        match self {
            Self::Error => 0,
            Self::Warn => 1,
            Self::Info => 2,
            Self::Debug => 3,
            Self::Trace => 4,
        }
    }

    #[cfg(feature = "mcp-debug")]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    /// Initialize file-based tracing with a fixed filename in temp directory
    /// Uses lazy file creation - file only created on first log write
    pub fn init_file_tracing() {
        let log_path = Self::get_trace_log_path();

        // Create lazy file writer that only creates file on first write
        let lazy_writer = LazyFileWriter::new(log_path);

        // Create the subscriber with dynamic filtering
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(lazy_writer)
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true);

        let subscriber = Registry::default().with(DynamicFilter).with(file_layer);

        subscriber.init();

        // Don't log anything here - it would create the file and violate "do no harm"
        // The file should only be created when the user explicitly sets a tracing level
    }

    /// Get the current tracing level
    pub fn get_current_tracing_level() -> Self {
        match CURRENT_LEVEL.load(Ordering::Relaxed) {
            0 => Self::Error,
            2 => Self::Info,
            3 => Self::Debug,
            4 => Self::Trace,
            _ => Self::Warn, // Default fallback (handles 1 and any invalid values)
        }
    }

    /// Set the current tracing level dynamically
    #[cfg(feature = "mcp-debug")]
    pub fn set_tracing_level(level: Self) {
        CURRENT_LEVEL.store(level.as_u8(), Ordering::Relaxed);

        // Log at the level that was just set
        match level {
            Self::Error => tracing::error!("Tracing level set to: error"),
            Self::Warn => tracing::warn!("Tracing level set to: warn"),
            Self::Info => tracing::info!("Tracing level set to: info"),
            Self::Debug => tracing::debug!("Tracing level set to: debug"),
            Self::Trace => tracing::trace!("Tracing level set to: trace"),
        }
    }

    /// Get the path to the trace log file
    /// Useful for testing and troubleshooting
    pub fn get_trace_log_path() -> std::path::PathBuf {
        std::env::temp_dir().join("bevy_brp_mcp_trace.log")
    }
}
