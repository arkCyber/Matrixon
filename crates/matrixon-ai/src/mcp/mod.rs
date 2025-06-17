//! Model Context Protocol (MCP) Integration
//!
//! This module provides integration with MCP servers for enhanced AI capabilities.
//! It includes both client and server implementations for MCP protocol.
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use std::time::Instant;
use tracing::{info, instrument};

pub mod client;
pub mod error;
pub mod server;
pub mod types;

pub use client::McpClient;
pub use error::McpError;
pub use server::McpServer;
pub use types::*;

/// MCP protocol version
pub const MCP_VERSION: &str = "1.0.0";

/// Initialize MCP integration
#[instrument(level = "debug")]
pub async fn init() -> Result<(), McpError> {
    let start = Instant::now();
    info!("🔧 Initializing MCP integration");

    // Initialize MCP client
    let _client = McpClient::new()?;
    info!("✅ MCP client initialized");

    // Initialize MCP server
    let _server = McpServer::new()?;
    info!("✅ MCP server initialized");

    info!("🎉 MCP integration completed in {:?}", start.elapsed());
    Ok(())
}
