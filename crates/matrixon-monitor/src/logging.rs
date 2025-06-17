//! Logging System
//! 
//! This module provides structured logging functionality for the Matrixon server.
//! It includes console and file logging, log rotation, and configurable log levels.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! Date: 2024-03-21

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{
    debug, info, instrument, warn,
    Level,
    Subscriber,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

use crate::{MonitorError, config::LoggingConfig};

/// Logging manager for handling structured logging
///
/// The LoggingManager struct manages logging configuration, initialization, and shutdown.
///
/// # Example
/// ```rust
/// let config = LoggingConfig::default();
/// let manager = LoggingManager::new(config).await.unwrap();
/// manager.start().await.unwrap();
/// ```
pub struct LoggingManager {
    /// Configuration
    config: Arc<LoggingConfig>,
    /// Running status
    is_running: Arc<RwLock<bool>>,
    /// Log file appender (wrapped in NonBlocking for thread safety)
    file_appender: Option<tracing_appender::non_blocking::NonBlocking>,
}

impl LoggingManager {
    /// Initialize the logging manager
    ///
    /// Sets up structured logging with console and file layers.
    ///
    /// # Arguments
    /// * `config` - Logging configuration
    ///
    /// # Returns
    /// * `Result<Self, String>`
    #[instrument(level = "debug")]
    pub async fn new(config: LoggingConfig) -> Result<Self, String> {
        let start = std::time::Instant::now();
        debug!("🔧 Initializing logging manager");

        let file_appender = if config.enable_file_logging {
            let (non_blocking, _guard) = tracing_appender::non_blocking(RollingFileAppender::new(
                Rotation::DAILY,
                &config.log_directory,
                "matrixon.log",
            ));
            Some(non_blocking)
        } else {
            None
        };

        let manager = Self {
            config: Arc::new(config),
            is_running: Arc::new(RwLock::new(false)),
            file_appender,
        };

        info!("✅ Logging manager initialized in {:?}", start.elapsed());
        Ok(manager)
    }

    /// Start the logging manager
    ///
    /// Starts logging output to console and file as configured.
    ///
    /// # Returns
    /// * `Result<(), String>`
    #[instrument(level = "debug", skip(self))]
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.is_running.write().await;
        if *running {
            warn!("⚠️ Logging manager is already running");
            return Ok(());
        }
        *running = true;

        // Initialize logging
        self.init_logging().await.map_err(|e| e.to_string())?;

        info!("✅ Logging manager started");
        Ok(())
    }

    /// Stop the logging manager
    ///
    /// Stops logging output and releases resources.
    ///
    /// # Returns
    /// * `Result<(), String>`
    #[instrument(level = "debug", skip(self))]
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.is_running.write().await;
        if !*running {
            debug!("⚠️ Logging manager is not running");
            return Ok(());
        }
        *running = false;

        info!("✅ Logging manager stopped");
        Ok(())
    }

    /// Initialize logging system
    async fn init_logging(&self) -> Result<(), MonitorError> {
        let config = &self.config;

        // Parse log level
        let log_level = config.log_level.parse::<Level>()
            .map_err(|e| MonitorError::LoggingError(format!("Invalid log level: {}", e)))?;

        // Create console layer
        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_level(true)
            .with_ansi(true);

        // Create file layer if enabled
        let file_layer = self.file_appender.as_ref().map(|appender| fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_span_events(FmtSpan::CLOSE)
                .with_level(true)
                .with_ansi(false)
                .with_writer(appender.clone()));

        // Create filter
        let filter = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new(format!("matrixon={}", log_level)))
            .map_err(|e| MonitorError::LoggingError(format!("Failed to create log filter: {}", e)))?;

        // Create base subscriber with filter
        let subscriber = tracing_subscriber::registry()
            .with(filter);

        // Convert console layer to boxed trait object
        let boxed_console = Box::new(console_layer) as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>;
        
        // Add console layer
        let subscriber = subscriber.with(boxed_console);

        // Add file layer if enabled
        let subscriber = match file_layer {
            Some(file_layer) => {
                let boxed_file = Box::new(file_layer) as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>;
                subscriber.with(boxed_file)
            },
            None => subscriber.with(Box::new(tracing_subscriber::fmt::Layer::default()) as Box<dyn tracing_subscriber::Layer<_> + Send + Sync>)
        };

        // Set global subscriber
        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| MonitorError::LoggingError(format!("Failed to set global subscriber: {}", e)))?;

        Ok(())
    }
}

/// Log operation span
#[macro_export]
macro_rules! operation_span {
    ($name:expr) => {
        tracing::info_span!("operation", name = $name)
    };
}

/// Log operation start
#[macro_export]
macro_rules! operation_start {
    ($name:expr) => {
        tracing::info!("🔧 Starting operation: {}", $name)
    };
}

/// Log operation complete
#[macro_export]
macro_rules! operation_complete {
    ($name:expr, $duration:expr) => {
        tracing::info!("✅ Completed operation {} in {:?}", $name, $duration)
    };
}

/// Log operation error
#[macro_export]
macro_rules! operation_error {
    ($name:expr, $error:expr) => {
        tracing::error!("❌ Operation {} failed: {}", $name, $error)
    };
}

/// Log operation warning
#[macro_export]
macro_rules! operation_warning {
    ($name:expr, $message:expr) => {
        tracing::warn!("⚠️ Operation {} warning: {}", $name, $message)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_logging_manager() {
        let config = LoggingConfig {
            log_level: "debug".to_string(),
            enable_file_logging: false,
            log_directory: "logs".to_string(),
            max_file_size: 1024 * 1024 * 10, // 10MB
            max_files: 5,
            enable_json_format: false,
        };

        let manager = LoggingManager::new(config).await.unwrap();
        assert!(manager.start().await.is_ok());
        assert!(manager.stop().await.is_ok());
    }
}
