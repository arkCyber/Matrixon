//! Request handlers for Matrixon API
//! 
//! This module provides request handlers for the Matrixon API endpoints.

use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use matrixon_core::MatrixonError;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Server status
    pub status: String,
    
    /// Server version
    pub version: String,
}

/// Version response
#[derive(Debug, Serialize)]
pub struct VersionResponse {
    /// Supported versions
    pub versions: Vec<String>,
}

/// Health check handler
pub async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    
    (StatusCode::OK, Json(response))
}

/// Version handler
pub async fn versions() -> impl IntoResponse {
    let response = VersionResponse {
        versions: vec!["r0.6.0".to_string()],
    };
    
    (StatusCode::OK, Json(response))
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error code
    pub errcode: String,
    
    /// Error message
    pub error: String,
}

impl From<MatrixonError> for ErrorResponse {
    fn from(err: MatrixonError) -> Self {
        Self {
            errcode: "M_UNKNOWN".to_string(),
            error: err.to_string(),
        }
    }
}

/// Login handler
pub async fn login() -> impl IntoResponse {
    (StatusCode::OK, Json("Login endpoint"))
}

/// Register handler
pub async fn register() -> impl IntoResponse {
    (StatusCode::OK, Json("Register endpoint"))
}

/// Join room handler
pub async fn join_room() -> impl IntoResponse {
    (StatusCode::OK, Json("Join room endpoint"))
}

/// Leave room handler
pub async fn leave_room() -> impl IntoResponse {
    (StatusCode::OK, Json("Leave room endpoint"))
}

/// Get messages handler
pub async fn get_messages() -> impl IntoResponse {
    (StatusCode::OK, Json("Get messages endpoint"))
}

/// Send event handler
pub async fn send_event() -> impl IntoResponse {
    (StatusCode::OK, Json("Send event endpoint"))
}

/// Get state handler
pub async fn get_state() -> impl IntoResponse {
    (StatusCode::OK, Json("Get state endpoint"))
}

/// Get profile handler
pub async fn get_profile() -> impl IntoResponse {
    (StatusCode::OK, Json("Get profile endpoint"))
}

/// Set displayname handler
pub async fn set_displayname() -> impl IntoResponse {
    (StatusCode::OK, Json("Set displayname endpoint"))
}

/// Set avatar handler
pub async fn set_avatar() -> impl IntoResponse {
    (StatusCode::OK, Json("Set avatar endpoint"))
}

/// Get devices handler
pub async fn get_devices() -> impl IntoResponse {
    (StatusCode::OK, Json("Get devices endpoint"))
}

/// Get device handler
pub async fn get_device() -> impl IntoResponse {
    (StatusCode::OK, Json("Get device endpoint"))
}

/// Update device handler
pub async fn update_device() -> impl IntoResponse {
    (StatusCode::OK, Json("Update device endpoint"))
}

/// Federation version handler
pub async fn federation_version() -> impl IntoResponse {
    (StatusCode::OK, Json("Federation version endpoint"))
}

/// Federation profile handler
pub async fn federation_profile() -> impl IntoResponse {
    (StatusCode::OK, Json("Federation profile endpoint"))
}

#[cfg(test)]
mod tests {
    
    use axum::http::StatusCode;

    use tower::ServiceExt;
    use axum::body::Body;
    use axum::http::Request;

    #[tokio::test]
    async fn test_health_check() {
        let app = super::super::routes::create_router();
        let response = app
            .oneshot(Request::builder()
                .uri("/_matrix/client/health")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
            
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_versions() {
        let app = super::super::routes::create_router();
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
