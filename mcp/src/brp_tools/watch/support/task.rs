//! Background task management for watch connections

use std::path::PathBuf;

use futures::StreamExt;
use serde_json::Value;
use tracing::{debug, error, info, warn};

/// Maximum size for a single chunk in the SSE stream (1MB)
const MAX_CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum size for the total buffer when processing incomplete lines (10MB)
const MAX_BUFFER_SIZE: usize = 10 * 1024 * 1024;

use super::logger::{self as watch_logger, BufferedWatchLogger};
use super::manager::{WATCH_MANAGER, WatchInfo};
use crate::brp_tools::support::BrpJsonRpcBuilder;
use crate::error::{Error, Result};
use crate::tools::{BRP_METHOD_GET_WATCH, BRP_METHOD_LIST_WATCH};

/// Process a single SSE line and log the update if valid
async fn parse_sse_line(line: &str, entity_id: u64, logger: &BufferedWatchLogger) -> Result<()> {
    // Handle SSE format: "data: {json}"
    if let Some(json_str) = line.strip_prefix("data: ") {
        if let Ok(data) = serde_json::from_str::<Value>(json_str) {
            debug!("Received watch update for entity {}: {:?}", entity_id, data);

            // Extract the result from JSON-RPC response
            if let Some(result) = data.get("result") {
                log_update(logger, result.clone()).await?;
            } else {
                debug!("No result in JSON-RPC response: {:?}", data);
            }
        } else {
            debug!("Failed to parse SSE data as JSON: {}", json_str);
        }
    } else {
        debug!("Received non-SSE line: {}", line);
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
    logger: &BufferedWatchLogger,
) -> Result<()> {
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
            debug!("Invalid UTF-8 in stream chunk: {}", e);
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
    while let Some(newline_pos) = line_buffer.find('\n') {
        let line = line_buffer.drain(..=newline_pos).collect::<String>();
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        // Update buffer size tracking
        *total_buffer_size = line_buffer.len();

        if line.trim().is_empty() {
            continue;
        }

        parse_sse_line(line, entity_id, logger).await?;
    }

    Ok(())
}

/// Process the watch stream from the BRP server
async fn process_watch_stream(
    response: reqwest::Response,
    entity_id: u64,
    logger: &BufferedWatchLogger,
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

    // Read the streaming response with bounded memory usage
    let mut stream = response.bytes_stream();
    let mut line_buffer = String::new();
    let mut total_buffer_size = 0;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                process_chunk(
                    &bytes,
                    &mut line_buffer,
                    &mut total_buffer_size,
                    entity_id,
                    logger,
                )
                .await?;
            }
            Err(e) => {
                error!("Error reading stream chunk: {}", e);
                break;
            }
        }
    }

    // Process any remaining incomplete line in the buffer
    if !line_buffer.trim().is_empty() {
        debug!(
            "Processing remaining incomplete line: {}",
            line_buffer.trim()
        );
        parse_sse_line(line_buffer.trim(), entity_id, logger).await?;
    }

    info!("Watch stream ended for entity {}", entity_id);
    Ok(())
}

/// Run the watch connection in a spawned task
async fn run_watch_connection(
    watch_id: u32,
    entity_id: u64,
    watch_type: String,
    brp_method: String,
    params: Value,
    port: u16,
    logger: BufferedWatchLogger,
) {
    info!(
        "Starting {} watch task for entity {} on port {}",
        watch_type, entity_id, port
    );

    // Create HTTP client for streaming
    let url = crate::brp_tools::support::brp_client::build_brp_url(port);
    let client = crate::brp_tools::support::http_client::get_client();

    // Build JSON-RPC request for watching
    let request_body = BrpJsonRpcBuilder::new(&brp_method)
        .params(params)
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
            if let Err(e) = process_watch_stream(response, entity_id, &logger).await {
                error!("Watch stream processing failed: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to connect to BRP server: {}", e);
            let _ = logger
                .write_update(
                    "CONNECTION_ERROR",
                    serde_json::json!({
                        "error": e.to_string(),
                        "timestamp": chrono::Local::now().to_rfc3339()
                    }),
                )
                .await;
        }
    }

    // Write final log entry
    let _ = logger
        .write_update(
            "WATCH_ENDED",
            serde_json::json!({
                "entity": entity_id,
                "timestamp": chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Remove this watch from the active watches with defensive checks
    {
        let mut manager = WATCH_MANAGER.lock().await;
        if manager.active_watches.remove(&watch_id).is_some() {
            info!(
                "Watch {} for entity {} automatically cleaned up after connection ended",
                watch_id, entity_id
            );
        } else {
            warn!(
                "Watch {} for entity {} attempted to clean up but was not found in active watches - possible phantom watch removal",
                watch_id, entity_id
            );
        }
    }
}

/// Generic function to start a watch task
async fn start_watch_task(
    entity_id: u64,
    watch_type: &str,
    brp_method: &str,
    params: Value,
    port: u16,
) -> Result<(u32, PathBuf)> {
    // Prepare all data that doesn't require the watch_id
    let watch_type_owned = watch_type.to_string();
    let brp_method_owned = brp_method.to_string();

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
            map.insert("port".to_string(), serde_json::json!(port));
            map.insert(
                "timestamp".to_string(),
                serde_json::json!(chrono::Local::now().to_rfc3339()),
            );
            Value::Object(map)
        }
        _ => serde_json::json!({
            "entity": entity_id,
            "port": port,
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
        watch_id,
        entity_id,
        watch_type_owned,
        brp_method_owned,
        params,
        port,
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
        "entity": entity_id,
        "components": components
    });

    start_watch_task(entity_id, "get", BRP_METHOD_GET_WATCH, params, port).await
}

/// Start a background task for entity list watching
pub async fn start_list_watch_task(entity_id: u64, port: u16) -> Result<(u32, PathBuf)> {
    let params = serde_json::json!({
        "entity": entity_id
    });

    start_watch_task(entity_id, "list", BRP_METHOD_LIST_WATCH, params, port).await
}
