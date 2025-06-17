// =============================================================================
// Standalone Matrixon Matrix Server - For Testing User Login
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Date: 2024-12-11
// Version: 0.11.0-alpha
// License: Apache 2.0 / MIT
//
// Description:
//   Standalone Matrixon Matrix server for testing user login functionality
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
async fn login(State(state): State<AppState>, Json(payload): Json<Value>) -> impl IntoResponse {
    info!("🔓 User login endpoint called with payload: {:?}", payload);
    
    // Extract login credentials
    let login_type = payload["type"].as_str().unwrap_or("");
    let identifier = payload["identifier"]["user"].as_str()
        .or_else(|| payload["user"].as_str())
        .unwrap_or("");
    let password = payload["password"].as_str().unwrap_or("");
    
    info!("🔍 Login attempt - Type: {}, User: {}", login_type, identifier);
    
    // For demo purposes, allow any user/password combination
    if identifier.is_empty() || password.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "errcode": "M_MISSING_PARAM",
            "error": "Missing user identifier or password"
        }))).into_response();
    }
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let user_id = if identifier.starts_with('@') {
        identifier.to_string()
    } else {
        format!("@{}:localhost", identifier)
    };
    
    // Create or update user
    let device_id = format!("LOGIN_DEVICE_{}", timestamp);
    let access_token = format!("syt_{}_{}", identifier, timestamp);
    
    {
        let mut users = state.users.write().unwrap();
        users.insert(user_id.clone(), User {
            user_id: user_id.clone(),
            display_name: Some(format!("User {}", identifier)),
            avatar_url: None,
            password_hash: password.to_string(), // In real implementation, this should be hashed
            access_tokens: vec![access_token.clone()],
            device_id: device_id.clone(),
            created_at: timestamp,
        });
    }
    
    info!("✅ Login successful for user: {}", user_id);
    
    Json(json!({
        "user_id": user_id,
        "access_token": access_token,
        "device_id": device_id,
        "well_known": {
            "m.homeserver": {
                "base_url": "http://localhost:6167"
            }
        }
    }))
}

/// POST /_matrix/client/r0/register - User registration
async fn register(State(state): State<AppState>, Json(payload): Json<Value>) -> impl IntoResponse {
    info!("🔐 User registration endpoint called with payload: {:?}", payload);
    
    let username = payload["username"].as_str().unwrap_or("");
    let password = payload["password"].as_str().unwrap_or("");
    
    if username.is_empty() || password.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "errcode": "M_MISSING_PARAM",
            "error": "Missing username or password"
        }))).into_response();
    }
    
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let user_id = format!("@{}:localhost", username);
    let device_id = format!("REGISTER_DEVICE_{}", timestamp);
    let access_token = format!("syt_{}_{}", username, timestamp);
    
    // Check if user already exists
    {
        let users = state.users.read().unwrap();
        if users.contains_key(&user_id) {
            return (StatusCode::CONFLICT, Json(json!({
                "errcode": "M_USER_IN_USE",
                "error": "User ID already taken"
            }))).into_response();
        }
    }
    
    // Create new user
    {
        let mut users = state.users.write().unwrap();
        users.insert(user_id.clone(), User {
            user_id: user_id.clone(),
            display_name: Some(format!("User {}", username)),
            avatar_url: None,
            password_hash: password.to_string(), // In real implementation, this should be hashed
            access_tokens: vec![access_token.clone()],
            device_id: device_id.clone(),
            created_at: timestamp,
        });
    }
    
    info!("✅ Registration successful for user: {}", user_id);
    
    Json(json!({
        "user_id": user_id,
        "access_token": access_token,
        "device_id": device_id
    }))
}

/// GET /_matrix/client/r0/account/whoami - Get current user info
async fn whoami(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    info!("👤 Whoami endpoint called");
    
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    
    let token = if auth_header.starts_with("Bearer ") {
        &auth_header[7..]
    } else {
        auth_header
    };
    
    // Find user by access token
    let users = state.users.read().unwrap();
    for user in users.values() {
        if user.access_tokens.contains(&token.to_string()) {
            return Json(json!({
                "user_id": user.user_id,
                "device_id": user.device_id,
                "is_guest": false
            })).into_response();
        }
    }
    
    (StatusCode::UNAUTHORIZED, Json(json!({
        "errcode": "M_UNKNOWN_TOKEN",
        "error": "Invalid access token"
    }))).into_response()
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    info!("❤️ Health check endpoint called");
    Json(json!({
        "status": "healthy",
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        "version": "0.11.0-alpha",
        "server": "Standalone Matrixon"
    }))
}

/// Root endpoint
async fn root() -> impl IntoResponse {
    Json(json!({
        "server": "Standalone Matrixon Matrix Server",
        "version": "0.11.0-alpha",
        "description": "Ultra High Performance Matrix NextServer",
        "author": "arkSong (arksong2018@gmail.com)",
        "endpoints": {
            "versions": "/_matrix/client/versions",
            "login_types": "/_matrix/client/r0/login",
            "login": "POST /_matrix/client/r0/login",
            "register": "POST /_matrix/client/r0/register",
            "whoami": "/_matrix/client/r0/account/whoami",
            "health": "/health"
        },
        "test_commands": {
            "register": "curl -X POST http://localhost:6167/_matrix/client/r0/register -H \"Content-Type: application/json\" -d '{\"username\":\"testuser\",\"password\":\"testpass\"}'",
            "login": "curl -X POST http://localhost:6167/_matrix/client/r0/login -H \"Content-Type: application/json\" -d '{\"type\":\"m.login.password\",\"identifier\":{\"type\":\"m.id.user\",\"user\":\"testuser\"},\"password\":\"testpass\"}'",
            "whoami": "curl -H \"Authorization: Bearer YOUR_ACCESS_TOKEN\" http://localhost:6167/_matrix/client/r0/account/whoami"
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
        
        // Authentication endpoints
        .route("/_matrix/client/r0/login", get(get_login_types).post(login))
        .route("/_matrix/client/v1/login", get(get_login_types).post(login))
        .route("/_matrix/client/v3/login", get(get_login_types).post(login))
        
        .route("/_matrix/client/r0/register", post(register))
        .route("/_matrix/client/v1/register", post(register))
        .route("/_matrix/client/v3/register", post(register))
        
        .route("/_matrix/client/r0/account/whoami", get(whoami))
        .route("/_matrix/client/v1/account/whoami", get(whoami))
        .route("/_matrix/client/v3/account/whoami", get(whoami))
        
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

    info!("🚀 Starting Standalone Matrixon Matrix Server v0.11.0-alpha");
    info!("👨‍💻 Created by arkSong (arksong2018@gmail.com)");
    
    let app = create_router();
    let port = std::env::args().nth(1).unwrap_or_else(|| "6167".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    info!("🌐 Server listening on http://{}", addr);
    info!("📋 Available endpoints:");
    info!("   GET  / - Root information and test commands");
    info!("   GET  /health - Health check");
    info!("   GET  /_matrix/client/versions - Matrix versions");
    info!("   GET/POST /_matrix/client/r0/login - Authentication");
    info!("   POST /_matrix/client/r0/register - User registration");
    info!("   GET  /_matrix/client/r0/account/whoami - Current user info");
    info!("");
    info!("🔧 Test commands:");
    info!("   Register: curl -X POST http://localhost:6167/_matrix/client/r0/register -H \"Content-Type: application/json\" -d '{{\"username\":\"testuser\",\"password\":\"testpass\"}}'");
    info!("   Login: curl -X POST http://localhost:6167/_matrix/client/r0/login -H \"Content-Type: application/json\" -d '{{\"type\":\"m.login.password\",\"identifier\":{{\"type\":\"m.id.user\",\"user\":\"testuser\"}},\"password\":\"testpass\"}}'");
    
    let listener = TcpListener::bind(&addr).await?;
    info!("✅ Standalone Matrixon server successfully started and ready for connections!");
    
    axum::serve(listener, app).await?;
    
    Ok(())
} 
