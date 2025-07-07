//! # Bevy BRP MCP Server
//!
//! A Model Context Protocol server that provides tools for interacting with
//! Bevy applications through the Bevy Remote Protocol (BRP).
//!
//! This server enables remote debugging, inspection, and manipulation of
//! Bevy applications at runtime through a standardized MCP interface.

use std::error::Error;

use rmcp::ServiceExt;
use rmcp::transport::stdio;

mod app_tools;
mod brp_tools;
mod constants;
mod error;
mod log_tools;
mod registry;
mod service;
mod support;
mod tool_definitions;
mod tool_generator;
mod tools;

use service::BrpMcpService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize file-based tracing with dynamic level management
    // Uses lazy file creation - file only created on first log write
    support::tracing::init_file_tracing();

    // Initialize the watch manager
    brp_tools::initialize_watch_manager().await;

    let service = BrpMcpService::new();

    let server = service.serve(stdio()).await?;
    server.waiting().await?;

    Ok(())
}
