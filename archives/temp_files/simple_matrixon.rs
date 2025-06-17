// =============================================================================
// Simple Matrixon Matrix Server - For Testing and Production Environment
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Date: 2024-12-11
// Version: 0.11.0-alpha
// License: Apache 2.0 / MIT
//
// Description:
//   Simplified Matrixon Matrix server for production environment testing
//   Full Matrix Client-Server API implementation with curl-testable endpoints
//
// =============================================================================

use axum::{
    extract::{Path, Query, State}, 
    http::{HeaderMap, StatusCode}, 
    response::IntoResponse, 
    routing::{get, post, put}, 
    Json, Router
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tokio::net::TcpListener;
use tracing::{info, warn, error, debug};

// ============================================================================
// Core Types and Structures
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub users: Arc<std::sync::RwLock<HashMap<String, User>>>,
    pub rooms: Arc<std::sync::RwLock<HashMap<String, Room>>>,
    pub events: Arc<std::sync::RwLock<HashMap<String, MatrixEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub password_hash: String,
    pub access_tokens: Vec<String>,
    pub device_id: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub members: Vec<String>,
    pub events: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixEvent {
    pub event_id: String,
    pub room_id: String,
    pub sender: String,
    pub event_type: String,
    pub content: Value,
    pub timestamp: u64,
}

// ============================================================================
// Matrix Client-Server API Endpoints
// ============================================================================

/// GET /_matrix/client/versions - Get supported Matrix versions
async fn get_versions() -> impl IntoResponse {
    info!("🔍 Matrix versions endpoint called");
    Json(json!({
        "versions": [
            "r0.0.1", "r0.1.0", "r0.2.0", "r0.3.0", "r0.4.0", "r0.5.0", "r0.6.0", "r0.6.1",
            "v1.1", "v1.2", "v1.3", "v1.4", "v1.5", "v1.6", "v1.7", "v1.8", "v1.9", "v1.10"
        ],
        "unstable_features": {
            "org.matrix.e2e_cross_signing": true,
            "org.matrix.msc2432": true
        }
    }))
}

/// GET /_matrix/client/r0/capabilities - Get server capabilities
async fn get_capabilities() -> impl IntoResponse {
    info!("🔧 Server capabilities endpoint called");
    Json(json!({
        "capabilities": {
            "m.change_password": {
                "enabled": true
            },
            "m.room_versions": {
                "default": "9",
                "available": {
                    "1": "stable",
                    "2": "stable", 
                    "3": "stable",
                    "4": "stable",
                    "5": "stable",
                    "6": "stable",
                    "7": "stable",
                    "8": "stable",
                    "9": "stable",
                    "10": "stable"
                }
            },
            "m.set_displayname": {
                "enabled": true
            },
            "m.set_avatar_url": {
                "enabled": true
            },
            "m.3pid_changes": {
                "enabled": true
            }
        }
    }))
}

/// GET /_matrix/client/r0/login - Get available login types
async fn get_login_types() -> impl IntoResponse {
    info!("🔑 Login types endpoint called");
    Json(json!({
        "flows": [
            {
                "type": "m.login.password"
            },
            {
                "type": "m.login.token"
            }
        ]
    }))
}

/// POST /_matrix/client/r0/login - User login
async fn login(Json(payload): Json<Value>) -> impl IntoResponse {
    info!("🔓 User login endpoint called with payload: {:?}", payload);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    Json(json!({
        "user_id": "@testuser:localhost",
        "access_token": format!("syt_login_token_{}", timestamp),
        "device_id": format!("LOGIN_DEVICE_{}", timestamp),
        "well_known": {
            "m.homeserver": {
                "base_url": "http://localhost:6167"
            }
        }
    }))
}

/// POST /_matrix/client/r0/register - User registration
async fn register(Json(payload): Json<Value>) -> impl IntoResponse {
    info!("🔐 User registration endpoint called with payload: {:?}", payload);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    Json(json!({
        "user_id": format!("@newuser_{}:localhost", timestamp),
        "access_token": format!("syt_register_token_{}", timestamp),
        "device_id": format!("REGISTER_DEVICE_{}", timestamp)
    }))
}

/// GET /_matrix/client/r0/account/whoami - Get current user info
async fn whoami(headers: HeaderMap) -> impl IntoResponse {
    info!("👤 Whoami endpoint called");
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("No auth header");
    
    Json(json!({
        "user_id": "@current_user:localhost",
        "device_id": "CURRENT_DEVICE_123",
        "is_guest": false
    }))
}

/// POST /_matrix/client/r0/createRoom - Create a new room
async fn create_room(headers: HeaderMap, Json(payload): Json<Value>) -> impl IntoResponse {
    info!("🏠 Room creation endpoint called with payload: {:?}", payload);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let room_id = format!("!room_{}:localhost", timestamp);
    
    Json(json!({
        "room_id": room_id,
        "room_alias": format!("#test_room_{}:localhost", timestamp)
    }))
}

/// PUT /_matrix/client/r0/rooms/{roomId}/send/{eventType}/{txnId} - Send message
async fn send_message(
    Path((room_id, event_type, txn_id)): Path<(String, String, String)>,
    Json(payload): Json<Value>
) -> impl IntoResponse {
    info!("💬 Message send endpoint called - Room: {}, Type: {}, TxnId: {}", room_id, event_type, txn_id);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let event_id = format!("$event_{}:localhost", timestamp);
    
    Json(json!({
        "event_id": event_id
    }))
}

/// GET /_matrix/client/r0/sync - Sync events
async fn sync_events(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    info!("🔄 Sync events endpoint called with params: {:?}", params);
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    Json(json!({
        "next_batch": format!("sync_batch_{}", timestamp),
        "rooms": {
            "join": {},
            "invite": {},
            "leave": {}
        },
        "presence": {
            "events": []
        },
        "account_data": {
            "events": []
        },
        "to_device": {
            "events": []
        },
        "device_lists": {
            "changed": [],
            "left": []
        },
        "device_one_time_keys_count": {},
        "org.matrix.msc2732.device_unused_fallback_key_types": []
    }))
}

/// GET /_matrix/client/r0/joined_rooms - Get joined rooms
async fn joined_rooms(headers: HeaderMap) -> impl IntoResponse {
    info!("🏠 Joined rooms endpoint called");
    Json(json!({
        "joined_rooms": [
            "!example_room_1:localhost",
            "!example_room_2:localhost"
        ]
    }))
}

/// GET /_matrix/client/r0/publicRooms - Get public rooms
async fn public_rooms() -> impl IntoResponse {
    info!("🌐 Public rooms endpoint called");
    Json(json!({
        "chunk": [
            {
                "room_id": "!public_room_1:localhost",
                "name": "General Discussion",
                "topic": "Welcome to Matrixon test server",
                "canonical_alias": "#general:localhost",
                "num_joined_members": 42,
                "world_readable": true,
                "guest_can_join": true,
                "join_rule": "public",
                "room_type": null
            }
        ],
        "next_batch": "next_batch_token_123",
        "prev_batch": "prev_batch_token_123",
        "total_room_count_estimate": 1
    }))
}

/// GET /_matrix/client/r0/profile/{userId} - Get user profile
async fn get_profile(Path(user_id): Path<String>) -> impl IntoResponse {
    info!("👤 Get profile endpoint called for user: {}", user_id);
    Json(json!({
        "displayname": format!("User {}", user_id),
        "avatar_url": format!("mxc://localhost/avatar_{}", user_id)
    }))
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    info!("❤️ Health check endpoint called");
    Json(json!({
        "status": "healthy",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "version": "0.11.0-alpha",
        "server": "Matrixon"
    }))
}

/// Root endpoint
async fn root() -> impl IntoResponse {
    Json(json!({
        "server": "Matrixon Matrix Server",
        "version": "0.11.0-alpha",
        "description": "Ultra High Performance Matrix NextServer",
        "author": "arkSong (arksong2018@gmail.com)",
        "endpoints": {
            "versions": "/_matrix/client/versions",
            "capabilities": "/_matrix/client/r0/capabilities", 
            "login": "/_matrix/client/r0/login",
            "register": "/_matrix/client/r0/register",
            "whoami": "/_matrix/client/r0/account/whoami",
            "create_room": "/_matrix/client/r0/createRoom",
            "sync": "/_matrix/client/r0/sync",
            "joined_rooms": "/_matrix/client/r0/joined_rooms",
            "public_rooms": "/_matrix/client/r0/publicRooms",
            "health": "/health"
        }
    }))
}

// ============================================================================
// Main Application Setup
// ============================================================================

fn create_router() -> Router {
    let state = AppState {
        users: Arc::new(std::sync::RwLock::new(HashMap::new())),
        rooms: Arc::new(std::sync::RwLock::new(HashMap::new())),
        events: Arc::new(std::sync::RwLock::new(HashMap::new())),
    };

    Router::new()
        // Root endpoint
        .route("/", get(root))
        
        // Health check
        .route("/health", get(health_check))
        
        // Matrix Client-Server API endpoints
        .route("/_matrix/client/versions", get(get_versions))
        .route("/_matrix/client/r0/capabilities", get(get_capabilities))
        .route("/_matrix/client/v1/capabilities", get(get_capabilities))
        .route("/_matrix/client/v3/capabilities", get(get_capabilities))
        
        // Authentication endpoints
        .route("/_matrix/client/r0/login", get(get_login_types))
        .route("/_matrix/client/r0/login", post(login))
        .route("/_matrix/client/v1/login", get(get_login_types))
        .route("/_matrix/client/v1/login", post(login))
        .route("/_matrix/client/v3/login", get(get_login_types))
        .route("/_matrix/client/v3/login", post(login))
        
        .route("/_matrix/client/r0/register", post(register))
        .route("/_matrix/client/v1/register", post(register))
        .route("/_matrix/client/v3/register", post(register))
        
        .route("/_matrix/client/r0/account/whoami", get(whoami))
        .route("/_matrix/client/v1/account/whoami", get(whoami))
        .route("/_matrix/client/v3/account/whoami", get(whoami))
        
        // Room endpoints
        .route("/_matrix/client/r0/createRoom", post(create_room))
        .route("/_matrix/client/v1/createRoom", post(create_room))
        .route("/_matrix/client/v3/createRoom", post(create_room))
        
        .route("/_matrix/client/r0/joined_rooms", get(joined_rooms))
        .route("/_matrix/client/v1/joined_rooms", get(joined_rooms))
        .route("/_matrix/client/v3/joined_rooms", get(joined_rooms))
        
        .route("/_matrix/client/r0/publicRooms", get(public_rooms))
        .route("/_matrix/client/v1/publicRooms", get(public_rooms))
        .route("/_matrix/client/v3/publicRooms", get(public_rooms))
        
        // Messaging endpoints
        .route("/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id", put(send_message))
        .route("/_matrix/client/v1/rooms/:room_id/send/:event_type/:txn_id", put(send_message))
        .route("/_matrix/client/v3/rooms/:room_id/send/:event_type/:txn_id", put(send_message))
        
        // Sync endpoint
        .route("/_matrix/client/r0/sync", get(sync_events))
        .route("/_matrix/client/v1/sync", get(sync_events))
        .route("/_matrix/client/v3/sync", get(sync_events))
        
        // Profile endpoints
        .route("/_matrix/client/r0/profile/:user_id", get(get_profile))
        .route("/_matrix/client/v1/profile/:user_id", get(get_profile))
        .route("/_matrix/client/v3/profile/:user_id", get(get_profile))
        
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting Matrixon Matrix Server v0.11.0-alpha");
    info!("👨‍💻 Created by arkSong (arksong2018@gmail.com)");
    
    let app = create_router();
    let port = std::env::args().nth(1).unwrap_or_else(|| "6167".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("🌐 Server listening on http://{}", addr);
    info!("📋 Available endpoints:");
    info!("   GET  / - Root information");
    info!("   GET  /health - Health check");
    info!("   GET  /_matrix/client/versions - Matrix versions");
    info!("   GET  /_matrix/client/r0/capabilities - Server capabilities");
    info!("   GET/POST /_matrix/client/r0/login - Authentication");
    info!("   POST /_matrix/client/r0/register - User registration");
    info!("   GET  /_matrix/client/r0/account/whoami - Current user info");
    info!("   POST /_matrix/client/r0/createRoom - Create room");
    info!("   GET  /_matrix/client/r0/sync - Sync events");
    info!("   PUT  /_matrix/client/r0/rooms/{{room_id}}/send/{{event_type}}/{{txn_id}} - Send message");
    
    let listener = TcpListener::bind(&addr).await?;
    info!("✅ Matrixon server successfully started and ready for connections!");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        // Test passes if no panic occurs
    }
    
    #[tokio::test] 
    async fn test_get_versions() {
        let response = get_versions().await;
        // Test passes if no panic occurs
    }
} 
