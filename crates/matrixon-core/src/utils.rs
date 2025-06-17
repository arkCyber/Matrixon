//! Utility functions for Matrixon
//! 
//! This module provides common utility functions used throughout the Matrixon system.
//! These utilities help with logging, timing, ID generation, and Matrix protocol
//! specific operations.

use std::time::Instant;
use tracing::{debug, info, instrument, Level};
use chrono::{DateTime, Utc};
use serde_json::Value;
use crate::Result;

/// Log the start of an operation and return the start time
#[instrument(level = "debug")]
pub fn log_operation_start(operation: &str) -> Instant {
    debug!("🔧 Starting operation: {}", operation);
    Instant::now()
}

/// Log the end of an operation with duration
#[instrument(level = "debug")]
pub fn log_operation_end(operation: &str, start: Instant) {
    info!("✅ Completed {} in {:?}", operation, start.elapsed());
}

/// Format a duration in a human-readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    if duration.as_secs() < 60 {
        format!("{}ms", duration.as_millis())
    } else if duration.as_secs() < 3600 {
        format!("{}s", duration.as_secs())
    } else {
        format!("{}h {}m", duration.as_secs() / 3600, (duration.as_secs() % 3600) / 60)
    }
}

/// Generate a unique identifier
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Convert a string to a valid Matrix ID
pub fn to_matrix_id(input: &str) -> String {
    input.to_lowercase().replace(" ", "_")
}

/// Validate a Matrix ID format
pub fn is_valid_matrix_id(id: &str) -> bool {
    id.starts_with('@') && id.contains(':')
}

/// Get the current timestamp
pub fn get_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Format a timestamp in ISO 8601 format
pub fn format_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339()
}

/// Parse a timestamp from ISO 8601 format
pub fn parse_timestamp(input: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| crate::MatrixonError::Validation(format!("Invalid timestamp: {}", e)))
}

/// Convert a JSON value to a pretty-printed string
pub fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

/// Convert a JSON value to a compact string
pub fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
}

/// Get the log level from a string
pub fn get_log_level(level: &str) -> Level {
    match level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

/// Format a log level as a string
pub fn format_log_level(level: Level) -> String {
    match level {
        Level::TRACE => "trace",
        Level::DEBUG => "debug",
        Level::INFO => "info",
        Level::WARN => "warn",
        Level::ERROR => "error",
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use test_log::test;

    #[test]
    fn test_operation_logging() {
        let start = log_operation_start("test_operation");
        thread::sleep(Duration::from_millis(10));
        log_operation_end("test_operation", start);
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(3660)), "1h 1m");
    }

    #[test]
    fn test_id_generation() {
        let id1 = generate_id();
        let id2 = generate_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 36);
        assert_eq!(id2.len(), 36);
    }

    #[test]
    fn test_matrix_id_conversion() {
        let input = "Test User";
        let expected = "test_user";
        assert_eq!(to_matrix_id(input), expected);
    }

    #[test]
    fn test_matrix_id_validation() {
        assert!(is_valid_matrix_id("@user:example.com"));
        assert!(!is_valid_matrix_id("invalid"));
        assert!(!is_valid_matrix_id("@invalid"));
    }

    #[test]
    fn test_timestamp_operations() {
        let now = get_timestamp();
        let formatted = format_timestamp(now);
        let parsed = parse_timestamp(&formatted).unwrap();
        assert_eq!(now, parsed);
    }

    #[test]
    fn test_json_operations() {
        let value = serde_json::json!({
            "name": "test",
            "value": 123
        });
        let pretty = pretty_json(&value);
        let compact = compact_json(&value);
        assert!(pretty.contains("\n"));
        assert!(!compact.contains("\n"));
    }

    #[test]
    fn test_log_level_operations() {
        assert_eq!(get_log_level("debug"), Level::DEBUG);
        assert_eq!(get_log_level("invalid"), Level::INFO);
        assert_eq!(format_log_level(Level::DEBUG), "debug");
    }
} 
