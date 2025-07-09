use std::path::PathBuf;

use serde_json::{Value, json};

use crate::brp_tools::constants::{JSON_FIELD_LOG_PATH, JSON_FIELD_WATCH_ID};
use crate::error::Error;

pub fn format_watch_start_response_value(
    result: std::result::Result<(u32, PathBuf), Error>,
    operation_name: &str,
    entity_id: u64,
) -> Value {
    match result {
        Ok((watch_id, log_path)) => {
            let message = format!("Started {operation_name} {watch_id} for entity {entity_id}");
            json!({
                "status": "success",
                "message": message,
                JSON_FIELD_WATCH_ID: watch_id,
                JSON_FIELD_LOG_PATH: log_path.to_string_lossy()
            })
        }
        Err(e) => {
            json!({
                "status": "error",
                "message": e.to_string()
            })
        }
    }
}

pub fn format_watch_stop_response_value(
    result: std::result::Result<(), Error>,
    watch_id: u32,
) -> Value {
    match result {
        Ok(()) => {
            json!({
                "status": "success",
                "message": format!("Stopped watch {watch_id}")
            })
        }
        Err(e) => {
            json!({
                "status": "error",
                "message": e.to_string()
            })
        }
    }
}
