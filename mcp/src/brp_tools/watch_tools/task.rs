//! Background task management for watch connections

use std::path::PathBuf;
use std::time::Instant;

use error_stack::Report;
use futures::StreamExt;
use reqwest::Response;
use serde_json::Value;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use super::constants::BUFFER_CONTENT_FIELD;
use super::constants::BUFFER_SIZE_FIELD;
use super::constants::CHUNK_SIZE_FIELD;
use super::constants::CHUNKS_RECEIVED_BEFORE_ERROR_FIELD;
use super::constants::COMPONENT_UPDATE_EVENT;
use super::constants::CONNECTION_ERROR_EVENT;
use super::constants::CONTAINS_DATA_PREFIX_FIELD;
use super::constants::CONTAINS_NEWLINE_FIELD;
use super::constants::CONTENT_TYPE_FIELD;
use super::constants::CONTENT_TYPE_HEADER;
use super::constants::DATA_LENGTH_FIELD;
use super::constants::DEBUG_CHUNK_RECEIVED_EVENT;
use super::constants::DEBUG_FIRST_CHUNK_EVENT;
use super::constants::DEBUG_HTTP_RESPONSE_EVENT;
use super::constants::DEBUG_INCOMPLETE_LINE_IN_BUFFER_EVENT;
use super::constants::DEBUG_JSON_PARSE_FAILED_EVENT;
use super::constants::DEBUG_JSON_PARSED_EVENT;
use super::constants::DEBUG_LINE_RECEIVED_EVENT;
use super::constants::DEBUG_LINES_PROCESSED_EVENT;
use super::constants::DEBUG_NO_RESULT_EVENT;
use super::constants::DEBUG_STREAM_ENDED_EVENT;
use super::constants::DEBUG_STREAM_ERROR_EVENT;
use super::constants::DEBUG_STREAM_STARTED_EVENT;
use super::constants::ELAPSED_SECONDS_FIELD;
use super::constants::EMPTY_LINES_FIELD;
use super::constants::ENTITY_FIELD;
use super::constants::ERROR_FIELD;
use super::constants::FINAL_BUFFER_SIZE_FIELD;
use super::constants::FULL_DATA_FIELD;
use super::constants::HAD_INCOMPLETE_LINE_FIELD;
use super::constants::HAS_ERROR_FIELD;
use super::constants::HAS_ID_FIELD;
use super::constants::HAS_RESULT_FIELD;
use super::constants::HEADERS_COUNT_FIELD;
use super::constants::IS_SSE_DATA_FIELD;
use super::constants::JSON_KEYS_FIELD;
use super::constants::JSON_RPC_ERROR_FIELD;
use super::constants::JSON_RPC_ID_FIELD;
use super::constants::JSON_RPC_RESULT_FIELD;
use super::constants::LINE_BUFFER_SIZE_BEFORE_FIELD;
use super::constants::LINE_FIELD;
use super::constants::LINE_LENGTH_FIELD;
use super::constants::LINES_PROCESSED_FIELD;
use super::constants::MAX_BUFFER_SIZE;
use super::constants::MAX_CHUNK_SIZE;
use super::constants::MAX_PREVIEW_BYTES;
use super::constants::PREVIEW_FIELD;
use super::constants::RAW_DATA_FIELD;
use super::constants::REMAINING_BUFFER_SIZE_FIELD;
use super::constants::RESPONSE_STATUS_FIELD;
use super::constants::SSE_DATA_PREFIX;
use super::constants::STARTS_WITH_DATA_FIELD;
use super::constants::STATUS_FIELD;
use super::constants::STATUS_TEXT_FIELD;
use super::constants::TIMESTAMP_FIELD;
use super::constants::TOTAL_BUFFER_SIZE_BEFORE_FIELD;
use super::constants::TOTAL_CHUNKS_RECEIVED_FIELD;
use super::constants::UNKNOWN_STATUS_TEXT;
use super::constants::WATCH_ENDED_EVENT;
use super::constants::WATCH_STARTED_EVENT;
use super::constants::WATCH_TYPE_FIELD;
use super::logger::BufferedWatchLogger;
use super::manager::WATCH_MANAGER;
use super::manager::WatchInfo;
use crate::brp_tools::BrpClient;
use crate::brp_tools::Port;
use crate::error::Error;
use crate::error::Result;
use crate::tool::BrpMethod;
use crate::tool::ParameterName;

/// Parameters for a watch connection
struct WatchConnectionParams {
    watch_id:   u32,
    entity_id:  u64,
    kind:       String,
    brp_method: BrpMethod,
    params:     Value,
    port:       Port,
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
            DEBUG_LINE_RECEIVED_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                LINE_FIELD: line,
                LINE_LENGTH_FIELD: line.len(),
                IS_SSE_DATA_FIELD: line.starts_with(SSE_DATA_PREFIX),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    // Handle SSE format: "data: {json}"
    let Some(json_str) = line.strip_prefix(SSE_DATA_PREFIX) else {
        debug!("[{watch_type}] Received non-SSE line: {line}");
        return Ok(());
    };

    let Ok(data) = serde_json::from_str::<Value>(json_str) else {
        debug!("[{watch_type}] Failed to parse SSE data as JSON: {json_str}");

        // Log parse failure
        let _ = logger
            .write_debug_update(
                DEBUG_JSON_PARSE_FAILED_EVENT,
                serde_json::json!({
                    WATCH_TYPE_FIELD: watch_type,
                    ParameterName::Entity: entity_id,
                    RAW_DATA_FIELD: json_str,
                    DATA_LENGTH_FIELD: json_str.len(),
                    TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
                }),
            )
            .await;
        return Ok(());
    };

    debug!("[{watch_type}] Received watch update for entity {entity_id}: {data:?}");

    // Log successful JSON parsing
    let _ = logger.write_debug_update(
        DEBUG_JSON_PARSED_EVENT,
        serde_json::json!({
            WATCH_TYPE_FIELD: watch_type,
            ParameterName::Entity: entity_id,
            HAS_RESULT_FIELD: data.get(JSON_RPC_RESULT_FIELD).is_some(),
            HAS_ERROR_FIELD: data.get(JSON_RPC_ERROR_FIELD).is_some(),
            HAS_ID_FIELD: data.get(JSON_RPC_ID_FIELD).is_some(),
            JSON_KEYS_FIELD: data.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
        })
    ).await;

    // Extract the result from JSON-RPC response
    if let Some(result) = data.get(JSON_RPC_RESULT_FIELD) {
        log_update(logger, result.clone()).await?;
    } else {
        debug!("[{watch_type}] No result in JSON-RPC response: {data:?}");

        // Log missing result field
        let _ = logger
            .write_debug_update(
                DEBUG_NO_RESULT_EVENT,
                serde_json::json!({
                    WATCH_TYPE_FIELD: watch_type,
                    ParameterName::Entity: entity_id,
                    FULL_DATA_FIELD: data,
                    TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
                }),
            )
            .await;
    }
    Ok(())
}

/// Log a watch update with error handling
async fn log_update(logger: &BufferedWatchLogger, result: Value) -> Result<()> {
    if let Err(e) = logger.write_update(COMPONENT_UPDATE_EVENT, result).await {
        error!("Failed to write watch update to log: {e}");
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
            DEBUG_CHUNK_RECEIVED_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                CHUNK_SIZE_FIELD: bytes.len(),
                LINE_BUFFER_SIZE_BEFORE_FIELD: line_buffer.len(),
                TOTAL_BUFFER_SIZE_BEFORE_FIELD: *total_buffer_size,
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
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
            debug!("[{watch_type}] Invalid UTF-8 in stream chunk: {e}");
            return Ok(());
        },
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
                DEBUG_LINES_PROCESSED_EVENT,
                serde_json::json!({
                    WATCH_TYPE_FIELD: watch_type,
                    ParameterName::Entity: entity_id,
                    LINES_PROCESSED_FIELD: lines_processed,
                    EMPTY_LINES_FIELD: empty_lines,
                    REMAINING_BUFFER_SIZE_FIELD: line_buffer.len(),
                    TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
                }),
            )
            .await;
    }

    // Log incomplete lines in buffer
    if !line_buffer.is_empty() {
        let _ = logger
            .write_debug_update(
                DEBUG_INCOMPLETE_LINE_IN_BUFFER_EVENT,
                serde_json::json!({
                    WATCH_TYPE_FIELD: watch_type,
                    ParameterName::Entity: entity_id,
                    BUFFER_CONTENT_FIELD: line_buffer,
                    BUFFER_SIZE_FIELD: line_buffer.len(),
                    CONTAINS_DATA_PREFIX_FIELD: line_buffer.contains(SSE_DATA_PREFIX),
                    TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
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
    start_time: Instant,
    total_chunks: usize,
) {
    let elapsed = start_time.elapsed();
    let error_string = error.to_string();

    error!("Error reading stream chunk: {error}");

    // Log stream error
    let _ = logger
        .write_debug_update(
            DEBUG_STREAM_ERROR_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                ERROR_FIELD: error_string,
                CHUNKS_RECEIVED_BEFORE_ERROR_FIELD: total_chunks,
                ELAPSED_SECONDS_FIELD: elapsed.as_secs(),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
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
    let preview = if bytes.len() <= MAX_PREVIEW_BYTES {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        format!(
            "{}... (truncated from {} bytes)",
            String::from_utf8_lossy(&bytes[..MAX_PREVIEW_BYTES]),
            bytes.len()
        )
    };

    let _ = logger
        .write_debug_update(
            DEBUG_FIRST_CHUNK_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                CHUNK_SIZE_FIELD: bytes.len(),
                PREVIEW_FIELD: preview,
                STARTS_WITH_DATA_FIELD: String::from_utf8_lossy(bytes).starts_with(SSE_DATA_PREFIX.trim_end()),
                CONTAINS_NEWLINE_FIELD: bytes.contains(&b'\n'),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
            }),
        )
        .await;
}

/// Process the watch stream from the BRP server
async fn process_watch_stream(
    response: Response,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
    start_time: Instant,
) -> Result<()> {
    if !response.status().is_success() {
        let error_message = format!(
            "server returned {}: {}",
            response.status(),
            response
                .status()
                .canonical_reason()
                .unwrap_or(UNKNOWN_STATUS_TEXT)
        );
        error!("Failed to process watch stream: {error_message}");
        return Err(error_stack::Report::new(Error::BrpCommunication(format!(
            "Failed to process watch stream: {error_message}"
        ))));
    }

    // Log stream start
    let _ = logger
        .write_debug_update(
            DEBUG_STREAM_STARTED_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                RESPONSE_STATUS_FIELD: response.status().as_u16(),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    let total_chunks =
        consume_stream_chunks(response, entity_id, watch_type, logger, start_time).await?;

    info!("[{watch_type}] Watch stream ended for entity {entity_id} ({total_chunks} chunks)");
    Ok(())
}

/// Read all chunks from the streaming response and process them
async fn consume_stream_chunks(
    response: Response,
    entity_id: u64,
    watch_type: &str,
    logger: &BufferedWatchLogger,
    start_time: Instant,
) -> Result<usize> {
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
            },
            Err(e) => {
                handle_stream_error(e, entity_id, watch_type, logger, start_time, total_chunks)
                    .await;
                break;
            },
        }
    }

    // Process any remaining incomplete line in the buffer
    if !line_buffer.trim().is_empty() {
        debug!(
            "[{watch_type}] Processing remaining incomplete line: {}",
            line_buffer.trim()
        );
        parse_sse_line(line_buffer.trim(), entity_id, watch_type, logger).await?;
    }

    // Log stream end with details
    let _ = logger
        .write_debug_update(
            DEBUG_STREAM_ENDED_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: watch_type,
                ParameterName::Entity: entity_id,
                TOTAL_CHUNKS_RECEIVED_FIELD: total_chunks,
                FINAL_BUFFER_SIZE_FIELD: line_buffer.len(),
                HAD_INCOMPLETE_LINE_FIELD: !line_buffer.trim().is_empty(),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
            }),
        )
        .await;

    Ok(total_chunks)
}

/// Handle connection errors and log appropriately
async fn handle_connection_error(
    error: Report<Error>,
    conn_params: &WatchConnectionParams,
    logger: &BufferedWatchLogger,
    start_time: Instant,
) {
    let elapsed = start_time.elapsed();
    let error_string = error.to_string();

    error!("Failed to connect to BRP server: {error}");

    let _ = logger
        .write_update(
            CONNECTION_ERROR_EVENT,
            serde_json::json!({
                WATCH_TYPE_FIELD: &conn_params.kind,
                ParameterName::Entity: conn_params.entity_id,
                ERROR_FIELD: error_string,
                ELAPSED_SECONDS_FIELD: elapsed.as_secs(),
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
            }),
        )
        .await;
}

/// Run the watch connection in a spawned task
async fn run_watch_connection(conn_params: WatchConnectionParams, logger: BufferedWatchLogger) {
    info!(
        "Starting {} watch task for entity {} on port {}",
        conn_params.kind, conn_params.entity_id, conn_params.port
    );

    // Track start time for timeout detection
    let start_time = std::time::Instant::now();

    // Create BRP client
    let brp_client = BrpClient::new(
        conn_params.brp_method,
        conn_params.port,
        Some(conn_params.params.clone()),
    );

    match brp_client.execute_streaming().await {
        Ok(response) => {
            // Log initial HTTP response
            let _ = logger
                .write_debug_update(
                    DEBUG_HTTP_RESPONSE_EVENT,
                    serde_json::json!({
                        WATCH_TYPE_FIELD: &conn_params.kind,
                        ParameterName::Entity: conn_params.entity_id,
                        STATUS_FIELD: response.status().as_u16(),
                        STATUS_TEXT_FIELD: response.status().canonical_reason().unwrap_or(UNKNOWN_STATUS_TEXT),
                        HEADERS_COUNT_FIELD: response.headers().len(),
                        CONTENT_TYPE_FIELD: response
                            .headers()
                            .get(CONTENT_TYPE_HEADER)
                            .and_then(|value| value.to_str().ok()),
                        TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
                    }),
                )
                .await;

            if let Err(e) = process_watch_stream(
                response,
                conn_params.entity_id,
                &conn_params.kind,
                &logger,
                start_time,
            )
            .await
            {
                error!("Watch stream processing failed: {e}");
            }
        },
        Err(e) => {
            handle_connection_error(e, &conn_params, &logger, start_time).await;
        },
    }

    // Write final log entry
    let _ = logger
        .write_update(
            WATCH_ENDED_EVENT,
            serde_json::json!({
                ParameterName::Entity: conn_params.entity_id,
                TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
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
    port: Port,
) -> Result<(u32, PathBuf)> {
    // Prepare all data that doesn't require the watch_id
    let watch_type_owned = watch_type.to_string();
    let brp_method_owned = brp_method;

    // Perform all operations within a single lock to ensure atomicity
    let mut manager = WATCH_MANAGER.lock().await;

    // Generate ID while holding the lock
    let watch_id = manager.next_id();

    // Create log path and logger
    let log_path = BufferedWatchLogger::get_watch_log_path(watch_id, entity_id, watch_type);
    let buffered_watch_logger = BufferedWatchLogger::new(log_path.clone());

    // Create initial log entry
    let log_data = match params.clone() {
        Value::Object(mut map) => {
            map.insert(String::from(ParameterName::Port), serde_json::json!(port));
            map.insert(
                TIMESTAMP_FIELD.to_string(),
                serde_json::json!(chrono::Local::now().to_rfc3339()),
            );
            Value::Object(map)
        },
        _ => serde_json::json!({
            ParameterName::Entity: entity_id,
            ParameterName::Port: port,
            TIMESTAMP_FIELD: chrono::Local::now().to_rfc3339()
        }),
    };

    // If logging fails, we haven't registered anything yet
    let log_result = buffered_watch_logger
        .write_update(WATCH_STARTED_EVENT, log_data)
        .await;

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
            kind: watch_type_owned,
            brp_method: brp_method_owned,
            params,
            port,
        },
        buffered_watch_logger,
    ));

    // Register immediately while still holding the lock
    manager.active_watches.insert(
        watch_id,
        (
            WatchInfo {
                watch_id,
                entity_id,
                kind: watch_type.to_string(),
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
pub(super) async fn start_entity_watch_task(
    entity_id: u64,
    components: Option<Vec<String>>,
    port: Port,
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
        ParameterName::Entity: entity_id,
        ParameterName::Components: components
    });

    start_watch_task(
        entity_id,
        "get",
        BrpMethod::WorldGetComponentsWatch,
        params,
        port,
    )
    .await
}

/// Start a background task for entity list watching
pub(super) async fn start_list_watch_task(entity_id: u64, port: Port) -> Result<(u32, PathBuf)> {
    let params = serde_json::json!({
        ENTITY_FIELD: entity_id
    });

    start_watch_task(
        entity_id,
        "list",
        BrpMethod::WorldListComponentsWatch,
        params,
        port,
    )
    .await
}
