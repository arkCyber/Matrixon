//! API routes for Matrixon
//! 
//! This module defines the API routes for the Matrixon server.

use axum::{
    Router,
    routing::{get, post, put},
};
use tower_http::trace::TraceLayer;

use crate::handlers;

/// Create the API router
pub fn create_router() -> Router {
    Router::new()
        // Client API routes
        .route("/_matrix/client/versions", get(handlers::versions))
        .route("/_matrix/client/health", get(handlers::health_check))
        
        // Authentication routes
        .route("/_matrix/client/r0/login", post(handlers::login))
        .route("/_matrix/client/r0/register", post(handlers::register))
        
        // Room routes
        .route("/_matrix/client/r0/rooms/:room_id/join", post(handlers::join_room))
        .route("/_matrix/client/r0/rooms/:room_id/leave", post(handlers::leave_room))
        .route("/_matrix/client/r0/rooms/:room_id/messages", get(handlers::get_messages))
        
        // Event routes
        .route("/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id", put(handlers::send_event))
        .route("/_matrix/client/r0/rooms/:room_id/state/:event_type/:state_key", get(handlers::get_state))
        
        // Profile routes
        .route("/_matrix/client/r0/profile/:user_id", get(handlers::get_profile))
        .route("/_matrix/client/r0/profile/:user_id/displayname", put(handlers::set_displayname))
        .route("/_matrix/client/r0/profile/:user_id/avatar_url", put(handlers::set_avatar))
        
        // Device routes
        .route("/_matrix/client/r0/devices", get(handlers::get_devices))
        .route("/_matrix/client/r0/devices/:device_id", get(handlers::get_device))
        .route("/_matrix/client/r0/devices/:device_id", put(handlers::update_device))
        
        // Federation routes
        .route("/_matrix/federation/v1/version", get(handlers::federation_version))
        .route("/_matrix/federation/v1/query/profile", get(handlers::federation_profile))
        
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode}
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_versions_endpoint() {
        let app = create_router();
        
        let response = app
            .oneshot(Request::builder()
                .uri("/_matrix/client/versions")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
            
        assert_eq!(response.status(), StatusCode::OK);
    }
}
