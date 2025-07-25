//! Background task management for watch connections

use std::path::PathBuf;

use futures::StreamExt;
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// Maximum size for a single chunk in the SSE stream (1MB)
const MAX_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum size for the total buffer when processing incomplete lines (10MB)
const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

use super::super::{BrpJsonRpcBuilder, http_client};
use super::logger::{self as watch_logger, BufferedWatchLogger};
use super::manager::{WATCH_MANAGER, WatchInfo};
use crate::brp_tools;
use crate::error::{Error, Result};
use crate::tool::{BrpMethod, ParameterName};

/// Parameters for a watch connection
struct WatchConnectionParams {
    watch_id:   u32,
    entity_id:  u64,
    watch_type: String,
    brp_method: String,
    params:     Value,
    port:       u16,
}

/// Process a single SSE line and log the update if valid
async fn parse_sse_line(
    line: &str,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
) -> Result<()> {
    // Log EVERY line received for debugging
    let _ = logger
        .write_debug_update(
            "DEBUG_LINE_RECEIVED",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "line": line,
                "line_length": line.len(),
                "is_sse_data": line.starts_with("data: "),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Handle SSE format: "data: {json}"
    if let Some(json_str) = line.strip_prefix("data: ") {
        if let Ok(data) = serde_json::from_str::<Value>(json_str) {
            debug!(
                "[{}] Received watch update for entity {}: {:?}",
                watch_type, entity_id, data
            );

            // Log successful JSON parsing
            let _ = logger.write_debug_update(
                "DEBUG_JSON_PARSED",
                serde_json::json!({
                    "watch_type": watch_type,
                    ParameterName::Entity.as_ref(): entity_id,
                    "has_result": data.get("result").is_some(),
                    "has_error": data.get("error").is_some(),
                    "has_id": data.get("id").is_some(),
                    "json_keys": data.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
                    "timestamp": chrono::Local::now().to_rfc3339()
                })
            ).await;

            // Extract the result from JSON-RPC response
            if let Some(result) = data.get("result") {
                log_update(logger, result.clone()).await?;
            } else {
                debug!(
                    "[{}] No result in JSON-RPC response: {:?}",
                    watch_type, data
                );

                // Log missing result field
                let _ = logger
                    .write_debug_update(
                        "DEBUG_NO_RESULT",
                        serde_json::json!({
                            "watch_type": watch_type,
                            ParameterName::Entity.as_ref(): entity_id,
                            "full_data": data,
                            "timestamp": chrono::Local::now().to_rfc3339()
                        }),
                    )
                    .await;
            }
        } else {
            debug!(
                "[{}] Failed to parse SSE data as JSON: {}",
                watch_type, json_str
            );

            // Log parse failure
            let _ = logger
                .write_debug_update(
                    "DEBUG_JSON_PARSE_FAILED",
                    serde_json::json!({
                        "watch_type": watch_type,
                        ParameterName::Entity.as_ref(): entity_id,
                        "raw_data": json_str,
                        "data_length": json_str.len(),
                        "timestamp": chrono::Local::now().to_rfc3339()
                    }),
                )
                .await;
        }
    } else {
        debug!("[{}] Received non-SSE line: {}", watch_type, line);
    }
    Ok(())
}

/// Log a watch update with error handling
async fn log_update(logger: &BufferedWatchLogger, result: Value) -> Result<()> {
    if let Err(e) = logger.write_update("COMPONENT_UPDATE", result).await {
        error!("Failed to write watch update to log: {}", e);
        return Err(error_stack::Report::new(Error::failed_to(
            "write watch update to log",
            &e,
        )));
    }
    Ok(())
}

/// Process a single chunk from the stream
async fn process_chunk(
    bytes: &[u8],
    line_buffer: &mut String,
    total_buffer_size: &mut usize,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
) -> Result<()> {
    // Log chunk size
    let _ = logger
        .write_debug_update(
            "DEBUG_CHUNK_RECEIVED",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "chunk_size": bytes.len(),
                "line_buffer_size_before": line_buffer.len(),
                "total_buffer_size_before": *total_buffer_size,
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Check chunk size limit
    if bytes.len() > MAX_CHUNK_SIZE {
        return Err(error_stack::Report::new(Error::InvalidState(format!(
            "Stream chunk size {} exceeds maximum {}",
            bytes.len(),
            MAX_CHUNK_SIZE
        ))));
    }

    // Convert bytes to string
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => {
            debug!("[{}] Invalid UTF-8 in stream chunk: {}", watch_type, e);
            return Ok(());
        }
    };

    // Add to line buffer and check total buffer size
    line_buffer.push_str(text);
    *total_buffer_size += text.len();

    if *total_buffer_size > MAX_BUFFER_SIZE {
        return Err(error_stack::Report::new(Error::InvalidState(format!(
            "Stream buffer size {} exceeds maximum {}",
            *total_buffer_size, MAX_BUFFER_SIZE
        ))));
    }

    // Process complete lines from the buffer
    let mut lines_processed = 0;
    let mut empty_lines = 0;

    while let Some(newline_pos) = line_buffer.find('\n') {
        let line = line_buffer.drain(..=newline_pos).collect::<String>();
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // Update buffer size tracking
        *total_buffer_size = line_buffer.len();

        if line.trim().is_empty() {
            empty_lines += 1;
            continue;
        }

        lines_processed += 1;
        parse_sse_line(line, entity_id, watch_type, logger).await?;
    }

    // Log number of lines processed
    if lines_processed > 0 || empty_lines > 0 {
        let _ = logger
            .write_debug_update(
                "DEBUG_LINES_PROCESSED",
                serde_json::json!({
                    "watch_type": watch_type,
                    ParameterName::Entity.as_ref(): entity_id,
                    "lines_processed": lines_processed,
                    "empty_lines": empty_lines,
                    "remaining_buffer_size": line_buffer.len(),
                    "timestamp": chrono::Local::now().to_rfc3339()
                }),
            )
            .await;
    }

    // Log incomplete lines in buffer
    if !line_buffer.is_empty() {
        let _ = logger
            .write_debug_update(
                "DEBUG_INCOMPLETE_LINE_IN_BUFFER",
                serde_json::json!({
                    "watch_type": watch_type,
                    ParameterName::Entity.as_ref(): entity_id,
                    "buffer_content": line_buffer,
                    "buffer_size": line_buffer.len(),
                    "contains_data_prefix": line_buffer.contains("data: "),
                    "timestamp": chrono::Local::now().to_rfc3339()
                }),
            )
            .await;
    }

    Ok(())
}

/// Handle stream error
async fn handle_stream_error(
    error: reqwest::Error,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
    start_time: std::time::Instant,
    total_chunks: usize,
) {
    let elapsed = start_time.elapsed();
    let error_string = error.to_string();

    error!("Error reading stream chunk: {}", error);

    // Log stream error
    let _ = logger
        .write_debug_update(
            "DEBUG_STREAM_ERROR",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "error": error_string,
                "chunks_received_before_error": total_chunks,
                "elapsed_seconds": elapsed.as_secs(),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;
}

/// Log the first chunk of data for debugging
async fn log_first_chunk(
    bytes: &[u8],
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
) {
    let preview = if bytes.len() <= 500 {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        format!(
            "{}... (truncated from {} bytes)",
            String::from_utf8_lossy(&bytes[..500]),
            bytes.len()
        )
    };

    let _ = logger
        .write_debug_update(
            "DEBUG_FIRST_CHUNK",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "chunk_size": bytes.len(),
                "preview": preview,
                "starts_with_data": String::from_utf8_lossy(bytes).starts_with("data:"),
                "contains_newline": bytes.contains(&b'\n'),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;
}

/// Process the watch stream from the BRP server
#[allow(clippy::too_many_lines)]
async fn process_watch_stream(
    response: reqwest::Response,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
    start_time: std::time::Instant,
) -> Result<()> {
    if !response.status().is_success() {
        let error_msg = format!(
            "server returned {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        );
        error!("Failed to process watch stream: {}", error_msg);
        return Err(error_stack::Report::new(Error::BrpCommunication(format!(
            "Failed to process watch stream: {error_msg}"
        ))));
    }

    // Log stream start
    let _ = logger
        .write_debug_update(
            "DEBUG_STREAM_STARTED",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "response_status": response.status().as_u16(),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Read the streaming response with bounded memory usage
    let mut stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut total_buffer_size = 0;
    let mut total_chunks = 0;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                total_chunks += 1;

                // Special logging for first chunk
                if total_chunks == 1 {
                    log_first_chunk(&bytes, entity_id, watch_type, logger).await;
                }

                process_chunk(
                    &bytes,
                    &mut line_buffer,
                    &mut total_buffer_size,
                    entity_id,
                    watch_type,
                    logger,
                )
                .await?;
            }
            Err(e) => {
                handle_stream_error(e, entity_id, watch_type, logger, start_time, total_chunks)
                    .await;
                break;
            }
        }
    }

    // Process any remaining incomplete line in the buffer
    if !line_buffer.trim().is_empty() {
        debug!(
            "[{}] Processing remaining incomplete line: {}",
            watch_type,
            line_buffer.trim()
        );
        parse_sse_line(line_buffer.trim(), entity_id, watch_type, logger).await?;
    }

    // Scenario 3 removed - redundant with Scenario 2 stream error timeout detection

    // Log stream end with details
    let _ = logger
        .write_debug_update(
            "DEBUG_STREAM_ENDED",
            serde_json::json!({
                "watch_type": watch_type,
                ParameterName::Entity.as_ref(): entity_id,
                "total_chunks_received": total_chunks,
                "final_buffer_size": line_buffer.len(),
                "had_incomplete_line": !line_buffer.trim().is_empty(),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    info!(
        "[{}] Watch stream ended for entity {}",
        watch_type, entity_id
    );
    Ok(())
}

/// Handle connection errors and log appropriately
async fn handle_connection_error(
    error: reqwest::Error,
    conn_params: &WatchConnectionParams,
    logger: &BufferedWatchLogger,
    start_time: std::time::Instant,
) {
    let elapsed = start_time.elapsed();
    let error_string = error.to_string();

    error!("Failed to connect to BRP server: {}", error);

    let _ = logger
        .write_update(
            "CONNECTION_ERROR",
            serde_json::json!({
                "watch_type": &conn_params.watch_type,
                ParameterName::Entity.as_ref(): conn_params.entity_id,
                "error": error_string,
                "elapsed_seconds": elapsed.as_secs(),
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;
}

/// Run the watch connection in a spawned task
async fn run_watch_connection(conn_params: WatchConnectionParams, logger: BufferedWatchLogger) {
    info!(
        "Starting {} watch task for entity {} on port {}",
        conn_params.watch_type, conn_params.entity_id, conn_params.port
    );

    // Track start time for timeout detection
    let start_time = std::time::Instant::now();

    // Create HTTP client for streaming with no timeout
    let url = brp_tools::build_brp_url(conn_params.port);
    let client = http_client::create_watch_client(Some(0)); // 0 = no timeout for watches

    // Build JSON-RPC request for watching
    let request_body = BrpJsonRpcBuilder::new(&conn_params.brp_method)
        .params(conn_params.params.clone())
        .build()
        .to_string();

    match client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(request_body)
        // Don't set timeout for streaming connections
        .send()
        .await
    {
        Ok(response) => {
            // Log initial HTTP response
            let _ = logger.write_debug_update(
                "DEBUG_HTTP_RESPONSE",
                serde_json::json!({
                    "watch_type": &conn_params.watch_type,
                    ParameterName::Entity.as_ref(): conn_params.entity_id,
                    "status": response.status().as_u16(),
                    "status_text": response.status().canonical_reason().unwrap_or("Unknown"),
                    "headers_count": response.headers().len(),
                    "content_type": response.headers().get("content-type").and_then(|v| v.to_str().ok()),
                    "timestamp": chrono::Local::now().to_rfc3339()
                })
            ).await;

            if let Err(e) = process_watch_stream(
                response,
                conn_params.entity_id,
                &conn_params.watch_type,
                &logger,
                start_time,
            )
            .await
            {
                error!("Watch stream processing failed: {}", e);
            }
        }
        Err(e) => {
            handle_connection_error(e, &conn_params, &logger, start_time).await;
        }
    }

    // Write final log entry
    let _ = logger
        .write_update(
            "WATCH_ENDED",
            serde_json::json!({
                ParameterName::Entity.as_ref(): conn_params.entity_id,
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Remove this watch from the active watches with defensive checks
    {
        let mut manager = WATCH_MANAGER.lock().await;
        if manager
            .active_watches
            .remove(&conn_params.watch_id)
            .is_some()
        {
            info!(
                "Watch {} for entity {} automatically cleaned up after connection ended",
                conn_params.watch_id, conn_params.entity_id
            );
        } else {
            warn!(
                "Watch {} for entity {} attempted to clean up but was not found in active watches - possible phantom watch removal",
                conn_params.watch_id, conn_params.entity_id
            );
        }
    }
}

/// Generic function to start a watch task
async fn start_watch_task(
    entity_id: u64,
    watch_type: &str,
    brp_method: BrpMethod,
    params: Value,
    port: u16,
) -> Result<(u32, PathBuf)> {
    // Prepare all data that doesn't require the watch_id
    let watch_type_owned = watch_type.to_string();
    let brp_method_owned = brp_method.as_str().to_string();

    // Perform all operations within a single lock to ensure atomicity
    let mut manager = WATCH_MANAGER.lock().await;

    // Generate ID while holding the lock
    let watch_id = manager.next_id();

    // Create log path and logger
    let log_path = watch_logger::get_watch_log_path(watch_id, entity_id, watch_type);
    let logger = BufferedWatchLogger::new(log_path.clone());

    // Create initial log entry
    let log_data = match params.clone() {
        Value::Object(mut map) => {
            map.insert(
                ParameterName::Port.as_ref().to_string(),
                serde_json::json!(port),
            );
            map.insert(
                "timestamp".to_string(),
                serde_json::json!(chrono::Local::now().to_rfc3339()),
            );
            Value::Object(map)
        }
        _ => serde_json::json!({
            ParameterName::Entity.as_ref(): entity_id,
            ParameterName::Port.as_ref(): port,
            "timestamp": chrono::Local::now().to_rfc3339()
        }),
    };

    // If logging fails, we haven't registered anything yet
    let log_result = logger.write_update("WATCH_STARTED", log_data).await;

    if let Err(e) = log_result {
        return Err(error_stack::Report::new(Error::WatchOperation(format!(
            "Failed to log initial entry for entity {entity_id}: {e}"
        ))));
    }

    // Spawn task
    let handle = tokio::spawn(run_watch_connection(
        WatchConnectionParams {
            watch_id,
            entity_id,
            watch_type: watch_type_owned,
            brp_method: brp_method_owned,
            params,
            port,
        },
        logger,
    ));

    // Register immediately while still holding the lock
    manager.active_watches.insert(
        watch_id,
        (
            WatchInfo {
                watch_id,
                entity_id,
                watch_type: watch_type.to_string(),
                log_path: log_path.clone(),
                port,
            },
            handle,
        ),
    );

    // Release lock by dropping manager
    drop(manager);

    Ok((watch_id, log_path))
}

/// Start a background task for entity component watching
pub async fn start_entity_watch_task(
    entity_id: u64,
    components: Option<Vec<String>>,
    port: u16,
) -> Result<(u32, PathBuf)> {
    // Validate components parameter
    let components = components.ok_or_else(|| {
        error_stack::Report::new(Error::missing("components parameter is required for entity watch. Specify which components to monitor"))
    })?;

    if components.is_empty() {
        return Err(error_stack::Report::new(Error::invalid(
            "components array",
            "cannot be empty. Specify at least one component to watch",
        )));
    }

    // Build the watch parameters
    let params = serde_json::json!({
        ParameterName::Entity.as_ref(): entity_id,
        ParameterName::Components.as_ref(): components
    });

    start_watch_task(entity_id, "get", BrpMethod::BevyGetWatch, params, port).await
}

/// Start a background task for entity list watching
pub async fn start_list_watch_task(entity_id: u64, port: u16) -> Result<(u32, PathBuf)> {
    let params = serde_json::json!({
        "entity": entity_id
    });

    start_watch_task(entity_id, "list", BrpMethod::BevyListWatch, params, port).await
}
