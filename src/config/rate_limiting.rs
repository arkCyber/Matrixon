// =============================================================================
// Matrixon Matrix NextServer - Rate Limiting Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Configuration management and validation. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • Configuration parsing and validation
//   • Environment variable handling
//   • Default value management
//   • Type-safe configuration
//   • Runtime configuration updates
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    str::FromStr,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::{Config, Error, Result};

/// Main rate limiting configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
    /// Whether rate limiting is globally enabled
    pub enabled: bool,

    /// Message sending rate limits
    pub rc_message: RateLimitConfig,

    /// Registration rate limits (per IP)
    pub rc_registration: RateLimitConfig,

    /// Login rate limits
    pub rc_login: LoginRateLimitConfig,

    /// Room join rate limits  
    pub rc_joins: JoinRateLimitConfig,

    /// Invite rate limits
    pub rc_invites: InviteRateLimitConfig,

    /// Admin redaction rate limits
    pub rc_admin_redaction: RateLimitConfig,

    /// 3PID validation rate limits
    pub rc_3pid_validation: RateLimitConfig,

    /// Media creation rate limits
    pub rc_media_create: RateLimitConfig,

    /// Federation rate limits
    pub rc_federation: FederationRateLimitConfig,

    /// Rate limiting exemptions
    pub exemptions: ExemptionConfig,

    /// Advanced configuration options
    pub advanced: AdvancedRateLimitConfig,
}

/// Individual rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub per_second: f64,
    
    /// Burst capacity (immediate requests allowed)
    pub burst_count: u32,
    
    /// Whether this specific limit is enabled
    pub enabled: bool,

    /// Optional minimum interval between requests (overrides per_second for minimum delay)
    pub min_interval_ms: Option<u64>,

    /// Optional maximum penalty duration for violations
    pub max_penalty_duration_ms: Option<u64>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_second: 1.0,
            burst_count: 10,
            enabled: true,
            min_interval_ms: None,
            max_penalty_duration_ms: None,
        }
    }
}

/// Login-specific rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRateLimitConfig {
    /// Per-IP address login attempts
    pub address: RateLimitConfig,
    
    /// Per-account login attempts  
    pub account: RateLimitConfig,
    
    /// Failed login attempts (with progressive penalties)
    pub failed_attempts: RateLimitConfig,

    /// Lockout configuration for repeated failures
    pub lockout: LockoutConfig,
}

/// Account lockout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutConfig {
    /// Whether account lockout is enabled
    pub enabled: bool,
    
    /// Number of failed attempts before lockout
    pub max_attempts: u32,
    
    /// Initial lockout duration in seconds
    pub initial_duration_seconds: u64,
    
    /// Maximum lockout duration in seconds
    pub max_duration_seconds: u64,
    
    /// Multiplier for progressive lockout durations
    pub duration_multiplier: f64,
}

impl Default for LockoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
            initial_duration_seconds: 300, // 5 minutes
            max_duration_seconds: 86400,   // 24 hours
            duration_multiplier: 2.0,
        }
    }
}

/// Room join rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRateLimitConfig {
    /// Local room joins (rooms on this server)
    pub local: RateLimitConfig,
    
    /// Remote room joins (rooms on other servers)
    pub remote: RateLimitConfig,
    
    /// Per-room join limits (to prevent mass join attacks)
    pub per_room: RateLimitConfig,

    /// Special limits for large rooms
    pub large_room_threshold: u32,
    pub large_room_limits: RateLimitConfig,
}

/// Invite rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteRateLimitConfig {
    /// Per-room invite limits
    pub per_room: RateLimitConfig,
    
    /// Per-user invite limits (for the recipient)
    pub per_user: RateLimitConfig,
    
    /// Per-issuer invite limits (for the sender)
    pub per_issuer: RateLimitConfig,

    /// Bulk invite limits (creating multiple invites at once)
    pub bulk_invite: BulkInviteConfig,
}

/// Bulk invite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkInviteConfig {
    /// Whether bulk invites are allowed
    pub enabled: bool,
    
    /// Maximum invites per bulk operation
    pub max_invites_per_request: u32,
    
    /// Rate limits for bulk invite operations
    pub rate_limit: RateLimitConfig,
}

impl Default for BulkInviteConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_invites_per_request: 50,
            rate_limit: RateLimitConfig {
                per_second: 0.1,
                burst_count: 3,
                enabled: true,
                min_interval_ms: None,
                max_penalty_duration_ms: Some(3600000), // 1 hour
            },
        }
    }
}

/// Federation rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRateLimitConfig {
    /// Window size in milliseconds for rate calculation
    pub window_size_ms: u64,
    
    /// Number of requests before applying sleep delay
    pub sleep_limit: u32,
    
    /// Sleep delay in milliseconds
    pub sleep_delay_ms: u64,
    
    /// Hard rejection limit
    pub reject_limit: u32,
    
    /// Maximum concurrent requests per server
    pub concurrent_limit: u32,

    /// Per-server custom limits
    pub per_server_limits: HashMap<String, FederationServerLimits>,

    /// Whether to apply stricter limits to unknown servers
    pub strict_unknown_servers: bool,
}

/// Per-server federation limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationServerLimits {
    /// Custom concurrent limit for this server
    pub concurrent_limit: Option<u32>,
    
    /// Custom reject limit for this server
    pub reject_limit: Option<u32>,
    
    /// Whether this server is trusted (reduced limits)
    pub trusted: bool,
}

/// Rate limiting exemption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExemptionConfig {
    /// User IDs exempt from rate limiting
    pub exempt_users: HashSet<String>,
    
    /// IP addresses exempt from rate limiting
    pub exempt_ips: HashSet<IpAddr>,
    
    /// IP CIDR ranges exempt from rate limiting
    pub exempt_ip_ranges: Vec<String>,
    
    /// Server names exempt from federation rate limiting
    pub exempt_servers: HashSet<String>,
    
    /// Whether application services are exempt
    pub exempt_appservices: bool,

    /// Whether room admins get higher limits
    pub admin_multiplier: f64,

    /// Whether server admins are completely exempt
    pub exempt_server_admins: bool,
}

impl Default for ExemptionConfig {
    fn default() -> Self {
        Self {
            exempt_users: HashSet::new(),
            exempt_ips: HashSet::new(),
            exempt_ip_ranges: Vec::new(),
            exempt_servers: HashSet::new(),
            exempt_appservices: false,
            admin_multiplier: 5.0,
            exempt_server_admins: true,
        }
    }
}

/// Advanced rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRateLimitConfig {
    /// Whether to enable adaptive rate limiting based on server load
    pub adaptive_limiting: bool,
    
    /// Server load threshold for adaptive limiting (0.0 - 1.0)
    pub load_threshold: f64,
    
    /// Multiplier to apply when server is under high load
    pub load_penalty_multiplier: f64,

    /// Whether to persist rate limiting state across restarts
    pub persist_state: bool,
    
    /// How often to clean up expired rate limiting state (seconds)
    pub cleanup_interval_seconds: u64,
    
    /// How long to keep rate limiting state after last activity (seconds)
    pub state_expiry_seconds: u64,

    /// Whether to log rate limiting violations
    pub log_violations: bool,
    
    /// Maximum number of violation logs to keep
    pub max_violation_logs: u32,

    /// Whether to enable rate limiting metrics
    pub enable_metrics: bool,

    /// Custom rate limiting algorithms
    pub algorithms: AlgorithmConfig,
}

impl Default for AdvancedRateLimitConfig {
    fn default() -> Self {
        Self {
            adaptive_limiting: false,
            load_threshold: 0.8,
            load_penalty_multiplier: 0.5,
            persist_state: true,
            cleanup_interval_seconds: 3600, // 1 hour
            state_expiry_seconds: 86400,    // 24 hours
            log_violations: true,
            max_violation_logs: 10000,
            enable_metrics: true,
            algorithms: AlgorithmConfig::default(),
        }
    }
}

/// Rate limiting algorithm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// Primary algorithm: TokenBucket, FixedWindow, SlidingWindow
    pub primary_algorithm: RateLimitAlgorithm,
    
    /// Whether to use secondary algorithm for burst detection
    pub enable_burst_detection: bool,
    
    /// Secondary algorithm for burst detection
    pub burst_algorithm: RateLimitAlgorithm,

    /// Custom algorithm parameters
    pub custom_parameters: HashMap<String, f64>,
}

impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            primary_algorithm: RateLimitAlgorithm::TokenBucket,
            enable_burst_detection: false,
            burst_algorithm: RateLimitAlgorithm::SlidingWindow,
            custom_parameters: HashMap::new(),
        }
    }
}

/// Available rate limiting algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimitAlgorithm {
    /// Token bucket algorithm (default)
    TokenBucket,
    /// Fixed window algorithm
    FixedWindow,
    /// Sliding window algorithm
    SlidingWindow,
    /// Leaky bucket algorithm
    LeakyBucket,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rc_message: RateLimitConfig {
                per_second: 0.2,
                burst_count: 10,
                enabled: true,
                min_interval_ms: None,
                max_penalty_duration_ms: Some(60000), // 1 minute
            },
            rc_registration: RateLimitConfig {
                per_second: 0.17,
                burst_count: 3,
                enabled: true,
                min_interval_ms: Some(5000), // Minimum 5 seconds between registrations
                max_penalty_duration_ms: Some(3600000), // 1 hour max penalty
            },
            rc_login: LoginRateLimitConfig {
                address: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(1800000), // 30 minutes
                },
                account: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(1800000),
                },
                failed_attempts: RateLimitConfig {
                    per_second: 0.17,
                    burst_count: 3,
                    enabled: true,
                    min_interval_ms: Some(1000),
                    max_penalty_duration_ms: Some(86400000), // 24 hours
                },
                lockout: LockoutConfig::default(),
            },
            rc_joins: JoinRateLimitConfig {
                local: RateLimitConfig {
                    per_second: 0.1,
                    burst_count: 10,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(300000), // 5 minutes
                },
                remote: RateLimitConfig {
                    per_second: 0.01,
                    burst_count: 10,
                    enabled: true,
                    min_interval_ms: Some(10000), // 10 seconds for remote joins
                    max_penalty_duration_ms: Some(1800000), // 30 minutes
                },
                per_room: RateLimitConfig {
                    per_second: 1.0,
                    burst_count: 10,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(600000), // 10 minutes
                },
                large_room_threshold: 1000,
                large_room_limits: RateLimitConfig {
                    per_second: 0.5,
                    burst_count: 5,
                    enabled: true,
                    min_interval_ms: Some(2000),
                    max_penalty_duration_ms: Some(1800000),
                },
            },
            rc_invites: InviteRateLimitConfig {
                per_room: RateLimitConfig {
                    per_second: 0.3,
                    burst_count: 10,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(600000),
                },
                per_user: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(1800000),
                },
                per_issuer: RateLimitConfig {
                    per_second: 0.5,
                    burst_count: 5,
                    enabled: true,
                    min_interval_ms: None,
                    max_penalty_duration_ms: Some(3600000),
                },
                bulk_invite: BulkInviteConfig::default(),
            },
            rc_admin_redaction: RateLimitConfig {
                per_second: 1.0,
                burst_count: 50,
                enabled: true,
                min_interval_ms: None,
                max_penalty_duration_ms: Some(300000),
            },
            rc_3pid_validation: RateLimitConfig {
                per_second: 0.003,
                burst_count: 5,
                enabled: true,
                min_interval_ms: Some(30000), // 30 seconds minimum
                max_penalty_duration_ms: Some(3600000),
            },
            rc_media_create: RateLimitConfig {
                per_second: 10.0,
                burst_count: 50,
                enabled: true,
                min_interval_ms: None,
                max_penalty_duration_ms: Some(1800000),
            },
            rc_federation: FederationRateLimitConfig {
                window_size_ms: 1000,
                sleep_limit: 10,
                sleep_delay_ms: 500,
                reject_limit: 50,
                concurrent_limit: 3,
                per_server_limits: HashMap::new(),
                strict_unknown_servers: true,
            },
            exemptions: ExemptionConfig::default(),
            advanced: AdvancedRateLimitConfig::default(),
        }
    }
}

impl RateLimitingConfig {
    /// Validate the rate limiting configuration
    pub fn validate(&self) -> Result<()> {
        info!("🔍 Validating rate limiting configuration");

        // Validate individual rate limit configs
        self.validate_rate_limit_config(&self.rc_message, "rc_message")?;
        self.validate_rate_limit_config(&self.rc_registration, "rc_registration")?;
        self.validate_rate_limit_config(&self.rc_login.address, "rc_login.address")?;
        self.validate_rate_limit_config(&self.rc_login.account, "rc_login.account")?;
        self.validate_rate_limit_config(&self.rc_login.failed_attempts, "rc_login.failed_attempts")?;
        
        // Validate join limits
        self.validate_rate_limit_config(&self.rc_joins.local, "rc_joins.local")?;
        self.validate_rate_limit_config(&self.rc_joins.remote, "rc_joins.remote")?;
        self.validate_rate_limit_config(&self.rc_joins.per_room, "rc_joins.per_room")?;

        // Validate invite limits
        self.validate_rate_limit_config(&self.rc_invites.per_room, "rc_invites.per_room")?;
        self.validate_rate_limit_config(&self.rc_invites.per_user, "rc_invites.per_user")?;
        self.validate_rate_limit_config(&self.rc_invites.per_issuer, "rc_invites.per_issuer")?;

        // Validate other limits
        self.validate_rate_limit_config(&self.rc_admin_redaction, "rc_admin_redaction")?;
        self.validate_rate_limit_config(&self.rc_3pid_validation, "rc_3pid_validation")?;
        self.validate_rate_limit_config(&self.rc_media_create, "rc_media_create")?;

        // Validate federation config
        self.validate_federation_config()?;

        // Validate exemptions
        self.validate_exemptions()?;

        // Validate advanced config
        self.validate_advanced_config()?;

        info!("✅ Rate limiting configuration validation passed");
        Ok(())
    }

    fn validate_rate_limit_config(&self, config: &RateLimitConfig, name: &str) -> Result<()> {
        if config.per_second < 0.0 {
            return Err(Error::bad_config(&format!(
                "{}: per_second must be non-negative, got {}", name, config.per_second
            )));
        }

        if config.per_second > 1000.0 {
            warn!("⚠️ {}: Very high per_second rate: {}", name, config.per_second);
        }

        if config.burst_count == 0 {
            return Err(Error::bad_config(&format!(
                "{}: burst_count must be greater than 0, got {}", name, config.burst_count
            )));
        }

        if config.burst_count > 10000 {
            warn!("⚠️ {}: Very high burst_count: {}", name, config.burst_count);
        }

        if let Some(min_interval) = config.min_interval_ms {
            if min_interval > 3600000 { // 1 hour
                warn!("⚠️ {}: Very long min_interval_ms: {} ms", name, min_interval);
            }
        }

        if let Some(max_penalty) = config.max_penalty_duration_ms {
            if max_penalty > 86400000 { // 24 hours
                warn!("⚠️ {}: Very long max_penalty_duration_ms: {} ms", name, max_penalty);
            }
        }

        debug!("✅ {} configuration is valid", name);
        Ok(())
    }

    fn validate_federation_config(&self) -> Result<()> {
        let federation = &self.rc_federation;

        if federation.window_size_ms == 0 {
            return Err(Error::bad_config("rc_federation.window_size_ms must be greater than 0"));
        }

        if federation.sleep_limit == 0 {
            return Err(Error::bad_config("rc_federation.sleep_limit must be greater than 0"));
        }

        if federation.reject_limit == 0 {
            return Err(Error::bad_config("rc_federation.reject_limit must be greater than 0"));
        }

        if federation.concurrent_limit == 0 {
            return Err(Error::bad_config("rc_federation.concurrent_limit must be greater than 0"));
        }

        if federation.sleep_limit >= federation.reject_limit {
            warn!("⚠️ rc_federation.sleep_limit ({}) should be less than reject_limit ({})", 
                  federation.sleep_limit, federation.reject_limit);
        }

        debug!("✅ Federation rate limiting configuration is valid");
        Ok(())
    }

    fn validate_exemptions(&self) -> Result<()> {
        // Validate IP exemptions
        for ip_str in &self.exemptions.exempt_ip_ranges {
            if let Err(e) = ipnet::IpNet::from_str(ip_str) {
                return Err(Error::bad_config(&format!(
                    "Invalid IP range in exemptions: {}: {}", ip_str, e
                )));
            }
        }

        if self.exemptions.admin_multiplier <= 0.0 {
            return Err(Error::bad_config("exemptions.admin_multiplier must be positive"));
        }

        if self.exemptions.admin_multiplier > 100.0 {
            warn!("⚠️ Very high admin_multiplier: {}", self.exemptions.admin_multiplier);
        }

        debug!("✅ Rate limiting exemptions configuration is valid");
        Ok(())
    }

    fn validate_advanced_config(&self) -> Result<()> {
        let advanced = &self.advanced;

        if advanced.load_threshold < 0.0 || advanced.load_threshold > 1.0 {
            return Err(Error::bad_config(
                "advanced.load_threshold must be between 0.0 and 1.0"
            ));
        }

        if advanced.load_penalty_multiplier <= 0.0 {
            return Err(Error::bad_config(
                "advanced.load_penalty_multiplier must be positive"
            ));
        }

        if advanced.cleanup_interval_seconds == 0 {
            return Err(Error::bad_config(
                "advanced.cleanup_interval_seconds must be greater than 0"
            ));
        }

        if advanced.state_expiry_seconds == 0 {
            return Err(Error::bad_config(
                "advanced.state_expiry_seconds must be greater than 0"
            ));
        }

        debug!("✅ Advanced rate limiting configuration is valid");
        Ok(())
    }

    /// Get effective rate limit for a user (applying multipliers)
    pub fn get_effective_user_limit(&self, base_config: &RateLimitConfig, is_admin: bool) -> RateLimitConfig {
        if is_admin && self.exemptions.admin_multiplier > 1.0 {
            RateLimitConfig {
                per_second: base_config.per_second * self.exemptions.admin_multiplier,
                burst_count: ((base_config.burst_count as f64) * self.exemptions.admin_multiplier) as u32,
                enabled: base_config.enabled,
                min_interval_ms: base_config.min_interval_ms.map(|interval| {
                    (interval as f64 / self.exemptions.admin_multiplier) as u64
                }),
                max_penalty_duration_ms: base_config.max_penalty_duration_ms,
            }
        } else {
            base_config.clone()
        }
    }

    /// Check if IP is exempt from rate limiting
    pub fn is_ip_exempt(&self, ip: &IpAddr) -> bool {
        if self.exemptions.exempt_ips.contains(ip) {
            return true;
        }

        // Check IP ranges
        for range_str in &self.exemptions.exempt_ip_ranges {
            if let Ok(range) = ipnet::IpNet::from_str(range_str) {
                if range.contains(ip) {
                    return true;
                }
            }
        }

        false
    }

    /// Apply server load adjustment to rate limits
    pub fn apply_load_adjustment(&self, base_config: &RateLimitConfig, current_load: f64) -> RateLimitConfig {
        if !self.advanced.adaptive_limiting || current_load <= self.advanced.load_threshold {
            return base_config.clone();
        }

        let penalty = self.advanced.load_penalty_multiplier;
        RateLimitConfig {
            per_second: base_config.per_second * penalty,
            burst_count: ((base_config.burst_count as f64) * penalty) as u32,
            enabled: base_config.enabled,
            min_interval_ms: base_config.min_interval_ms.map(|interval| {
                (interval as f64 / penalty) as u64
            }),
            max_penalty_duration_ms: base_config.max_penalty_duration_ms,
        }
    }

    /// Export configuration as TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| Error::bad_config(&format!("Failed to serialize config to TOML: {}", e)))
    }

    /// Import configuration from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .map_err(|e| Error::bad_config(&format!("Failed to parse TOML config: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_default_config_validation() {
        let config = RateLimitingConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_per_second() {
        let mut config = RateLimitingConfig::default();
        config.rc_message.per_second = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_burst_count() {
        let mut config = RateLimitingConfig::default();
        config.rc_message.burst_count = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_federation_config() {
        let mut config = RateLimitingConfig::default();
        config.rc_federation.window_size_ms = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_load_threshold() {
        let mut config = RateLimitingConfig::default();
        config.advanced.load_threshold = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_admin_multiplier() {
        let config = RateLimitingConfig::default();
        let base_limit = RateLimitConfig {
            per_second: 1.0,
            burst_count: 10,
            enabled: true,
            min_interval_ms: None,
            max_penalty_duration_ms: None,
        };

        let admin_limit = config.get_effective_user_limit(&base_limit, true);
        assert!(admin_limit.per_second > base_limit.per_second);
        assert!(admin_limit.burst_count > base_limit.burst_count);

        let user_limit = config.get_effective_user_limit(&base_limit, false);
        assert_eq!(user_limit.per_second, base_limit.per_second);
    }

    #[test]
    fn test_ip_exemption() {
        let mut config = RateLimitingConfig::default();
        let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        
        // Should not be exempt initially
        assert!(!config.is_ip_exempt(&test_ip));
        
        // Add to exemptions
        config.exemptions.exempt_ips.insert(test_ip);
        assert!(config.is_ip_exempt(&test_ip));
    }

    #[test]
    fn test_ip_range_exemption() {
        let mut config = RateLimitingConfig::default();
        config.exemptions.exempt_ip_ranges.push("192.168.1.0/24".to_string());
        
        let test_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        assert!(config.is_ip_exempt(&test_ip));
        
        let other_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert!(!config.is_ip_exempt(&other_ip));
    }

    #[test]
    fn test_load_adjustment() {
        let config = RateLimitingConfig::default();
        let base_limit = RateLimitConfig {
            per_second: 10.0,
            burst_count: 20,
            enabled: true,
            min_interval_ms: None,
            max_penalty_duration_ms: None,
        };

        // No adjustment under threshold
        let adjusted = config.apply_load_adjustment(&base_limit, 0.5);
        assert_eq!(adjusted.per_second, base_limit.per_second);

        // Adjustment over threshold (but adaptive limiting disabled by default)
        let adjusted = config.apply_load_adjustment(&base_limit, 0.9);
        assert_eq!(adjusted.per_second, base_limit.per_second);
    }

    #[test]
    fn test_adaptive_load_adjustment() {
        let mut config = RateLimitingConfig::default();
        config.advanced.adaptive_limiting = true;
        config.advanced.load_threshold = 0.8;
        config.advanced.load_penalty_multiplier = 0.5;

        let base_limit = RateLimitConfig {
            per_second: 10.0,
            burst_count: 20,
            enabled: true,
            min_interval_ms: None,
            max_penalty_duration_ms: None,
        };

        // Adjustment over threshold
        let adjusted = config.apply_load_adjustment(&base_limit, 0.9);
        assert!(adjusted.per_second < base_limit.per_second);
        assert!(adjusted.burst_count < base_limit.burst_count);
    }

    #[test]
    fn test_toml_serialization() {
        let config = RateLimitingConfig::default();
        let toml_str = config.to_toml().unwrap();
        assert!(!toml_str.is_empty());
        
        let parsed_config = RateLimitingConfig::from_toml(&toml_str).unwrap();
        assert_eq!(config.enabled, parsed_config.enabled);
        assert_eq!(config.rc_message.per_second, parsed_config.rc_message.per_second);
    }

    #[test]
    fn test_invalid_ip_range() {
        let mut config = RateLimitingConfig::default();
        config.exemptions.exempt_ip_ranges.push("invalid_range".to_string());
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_lockout_config() {
        let lockout = LockoutConfig::default();
        assert!(lockout.enabled);
        assert_eq!(lockout.max_attempts, 5);
        assert!(lockout.duration_multiplier > 1.0);
    }

    #[test]
    fn test_bulk_invite_config() {
        let bulk = BulkInviteConfig::default();
        assert!(bulk.enabled);
        assert!(bulk.max_invites_per_request > 0);
        assert!(bulk.rate_limit.enabled);
    }

    #[test]
    fn test_algorithm_config() {
        let algo = AlgorithmConfig::default();
        assert!(matches!(algo.primary_algorithm, RateLimitAlgorithm::TokenBucket));
        assert!(!algo.enable_burst_detection);
    }

    #[test]
    fn test_federation_server_limits() {
        let limits = FederationServerLimits {
            concurrent_limit: Some(10),
            reject_limit: Some(100),
            trusted: true,
        };
        
        assert_eq!(limits.concurrent_limit, Some(10));
        assert!(limits.trusted);
    }

    #[test]
    fn test_exemption_config_defaults() {
        let exemptions = ExemptionConfig::default();
        assert!(exemptions.exempt_users.is_empty());
        assert!(exemptions.exempt_ips.is_empty());
        assert!(!exemptions.exempt_appservices);
        assert!(exemptions.exempt_server_admins);
        assert!(exemptions.admin_multiplier > 1.0);
    }
} 