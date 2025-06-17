/*
 * =============================================================================
 * Matrixon Matrix Server - Simple Main Entry Point
 * =============================================================================
 *
 * Project: Matrixon - Ultra High Performance Matrix NextServer
 * Author: arkSong (arksong2018@gmail.com)
 * Date: 2024-12-19
 * Version: 0.11.0-alpha
 *
 * Description:
 *   Simple main entry point for Matrixon Matrix Server
 *   Provides basic Matrix API endpoints for production testing
 *
 * Performance Targets:
 *   - 200k+ concurrent connections
 *   - <50ms response latency
 *   - >99% uptime
 *
 * =============================================================================
 */

use std::time::Instant;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Query, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Json, Response},
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing::{info, error, debug, instrument};

// Basic Matrix API structures
#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    #[serde(rename = "type")]
    login_type: String,
    user: String,
    password: String,
    device_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct LoginResponse {
    user_id: String,
    access_token: String,
    device_id: String,
    home_server: String,
    well_known: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    device_id: Option<String>,
    initial_device_display_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RegisterResponse {
    user_id: String,
    access_token: String,
    device_id: String,
    home_server: String,
}

#[derive(Serialize, Deserialize)]
struct VersionsResponse {
    versions: Vec<String>,
    unstable_features: HashMap<String, bool>,
}

#[derive(Serialize, Deserialize)]
struct CapabilitiesResponse {
    capabilities: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize)]
struct WhoAmIResponse {
    user_id: String,
    device_id: Option<String>,
    is_guest: bool,
}

// Room management structures
#[derive(Debug, Serialize, Deserialize)]
struct CreateRoomRequest {
    name: Option<String>,
    topic: Option<String>,
    preset: Option<String>,
    visibility: Option<String>,
    room_alias_name: Option<String>,
    invite: Option<Vec<String>>,
    room_version: Option<String>,
    power_level_content_override: Option<Value>,
    initial_state: Option<Vec<Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateRoomResponse {
    room_id: String,
    room_alias: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendMessageRequest {
    msgtype: String,
    body: String,
    format: Option<String>,
    formatted_body: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SendMessageResponse {
    event_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JoinRoomResponse {
    room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoomEvent {
    event_id: String,
    sender: String,
    #[serde(rename = "type")]
    event_type: String,
    content: Value,
    origin_server_ts: u64,
    room_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SyncResponse {
    next_batch: String,
    rooms: RoomsResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct RoomsResponse {
    join: HashMap<String, JoinedRoomResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JoinedRoomResponse {
    timeline: TimelineResponse,
    state: StateResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct TimelineResponse {
    events: Vec<RoomEvent>,
    limited: bool,
    prev_batch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateResponse {
    events: Vec<RoomEvent>,
}

// Simple in-memory storage
#[derive(Debug, Clone)]
struct Room {
    id: String,
    name: Option<String>,
    topic: Option<String>,
    creator: String,
    members: Vec<String>,
    messages: Vec<RoomEvent>,
    created_at: u64,
}

#[derive(Debug, Clone, Default)]
struct AppState {
    rooms: Arc<Mutex<HashMap<String, Room>>>,
    room_aliases: Arc<Mutex<HashMap<String, String>>>,
}

// Health check endpoint
#[instrument(level = "debug")]
async fn health_check() -> Json<Value> {
    let start = Instant::now();
    debug!("🔧 Health check requested");
    
    let response = json!({
        "status": "healthy",
        "service": "matrixon",
        "version": "0.11.0-alpha",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "uptime_ms": start.elapsed().as_millis()
    });
    
    info!("✅ Health check completed in {:?}", start.elapsed());
    Json(response)
}

// Metrics endpoint (Prometheus format)
#[instrument(level = "debug")]
async fn metrics() -> Response {
    let start = Instant::now();
    debug!("🔧 Metrics requested");
    
    let metrics = format!(
        "# HELP matrixon_requests_total Total number of requests\n\
         # TYPE matrixon_requests_total counter\n\
         matrixon_requests_total{{endpoint=\"health\"}} 1\n\
         # HELP matrixon_response_time_seconds Response time in seconds\n\
         # TYPE matrixon_response_time_seconds histogram\n\
         matrixon_response_time_seconds_bucket{{le=\"0.1\"}} 1\n\
         matrixon_response_time_seconds_count 1\n\
         matrixon_response_time_seconds_sum {:.6}\n",
        start.elapsed().as_secs_f64()
    );
    
    info!("✅ Metrics exported in {:?}", start.elapsed());
    
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics.into())
        .unwrap()
}

// Matrix Client API - Get supported versions
#[instrument(level = "debug")]
async fn get_versions() -> Json<VersionsResponse> {
    let start = Instant::now();
    debug!("🔧 Matrix versions requested");
    
    let response = VersionsResponse {
        versions: vec![
            "r0.0.1".to_string(),
            "r0.1.0".to_string(),
            "r0.2.0".to_string(),
            "r0.3.0".to_string(),
            "r0.4.0".to_string(),
            "r0.5.0".to_string(),
            "r0.6.0".to_string(),
            "r0.6.1".to_string(),
            "v1.1".to_string(),
            "v1.2".to_string(),
            "v1.3".to_string(),
            "v1.4".to_string(),
            "v1.5".to_string(),
            "v1.6".to_string(),
        ],
        unstable_features: {
            let mut features = HashMap::new();
            features.insert("org.matrix.msc2285.stable".to_string(), true);
            features.insert("org.matrix.msc3440.stable".to_string(), true);
            features
        },
    };
    
    info!("✅ Matrix versions returned in {:?}", start.elapsed());
    Json(response)
}

// Matrix Client API - Get capabilities
#[instrument(level = "debug")]
async fn get_capabilities() -> Json<CapabilitiesResponse> {
    let start = Instant::now();
    debug!("🔧 Matrix capabilities requested");
    
    let response = CapabilitiesResponse {
        capabilities: {
            let mut caps = HashMap::new();
            caps.insert("m.change_password".to_string(), json!({"enabled": true}));
            caps.insert("m.room_versions".to_string(), json!({
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
            }));
            caps.insert("m.set_displayname".to_string(), json!({"enabled": true}));
            caps.insert("m.set_avatar_url".to_string(), json!({"enabled": true}));
            caps.insert("m.3pid_changes".to_string(), json!({"enabled": true}));
            caps
        },
    };
    
    info!("✅ Matrix capabilities returned in {:?}", start.elapsed());
    Json(response)
}

// User registration
#[instrument(level = "debug")]
async fn register_user(Json(request): Json<RegisterRequest>) -> Result<Json<RegisterResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 User registration requested for: {}", request.username);
    
    // Simple validation
    if request.username.is_empty() || request.password.is_empty() {
        error!("❌ Registration failed: invalid username or password");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let user_id = format!("@{}:localhost", request.username);
    let device_id = request.device_id.unwrap_or_else(|| {
        format!("DEVICE_{}", chrono::Utc::now().timestamp())
    });
    
    let response = RegisterResponse {
        user_id: user_id.clone(),
        access_token: format!("matx_access_token_{}_{}", request.username, chrono::Utc::now().timestamp()),
        device_id,
        home_server: "localhost".to_string(),
    };
    
    info!("✅ User {} registered successfully in {:?}", user_id, start.elapsed());
    Ok(Json(response))
}

// User login
#[instrument(level = "debug")]
async fn login_user(Json(request): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 User login requested for: {}", request.user);
    
    // Simple validation
    if request.login_type != "m.login.password" {
        error!("❌ Login failed: unsupported login type");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    if request.user.is_empty() || request.password.is_empty() {
        error!("❌ Login failed: invalid credentials");
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    let user_id = if request.user.starts_with('@') {
        request.user.clone()
    } else {
        format!("@{}:localhost", request.user)
    };
    
    let device_id = request.device_id.unwrap_or_else(|| {
        format!("DEVICE_{}", chrono::Utc::now().timestamp())
    });
    
    let response = LoginResponse {
        user_id: user_id.clone(),
        access_token: format!("matx_access_token_{}_{}", request.user, chrono::Utc::now().timestamp()),
        device_id,
        home_server: "localhost".to_string(),
        well_known: Some(json!({
            "m.homeserver": {
                "base_url": "http://localhost:6167"
            }
        })),
    };
    
    info!("✅ User {} logged in successfully in {:?}", user_id, start.elapsed());
    Ok(Json(response))
}

// Who am I endpoint
#[instrument(level = "debug")]
async fn whoami(headers: HeaderMap) -> Result<Json<WhoAmIResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 WhoAmI requested");
    
    // Extract access token from Authorization header
    let auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Simple token validation (in production, this should validate against database)
    if !auth_header.starts_with("matx_access_token_") {
        error!("❌ WhoAmI failed: invalid access token");
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Extract user from token (simplified)
    let user_id = "@test_user:localhost".to_string(); // In production, extract from token
    
    let response = WhoAmIResponse {
        user_id: user_id.clone(),
        device_id: Some("DEVICE_123".to_string()),
        is_guest: false,
    };
    
    info!("✅ WhoAmI returned for {} in {:?}", user_id, start.elapsed());
    Ok(Json(response))
}

// Get joined rooms
#[instrument(level = "debug")]
async fn get_joined_rooms(headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Joined rooms requested");
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let response = json!({
        "joined_rooms": [
            "!example:localhost",
            "!test:localhost"
        ]
    });
    
    info!("✅ Joined rooms returned in {:?}", start.elapsed());
    Ok(Json(response))
}

// Get devices
#[instrument(level = "debug")]
async fn get_devices(headers: HeaderMap) -> Result<Json<Value>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Devices requested");
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let response = json!({
        "devices": [
            {
                "device_id": "DEVICE_123",
                "display_name": "Matrixon Test Device",
                "last_seen_ip": "127.0.0.1",
                "last_seen_ts": chrono::Utc::now().timestamp_millis()
            }
        ]
    });
    
    info!("✅ Devices returned in {:?}", start.elapsed());
    Ok(Json(response))
}

// Federation version endpoint
#[instrument(level = "debug")]
async fn federation_version() -> Json<Value> {
    let start = Instant::now();
    debug!("🔧 Federation version requested");
    
    let response = json!({
        "server": {
            "name": "Matrixon",
            "version": "0.11.0-alpha"
        }
    });
    
    info!("✅ Federation version returned in {:?}", start.elapsed());
    Json(response)
}

// Check registration availability
#[instrument(level = "debug")]
async fn registration_available(Query(params): Query<HashMap<String, String>>) -> Json<Value> {
    let start = Instant::now();
    debug!("🔧 Registration availability checked");
    
    let username = params.get("username").cloned().unwrap_or_default();
    
    let response = json!({
        "available": true,
        "username": username
    });
    
    info!("✅ Registration availability checked in {:?}", start.elapsed());
    Json(response)
}

// Create room endpoint
#[instrument(level = "debug")]
async fn create_room(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Room creation requested");
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Extract user_id from token (simplified)
    let user_id = format!("@test_user:localhost");
    
    let room_id = format!("!{}:localhost", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let room_alias = request.room_alias_name.as_ref()
        .map(|alias| format!("#{}:localhost", alias));
    
    let room = Room {
        id: room_id.clone(),
        name: request.name.clone(),
        topic: request.topic.clone(),
        creator: user_id.clone(),
        members: vec![user_id],
        messages: vec![],
        created_at: chrono::Utc::now().timestamp_millis() as u64,
    };
    
    // Store room
    {
        let mut rooms = state.rooms.lock().unwrap();
        rooms.insert(room_id.clone(), room);
    }
    
    // Store alias if provided
    if let Some(ref alias) = room_alias {
        let mut aliases = state.room_aliases.lock().unwrap();
        aliases.insert(alias.clone(), room_id.clone());
    }
    
    let response = CreateRoomResponse {
        room_id,
        room_alias,
    };
    
    info!("✅ Room created successfully in {:?}", start.elapsed());
    Ok(Json(response))
}

// Join room endpoint
#[instrument(level = "debug")]
async fn join_room(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(room_id_or_alias): Path<String>,
) -> Result<Json<JoinRoomResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Join room requested: {}", room_id_or_alias);
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user_id = "@test_user:localhost".to_string();
    
    // Resolve room alias to room ID if necessary
    let room_id = if room_id_or_alias.starts_with('#') {
        let aliases = state.room_aliases.lock().unwrap();
        aliases.get(&room_id_or_alias).cloned()
            .unwrap_or_else(|| room_id_or_alias.clone())
    } else {
        room_id_or_alias.clone()
    };
    
    // Add user to room
    {
        let mut rooms = state.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            if !room.members.contains(&user_id) {
                room.members.push(user_id);
            }
        }
    }
    
    let response = JoinRoomResponse {
        room_id: room_id.clone(),
    };
    
    info!("✅ Joined room {} in {:?}", room_id, start.elapsed());
    Ok(Json(response))
}

// Send message to room
#[instrument(level = "debug")]
async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((room_id, event_type, txn_id)): Path<(String, String, String)>,
    Json(request): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Send message to room: {}", room_id);
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user_id = "@test_user:localhost".to_string();
    let event_id = format!("${}:localhost", uuid::Uuid::new_v4());
    
    let event = RoomEvent {
        event_id: event_id.clone(),
        sender: user_id,
        event_type,
        content: json!({
            "msgtype": request.msgtype,
            "body": request.body,
            "format": request.format,
            "formatted_body": request.formatted_body
        }),
        origin_server_ts: chrono::Utc::now().timestamp_millis() as u64,
        room_id: room_id.clone(),
    };
    
    // Store message in room
    {
        let mut rooms = state.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            room.messages.push(event);
        }
    }
    
    let response = SendMessageResponse {
        event_id,
    };
    
    info!("✅ Message sent to room {} in {:?}", room_id, start.elapsed());
    Ok(Json(response))
}

// Get room messages
#[instrument(level = "debug")]
async fn get_room_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Get room messages: {}", room_id);
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let limit = params.get("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(10);
    
    let messages = {
        let rooms = state.rooms.lock().unwrap();
        if let Some(room) = rooms.get(&room_id) {
            room.messages.iter()
                .rev()
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
    };
    
    let response = json!({
        "chunk": messages,
        "start": "t1-start",
        "end": "t1-end"
    });
    
    info!("✅ Room messages retrieved for {} in {:?}", room_id, start.elapsed());
    Ok(Json(response))
}

// Sync endpoint (simplified)
#[instrument(level = "debug")]
async fn sync(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<SyncResponse>, StatusCode> {
    let start = Instant::now();
    debug!("🔧 Sync requested");
    
    // Simple authentication check
    let _auth_header = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user_id = "@test_user:localhost".to_string();
    let _since = params.get("since").cloned().unwrap_or_default();
    
    let mut joined_rooms = HashMap::new();
    
    // Get rooms the user has joined
    {
        let rooms = state.rooms.lock().unwrap();
        for (room_id, room) in rooms.iter() {
            if room.members.contains(&user_id) {
                let timeline_events = room.messages.iter()
                    .rev()
                    .take(10)
                    .cloned()
                    .collect::<Vec<_>>();
                
                joined_rooms.insert(room_id.clone(), JoinedRoomResponse {
                    timeline: TimelineResponse {
                        events: timeline_events,
                        limited: false,
                        prev_batch: Some("prev_batch_token".to_string()),
                    },
                    state: StateResponse {
                        events: vec![],
                    },
                });
            }
        }
    }
    
    let response = SyncResponse {
        next_batch: format!("sync_token_{}", chrono::Utc::now().timestamp()),
        rooms: RoomsResponse {
            join: joined_rooms,
        },
    };
    
    info!("✅ Sync completed in {:?}", start.elapsed());
    Ok(Json(response))
}

// Initialize tracing
fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// Shutdown handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("🛑 Shutdown signal received");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // Initialize tracing
    init_tracing();
    
    info!("🚀 Starting Matrixon Matrix Server v0.11.0-alpha");
    info!("📡 Initializing high-performance Matrix NextServer");
    
    // Initialize application state
    let app_state = AppState::default();
    
    // Build the application router
    let app = Router::new()
        // Health and metrics
        .route("/health", get(health_check))
        .route("/metrics", get(metrics))
        
        // Matrix Client-Server API
        .route("/_matrix/client/versions", get(get_versions))
        .route("/_matrix/client/r0/capabilities", get(get_capabilities))
        .route("/_matrix/client/v1/capabilities", get(get_capabilities))
        .route("/_matrix/client/r0/register", post(register_user))
        .route("/_matrix/client/v1/register", post(register_user))
        .route("/_matrix/client/r0/register/available", get(registration_available))
        .route("/_matrix/client/v1/register/available", get(registration_available))
        .route("/_matrix/client/r0/login", post(login_user))
        .route("/_matrix/client/v1/login", post(login_user))
        .route("/_matrix/client/r0/account/whoami", get(whoami))
        .route("/_matrix/client/v1/account/whoami", get(whoami))
        .route("/_matrix/client/r0/joined_rooms", get(get_joined_rooms))
        .route("/_matrix/client/v1/joined_rooms", get(get_joined_rooms))
        .route("/_matrix/client/r0/devices", get(get_devices))
        .route("/_matrix/client/v1/devices", get(get_devices))
        
        // Room management endpoints
        .route("/_matrix/client/r0/createRoom", post(create_room))
        .route("/_matrix/client/v1/createRoom", post(create_room))
        .route("/_matrix/client/r0/join/:room_id_or_alias", post(join_room))
        .route("/_matrix/client/v1/join/:room_id_or_alias", post(join_room))
        .route("/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id", put(send_message))
        .route("/_matrix/client/v1/rooms/:room_id/send/:event_type/:txn_id", put(send_message))
        .route("/_matrix/client/r0/rooms/:room_id/messages", get(get_room_messages))
        .route("/_matrix/client/v1/rooms/:room_id/messages", get(get_room_messages))
        .route("/_matrix/client/r0/sync", get(sync))
        .route("/_matrix/client/v1/sync", get(sync))
        
        // Matrix Federation API
        .route("/_matrix/federation/v1/version", get(federation_version))
        .route("/_matrix/key/v2/server", get(federation_version))
        
        // Set application state
        .with_state(app_state)
        
        // Add service layers
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    // Get bind address from environment or use default
    let bind_addr = std::env::var("MATRIXON_BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let bind_port = std::env::var("MATRIXON_BIND_PORT")
        .unwrap_or_else(|_| "6167".to_string())
        .parse::<u16>()
        .unwrap_or(6167);
    
    let addr = SocketAddr::from((
        bind_addr.parse::<std::net::IpAddr>().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
        bind_port
    ));

    info!("🌐 Binding to address: {}", addr);
    info!("📊 Performance targets: 200k+ connections, <50ms latency");
    info!("⚡ Startup completed in {:?}", start_time.elapsed());

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("🎉 Matrixon Matrix Server ready! Listening on {}", addr);
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("🛑 Matrixon Matrix Server shutdown complete");
    Ok(())
} 
