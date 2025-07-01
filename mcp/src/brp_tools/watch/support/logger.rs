//! Optimized watch logging with buffering and batching

use std::fmt::Write;
use std::path::PathBuf;

use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// Log entry to be written
#[derive(Debug)]
pub struct LogEntry {
    pub update_type: String,
    pub data:        serde_json::Value,
    pub timestamp:   chrono::DateTime<chrono::Local>,
}

/// Buffered logger for watch updates
pub struct BufferedWatchLogger {
    tx:          mpsc::Sender<LogEntry>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl BufferedWatchLogger {
    /// Create a new buffered logger and spawn the writer task
    pub fn new(log_path: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel(1000); // Buffer up to 1000 messages
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn the writer task
        tokio::spawn(async move {
            if let Err(e) = write_task(log_path, rx, shutdown_rx).await {
                error!("Watch logger write task failed: {}", e);
            }
        });

        Self {
            tx,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    /// Queue a log entry for writing (non-blocking)
    pub async fn write_update(
        &self,
        update_type: &str,
        data: serde_json::Value,
    ) -> Result<(), String> {
        let entry = LogEntry {
            update_type: update_type.to_string(),
            data,
            timestamp: chrono::Local::now(),
        };

        self.tx
            .send(entry)
            .await
            .map_err(|_| "Logger channel closed".to_string())
    }

    /// Queue a debug log entry for writing only if debug mode is enabled
    pub async fn write_debug_update(
        &self,
        update_type: &str,
        data: serde_json::Value,
    ) -> Result<(), String> {
        if matches!(
            crate::brp_tools::brp_set_tracing_level::get_current_level(),
            crate::support::tracing::TracingLevel::Debug
                | crate::support::tracing::TracingLevel::Trace
        ) {
            self.write_update(update_type, data).await
        } else {
            Ok(())
        }
    }
}

impl Drop for BufferedWatchLogger {
    fn drop(&mut self) {
        // Signal shutdown to the write task
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
            debug!("Sent shutdown signal to watch logger");
        }
    }
}

/// Helper function to flush the buffer to the file
async fn flush_buffer(
    file: &mut tokio::fs::File,
    buffer: &mut String,
    last_flush: &mut tokio::time::Instant,
) -> std::io::Result<()> {
    if !buffer.is_empty() {
        file.write_all(buffer.as_bytes()).await?;
        file.flush().await?;
        buffer.clear();
        *last_flush = tokio::time::Instant::now();
        debug!("Flushed watch log buffer");
    }
    Ok(())
}

/// Background task that batches and writes log entries
async fn write_task(
    log_path: PathBuf,
    mut rx: mpsc::Receiver<LogEntry>,
    mut shutdown_rx: oneshot::Receiver<()>,
) -> std::io::Result<()> {
    // Open file once and keep it open
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await?;

    // Buffer for batching writes
    let mut buffer = String::with_capacity(8192); // 8KB buffer
    let mut last_flush = tokio::time::Instant::now();
    let flush_interval = std::time::Duration::from_millis(100); // Flush every 100ms

    loop {
        // Try to receive with timeout, but also check for shutdown signal
        tokio::select! {
            // Check for shutdown signal
            _ = &mut shutdown_rx => {
                debug!("Watch logger received shutdown signal");
                break;
            }

            // Try to receive log entry with timeout
            timeout_result = tokio::time::timeout(flush_interval, rx.recv()) => {
                match timeout_result {
                    Ok(Some(entry)) => {
                        // Format entry into buffer
                        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
                        if let Ok(json) = serde_json::to_string(&entry.data) {
                            let _ = writeln!(
                                &mut buffer,
                                "[{}] {}: {}",
                                timestamp, entry.update_type, json
                            );
                        }

                        // Check if we should flush (buffer size or time)
                        if buffer.len() > 4096 || last_flush.elapsed() > flush_interval {
                            flush_buffer(&mut file, &mut buffer, &mut last_flush).await?;
                        }
                    }
                    Ok(None) => {
                        // Channel closed, flush remaining buffer and exit
                        debug!("Watch logger message channel closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout - flush if buffer has content
                        flush_buffer(&mut file, &mut buffer, &mut last_flush).await?;
                    }
                }
            }
        }
    }

    // Final flush before shutdown
    flush_buffer(&mut file, &mut buffer, &mut last_flush).await?;
    debug!("Watch logger write task shutting down cleanly");

    Ok(())
}

/// Get the log file path for a watch (same as before)
pub fn get_watch_log_path(watch_id: u32, entity_id: u64, watch_type: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let filename =
        format!("bevy_brp_mcp_watch_{watch_id}_{watch_type}_{entity_id}_{timestamp}.log");

    std::env::temp_dir().join(filename)
}
