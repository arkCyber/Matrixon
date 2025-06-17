//! Health Check Module
//!
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: Implements health check endpoints and status tracking for the Matrixon monitoring system. Provides HTTP endpoints for health, readiness, and liveness checks.
//!
//! All code is documented in English, with detailed function documentation, error handling, and performance characteristics.

use std::{
    net::SocketAddr,
    collections::HashMap
};
use axum::{routing::get, Router, Json, response::IntoResponse, http::StatusCode};
use serde::{Serialize, Deserialize};
use tokio::task::JoinHandle;
use tracing::{info, error, instrument};
use std::sync::Arc;

use crate::config::HealthConfig;
use crate::metrics::MetricsManager;
use crate::error::Result;

/// Health status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

impl IntoResponse for HealthStatus {
    fn into_response(self) -> axum::response::Response {
        match self {
            HealthStatus::Healthy => (StatusCode::OK, Json(self)).into_response(),
            HealthStatus::Degraded => (StatusCode::OK, Json(self)).into_response(),
            HealthStatus::Unhealthy => (StatusCode::SERVICE_UNAVAILABLE, Json(self)).into_response(),
        }
    }
}

/// Health manager for managing health checks and status
#[derive(Debug)]
pub struct HealthManager {
    config: HealthConfig,
    metrics: Arc<MetricsManager>,
    server_handle: Option<JoinHandle<()>>,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new(config: HealthConfig, metrics: Arc<MetricsManager>) -> Self {
        Self {
            config,
            metrics,
            server_handle: None,
        }
    }

    /// Initialize the health manager
    #[instrument(level = "info", skip(self))]
    pub async fn init(&mut self) -> Result<()> {
        if !self.config.enabled {
            info!("Health checks are disabled");
            return Ok(());
        }
        
        let port = self.config.endpoints.first()
            .and_then(|e| e.path.split(':').last())
            .unwrap_or("8080")
            .parse()
            .unwrap_or(8080);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let metrics = self.metrics.clone();
        
        let server = tokio::spawn(async move {
            let app = Router::new()
                .route("/health", get(health_handler))
                .route("/ready", get(ready_handler))
                .route("/live", get(live_handler));
            
            info!("Starting health check server on {}", addr);
            if let Ok(listener) = tokio::net::TcpListener::bind(&addr).await {
                axum::serve(listener, app)
                    .await
                    .unwrap_or_else(|e| error!("Health server error: {}", e));
            } else {
                error!("Failed to bind health server to {}", addr);
            }
        });
        
        self.server_handle = Some(server);
        info!("Health check system initialized");
        Ok(())
    }

    /// Shutdown the health manager
    #[instrument(level = "info", skip(self))]
    pub async fn shutdown(&mut self) {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        info!("Health check system shut down");
    }

    /// Check health status
    #[instrument(level = "debug", skip(self))]
    async fn check_status(&self) -> Result<Status> {
        let status = if self.metrics.get_metrics().await.is_ok() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };
        
        Ok(Status {
            status,
            message: "System health check".to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }

    pub async fn check_health(&self) -> Result<HashMap<String, String>> {
        let status = self.check_status().await?;
        let mut map = HashMap::new();
        map.insert("status".to_string(), status.status.to_string());
        map.insert("message".to_string(), status.message);
        map.insert("timestamp".to_string(), status.timestamp.to_string());
        Ok(map)
    }
}

/// Health status details
#[derive(Debug)]
struct Status {
    status: HealthStatus,
    message: String,
    timestamp: u64,
}

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub timestamp: u64,
}

/// Handler for /health endpoint
async fn health_handler() -> impl IntoResponse {
    let status = check_health().await;
    let response = HealthResponse {
        status,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    (StatusCode::OK, Json(response))
}

/// Handler for /ready endpoint
async fn ready_handler() -> impl IntoResponse {
    let status = check_readiness().await;
    let response = HealthResponse {
        status,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    (StatusCode::OK, Json(response))
}

/// Handler for /live endpoint
async fn live_handler() -> impl IntoResponse {
    let status = check_liveness().await;
    let response = HealthResponse {
        status,
        timestamp: chrono::Utc::now().timestamp() as u64,
    };
    (StatusCode::OK, Json(response))
}

/// Check overall system health
#[instrument(level = "debug")]
pub async fn check_health() -> HealthStatus {
    // TODO: Add real health checks (DB, cache, etc.)
    HealthStatus::Healthy
}

/// Check readiness
#[instrument(level = "debug")]
pub async fn check_readiness() -> HealthStatus {
    // TODO: Add real readiness checks
    HealthStatus::Healthy
}

/// Check liveness
#[instrument(level = "debug")]
pub async fn check_liveness() -> HealthStatus {
    // TODO: Add real liveness checks
    HealthStatus::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[tokio::test]
    async fn test_health_endpoints() {
        assert_eq!(check_health().await, HealthStatus::Healthy);
        assert_eq!(check_readiness().await, HealthStatus::Healthy);
        assert_eq!(check_liveness().await, HealthStatus::Healthy);
    }
}
