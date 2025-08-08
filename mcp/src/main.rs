//! # Bevy BRP MCP Server
//!
//! A Model Context Protocol server that provides tools for interacting with
//! Bevy applications through the Bevy Remote Protocol (BRP).
//!
//! This server enables remote debugging, inspection, and manipulation of
//! Bevy applications at runtime through a standardized MCP interface.

use std::error::Error;

use brp_tools::WatchManager;
use log_tools::TracingLevel;
use mcp_service::McpService;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

mod app_tools;
mod brp_tools;
mod error;
mod json_traits;
mod log_tools;
mod mcp_service;
mod tool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize file-based tracing with dynamic level management
    // Uses lazy file creation - file only created on first log write
    TracingLevel::init_file_tracing();

    // Initialize the watch manager
    WatchManager::initialize_watch_manager().await;

    let service = McpService::new();

    let server = service.serve(stdio()).await?;
    server.waiting().await?;

    Ok(())
}
