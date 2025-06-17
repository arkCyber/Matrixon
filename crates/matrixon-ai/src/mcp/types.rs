//! MCP Types
//!
//! This module defines types used in MCP protocol implementation.
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema for the tool
    pub input_schema: serde_json::Value,
    /// Output schema for the tool
    pub output_schema: serde_json::Value,
}

/// MCP Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// Request ID
    pub id: String,
    /// Request method
    pub method: String,
    /// Request parameters
    pub params: serde_json::Value,
}

/// MCP Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// Response ID
    pub id: String,
    /// Response result
    pub result: Option<serde_json::Value>,
    /// Response error
    pub error: Option<McpError>,
}

/// MCP Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Error data
    pub data: Option<serde_json::Value>,
}

/// MCP Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// Server protocol (http/ws)
    pub protocol: String,
    /// Server authentication token
    pub auth_token: Option<String>,
    /// Available tools
    pub tools: Vec<McpTool>,
}

/// MCP Client Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    /// Server URL
    pub server_url: String,
    /// Client authentication token
    pub auth_token: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retries
    pub max_retries: u32,
}

/// MCP Connection State
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Error state
    Error(String),
}

/// MCP Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMessage {
    /// Message type
    pub message_type: String,
    /// Message content
    pub content: serde_json::Value,
    /// Message metadata
    pub metadata: HashMap<String, String>,
}
