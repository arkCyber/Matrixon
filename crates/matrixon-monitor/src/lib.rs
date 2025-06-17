//! Matrixon Monitoring System
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Date: 2024-03-21
//! Version: 0.1.0
//!
//! Purpose: This crate provides comprehensive monitoring and metrics collection functionality for the Matrixon Matrix server, including system metrics, application metrics, Prometheus export, structured logging, health checks, alert management, and performance monitoring.
//!
//! All code is documented in English, with detailed function documentation, error handling, and performance characteristics.
//! 
//! This crate provides comprehensive monitoring and metrics collection
//! functionality for the Matrixon Matrix server. It includes:
//! - System metrics collection (CPU, memory, disk, network)
//! - Application metrics (request rates, latencies, error rates)
//! - Prometheus metrics export
//! - Structured logging
//! - Health check endpoints
//! - Alert management
//! - Performance monitoring
//! 

use std::{
    collections::HashMap,
    sync::Arc,
};

use tracing::{debug, info, instrument};
use serde::Serialize;
use axum::{
    Router, 
    routing::get, 
    response::{IntoResponse, Response}, 
    Json, 
    http::StatusCode,
    extract::State
};

use tokio::net::TcpListener;

pub mod config;
pub mod metrics;
pub mod system;
pub mod health;
pub mod logging;
pub mod alert;
pub mod performance;
pub mod error;

use config::MonitorConfig;
use metrics::MetricsManager;
use system::{SystemMonitor, SystemMetrics};
use health::HealthManager;
use alert::AlertManager;
use performance::PerformanceManager;
use crate::error::{Result, MonitorError};

#[derive(Debug, Serialize)]
struct MetricsResponse(HashMap<String, f64>);

impl IntoResponse for MetricsResponse {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

#[derive(Debug, Serialize)]
struct AlertsResponse(Vec<crate::alert::Alert>);

impl IntoResponse for AlertsResponse {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

/// Monitor service
#[derive(Debug)]
pub struct MonitorService {
    config: MonitorConfig,
    metrics: Arc<MetricsManager>,
    system: SystemMonitor,
    health: Arc<HealthManager>,
    alert: Arc<AlertManager>,
    performance: Arc<PerformanceManager>,
}

impl MonitorService {
    /// Create a new monitor service instance
    pub async fn new(config: MonitorConfig) -> Result<Self> {
        let metrics = Arc::new(MetricsManager::new(config.metrics.clone())?);
        let service = Self::new_with_metrics(&config, metrics).await?;
        Ok(service)
    }

    /// Create a new monitor service instance with existing metrics manager
    pub async fn new_with_metrics(config: &MonitorConfig, metrics: Arc<MetricsManager>) -> Result<Self> {
        let system = SystemMonitor::new(config.system.clone(), metrics.clone());
        let health = Arc::new(HealthManager::new(config.health.clone(), metrics.clone()));
        let alert = Arc::new(AlertManager::new().await?);
        let performance = Arc::new(PerformanceManager::new(config.performance.clone(), metrics.clone()));

        Ok(Self {
            config: config.clone(),
            metrics,
            system,
            health,
            alert,
            performance,
        })
    }

    /// Start the monitor service
    #[instrument(level = "debug", skip(self))]
    pub async fn start(&mut self) -> Result<()> {
        debug!("🔧 Starting monitor service");
        self.start_http_api().await?;
        Ok(())
    }

    /// Stop the monitor service
    #[instrument(level = "debug", skip(self))]
    pub async fn stop(&mut self) -> Result<()> {
        debug!("🛑 Stopping monitor service");
        Ok(())
    }

    async fn start_http_api(&self) -> Result<(), MonitorError> {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .route("/system", get(system_handler))
            .route("/alerts", get(alerts_handler))
            .route("/performance", get(performance_handler))
            .with_state(AppState {
                metrics: self.metrics.clone(),
                health: self.health.clone(),
                system: self.system.clone(),
                alert: self.alert.clone(),
                performance: self.performance.clone(),
            });

        let port = self.config.metrics.prometheus_endpoint.split(':').last().unwrap_or("3000");
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| MonitorError::HttpError(format!("Failed to bind to {}: {}", addr, e)))?;
        info!("HTTP API server listening on {}", addr);
        axum::serve(listener, app).await.map_err(|e| MonitorError::HttpError(format!("Failed to serve HTTP API: {}", e)))?;
        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    metrics: Arc<MetricsManager>,
    health: Arc<HealthManager>,
    system: SystemMonitor,
    alert: Arc<AlertManager>,
    performance: Arc<PerformanceManager>,
}

async fn metrics_handler(
    State(state): State<AppState>
) -> Result<Json<HashMap<String, f64>>, StatusCode> {
    state.metrics.get_metrics().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn health_handler(
    State(state): State<AppState>
) -> Result<Json<HashMap<String, String>>, StatusCode> {
    state.health.check_health().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn system_handler(
    State(state): State<AppState>
) -> Result<Json<SystemMetrics>, StatusCode> {
    state.system.get_metrics().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn alerts_handler(
    State(state): State<AppState>
) -> Result<Json<Vec<crate::alert::Alert>>, StatusCode> {
    state.alert.get_active_alerts().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn performance_handler(
    State(state): State<AppState>
) -> Result<Json<performance::PerformanceMetrics>, StatusCode> {
    state.performance.get_metrics().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Initialize the monitoring system
///
/// # Arguments
/// * `config` - Monitor configuration
///
/// # Returns
/// * `Result<MonitorService, MonitorError>`
#[instrument(level = "debug")]
pub async fn init(config: MonitorConfig) -> Result<MonitorService> {
    let start = std::time::Instant::now();
    debug!("🔧 Initializing monitoring system");

    let mut service = MonitorService::new(config).await?;
    service.start().await?;

    info!("✅ Monitoring system initialized in {:?}", start.elapsed());
    Ok(service)
}

/// Shutdown the monitoring system
///
/// # Arguments
/// * `service` - Monitor service instance
///
/// # Returns
/// * `Result<(), MonitorError>`
#[instrument(level = "debug")]
pub async fn shutdown(mut service: MonitorService) -> Result<(), MonitorError> {
    debug!("🔧 Shutting down monitoring system");
    service.stop().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;

    static TEST_MANAGER: OnceCell<Mutex<MonitorService>> = OnceCell::new();
    static METRICS_INITIALIZED: OnceCell<Arc<MetricsManager>> = OnceCell::new();

    async fn init_test_service() -> Result<()> {
        if TEST_MANAGER.get().is_none() {
            // Initialize metrics only once
            let metrics = METRICS_INITIALIZED.get_or_try_init(|| {
                let config = MonitorConfig::default();
                MetricsManager::new(config.metrics.clone())
                    .map(Arc::new)
            })?;

            let config = MonitorConfig::default();
            let mut service = MonitorService::new_with_metrics(&config, metrics.clone()).await?;
            service.start().await?;
            TEST_MANAGER.set(Mutex::new(service)).unwrap();
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_monitor_service() -> Result<()> {
        init_test_service().await?;
        let mut service = TEST_MANAGER.get().unwrap().lock().unwrap();
        assert!(service.stop().await.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_init_shutdown() -> Result<()> {
        init_test_service().await?;
        let mut service = TEST_MANAGER.get().unwrap().lock().unwrap();
        assert!(service.stop().await.is_ok());
        Ok(())
    }
}
