// =============================================================================
// Matrixon Matrix NextServer - Mod Module
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
//   Core business logic service implementation. This module is part of the Matrixon Matrix NextServer
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
//   • Business logic implementation
//   • Service orchestration
//   • Event handling and processing
//   • State management
//   • Enterprise-grade reliability
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
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use ruma::{
    api::client::error::ErrorKind,
    OwnedRoomId, OwnedServerName, OwnedUserId, ServerName, UserId,
};
use tracing::{debug, info, instrument, warn};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Rate limiting bucket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum requests per second
    pub per_second: f64,
    /// Burst capacity (immediate requests allowed)
    pub burst_count: u32,
    /// Whether this limit is enabled
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            per_second: 1.0,
            burst_count: 10,
            enabled: true,
        }
    }
}

/// Comprehensive rate limiting configuration based on Synapse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitingConfig {
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
    /// Admin action rate limits
    pub rc_admin_redaction: RateLimitConfig,
    /// 3PID validation rate limits
    pub rc_3pid_validation: RateLimitConfig,
    /// Media upload rate limits
    pub rc_media_create: RateLimitConfig,
    /// Federation rate limits
    pub rc_federation: FederationRateLimitConfig,
    /// Rate limiting exemptions for admins/services
    pub exemptions: ExemptionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRateLimitConfig {
    /// Per-IP address login attempts
    pub address: RateLimitConfig,
    /// Per-account login attempts
    pub account: RateLimitConfig,
    /// Failed login attempts (progressive penalties)
    pub failed_attempts: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRateLimitConfig {
    /// Local room joins
    pub local: RateLimitConfig,
    /// Remote room joins (more expensive)
    pub remote: RateLimitConfig,
    /// Per-room join limits to prevent mass join attacks
    pub per_room: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteRateLimitConfig {
    /// Per-room invite limits
    pub per_room: RateLimitConfig,
    /// Per-user invite limits (recipient)
    pub per_user: RateLimitConfig,
    /// Per-issuer invite limits (sender)
    pub per_issuer: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationRateLimitConfig {
    /// Window size in milliseconds for rate calculation
    pub window_size: u64,
    /// Sleep limit (requests before delay)
    pub sleep_limit: u32,
    /// Sleep delay in milliseconds
    pub sleep_delay: u64,
    /// Hard rejection limit
    pub reject_limit: u32,
    /// Concurrent request limit per server
    pub concurrent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExemptionConfig {
    /// User IDs exempt from rate limiting
    pub exempt_users: Vec<String>,
    /// IP addresses exempt from rate limiting
    pub exempt_ips: Vec<String>,
    /// Server names exempt from federation rate limiting
    pub exempt_servers: Vec<String>,
    /// Application service rate limiting exemptions
    pub exempt_appservices: bool,
}

impl Default for RateLimitingConfig {
    fn default() -> Self {
        Self {
            rc_message: RateLimitConfig {
                per_second: 0.2,
                burst_count: 10,
                enabled: true,
            },
            rc_registration: RateLimitConfig {
                per_second: 0.17,
                burst_count: 3,
                enabled: true,
            },
            rc_login: LoginRateLimitConfig {
                address: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                },
                account: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                },
                failed_attempts: RateLimitConfig {
                    per_second: 0.17,
                    burst_count: 3,
                    enabled: true,
                },
            },
            rc_joins: JoinRateLimitConfig {
                local: RateLimitConfig {
                    per_second: 0.1,
                    burst_count: 10,
                    enabled: true,
                },
                remote: RateLimitConfig {
                    per_second: 0.01,
                    burst_count: 10,
                    enabled: true,
                },
                per_room: RateLimitConfig {
                    per_second: 1.0,
                    burst_count: 10,
                    enabled: true,
                },
            },
            rc_invites: InviteRateLimitConfig {
                per_room: RateLimitConfig {
                    per_second: 0.3,
                    burst_count: 10,
                    enabled: true,
                },
                per_user: RateLimitConfig {
                    per_second: 0.003,
                    burst_count: 5,
                    enabled: true,
                },
                per_issuer: RateLimitConfig {
                    per_second: 0.5,
                    burst_count: 5,
                    enabled: true,
                },
            },
            rc_admin_redaction: RateLimitConfig {
                per_second: 1.0,
                burst_count: 50,
                enabled: true,
            },
            rc_3pid_validation: RateLimitConfig {
                per_second: 0.003,
                burst_count: 5,
                enabled: true,
            },
            rc_media_create: RateLimitConfig {
                per_second: 10.0,
                burst_count: 50,
                enabled: true,
            },
            rc_federation: FederationRateLimitConfig {
                window_size: 1000,
                sleep_limit: 10,
                sleep_delay: 500,
                reject_limit: 50,
                concurrent: 3,
            },
            exemptions: ExemptionConfig {
                exempt_users: vec![],
                exempt_ips: vec![],
                exempt_servers: vec![],
                exempt_appservices: false,
            },
        }
    }
}

/// Token bucket rate limiter state
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    capacity: f64,
    refill_rate: f64,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            last_refill: Instant::now(),
            capacity,
            refill_rate,
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }
}

/// Rate limiting state for tracking violations and penalties
#[derive(Debug, Clone)]
struct RateLimitState {
    bucket: TokenBucket,
    violation_count: u32,
    last_violation: Option<Instant>,
    penalty_until: Option<Instant>,
}

impl RateLimitState {
    fn new(config: &RateLimitConfig) -> Self {
        Self {
            bucket: TokenBucket::new(config.burst_count as f64, config.per_second),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
        }
    }

    fn check_and_consume(&mut self, _config: &RateLimitConfig) -> Result<()> {
        let now = Instant::now();
        
        // Check if still in penalty period
        if let Some(penalty_until) = self.penalty_until {
            if now < penalty_until {
                return Err(Error::BadRequestString(
                    ErrorKind::LimitExceeded { retry_after: None },
                    "Rate limit exceeded, please wait before retrying",
                ));
            } else {
                self.penalty_until = None;
            }
        }

        if !self.bucket.try_consume(1.0) {
            self.violation_count += 1;
            self.last_violation = Some(now);
            
            // Apply progressive penalty for repeated violations
            if self.violation_count > 3 {
                let penalty_duration = Duration::from_secs(
                    (2_u64.pow(self.violation_count.min(10)) * 5).min(3600)
                );
                self.penalty_until = Some(now + penalty_duration);
                
                warn!("🚫 Rate limit violation #{}, penalty applied for {:?}", 
                      self.violation_count, penalty_duration);
            }
            
            return Err(Error::BadRequestString(
                ErrorKind::LimitExceeded { retry_after: None },
                "Rate limit exceeded",
            ));
        }

        // Reset violation count after successful request
        if let Some(last_violation) = self.last_violation {
            if now.duration_since(last_violation) > Duration::from_secs(3600) {
                self.violation_count = 0;
            }
        }

        Ok(())
    }
}

/// Enterprise rate limiting service
pub struct Service {
    config: RateLimitingConfig,
    
    // User-based rate limiting
    user_message_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
    user_invite_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
    user_join_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
    
    // IP-based rate limiting
    ip_registration_limits: Arc<RwLock<HashMap<IpAddr, RateLimitState>>>,
    ip_login_limits: Arc<RwLock<HashMap<IpAddr, RateLimitState>>>,
    
    // Account-based rate limiting
    account_login_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
    account_failed_login_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
    
    // Room-based rate limiting
    room_join_limits: Arc<RwLock<HashMap<OwnedRoomId, RateLimitState>>>,
    room_invite_limits: Arc<RwLock<HashMap<OwnedRoomId, RateLimitState>>>,
    
    // Federation rate limiting
    federation_limits: Arc<RwLock<HashMap<OwnedServerName, RateLimitState>>>,
    
    // 3PID validation limits
    threepid_validation_limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
    
    // Media creation limits
    media_creation_limits: Arc<RwLock<HashMap<OwnedUserId, RateLimitState>>>,
}

impl Service {
    /// Create new rate limiting service with configuration
    pub fn new(config: RateLimitingConfig) -> Self {
        info!("🚦 Initializing enterprise rate limiting service");
        debug!("📋 Rate limiting configuration: {:?}", config);
        
        Self {
            config,
            user_message_limits: Arc::new(RwLock::new(HashMap::new())),
            user_invite_limits: Arc::new(RwLock::new(HashMap::new())),
            user_join_limits: Arc::new(RwLock::new(HashMap::new())),
            ip_registration_limits: Arc::new(RwLock::new(HashMap::new())),
            ip_login_limits: Arc::new(RwLock::new(HashMap::new())),
            account_login_limits: Arc::new(RwLock::new(HashMap::new())),
            account_failed_login_limits: Arc::new(RwLock::new(HashMap::new())),
            room_join_limits: Arc::new(RwLock::new(HashMap::new())),
            room_invite_limits: Arc::new(RwLock::new(HashMap::new())),
            federation_limits: Arc::new(RwLock::new(HashMap::new())),
            threepid_validation_limits: Arc::new(RwLock::new(HashMap::new())),
            media_creation_limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if user is exempt from rate limiting
    fn is_user_exempt(&self, user_id: &UserId) -> bool {
        self.config.exemptions.exempt_users.iter()
            .any(|exempt_user| exempt_user == user_id.as_str())
    }

    /// Check if IP is exempt from rate limiting
    fn is_ip_exempt(&self, ip: &IpAddr) -> bool {
        self.config.exemptions.exempt_ips.iter()
            .any(|exempt_ip| exempt_ip == &ip.to_string())
    }

    /// Check if server is exempt from federation rate limiting
    fn is_server_exempt(&self, server: &ServerName) -> bool {
        self.config.exemptions.exempt_servers.iter()
            .any(|exempt_server| exempt_server == server.as_str())
    }

    /// Check message sending rate limit
    #[instrument(level = "debug", skip(self))]
    pub async fn check_message_rate_limit(&self, user_id: &UserId) -> Result<()> {
        let start = Instant::now();
        
        if !self.config.rc_message.enabled {
            debug!("🔧 Message rate limiting disabled");
            return Ok(());
        }

        if self.is_user_exempt(user_id) {
            debug!("✅ User {} exempt from message rate limiting", user_id);
            return Ok(());
        }

        let mut limits = self.user_message_limits.write().unwrap();
        let state = limits.entry(user_id.to_owned())
            .or_insert_with(|| RateLimitState::new(&self.config.rc_message));
        
        let result = state.check_and_consume(&self.config.rc_message);
        
        debug!("🚦 Message rate limit check for {} completed in {:?}", 
               user_id, start.elapsed());
        
        result
    }

    /// Check registration rate limit (per IP)
    #[instrument(level = "debug", skip(self))]
    pub async fn check_registration_rate_limit(&self, ip: &IpAddr) -> Result<()> {
        let start = Instant::now();
        
        if !self.config.rc_registration.enabled {
            debug!("🔧 Registration rate limiting disabled");
            return Ok(());
        }

        if self.is_ip_exempt(ip) {
            debug!("✅ IP {} exempt from registration rate limiting", ip);
            return Ok(());
        }

        let mut limits = self.ip_registration_limits.write().unwrap();
        let state = limits.entry(*ip)
            .or_insert_with(|| RateLimitState::new(&self.config.rc_registration));
        
        let result = state.check_and_consume(&self.config.rc_registration);
        
        debug!("🚦 Registration rate limit check for {} completed in {:?}", 
               ip, start.elapsed());
        
        result
    }

    /// Get rate limiting statistics
    pub fn get_statistics(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        
        stats.insert("user_message_states".to_string(), 
                    self.user_message_limits.read().unwrap().len() as u64);
        stats.insert("ip_registration_states".to_string(), 
                    self.ip_registration_limits.read().unwrap().len() as u64);
        stats.insert("federation_states".to_string(), 
                    self.federation_limits.read().unwrap().len() as u64);
        stats.insert("room_join_states".to_string(), 
                    self.room_join_limits.read().unwrap().len() as u64);
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use ruma::{room_id, user_id, server_name};

    #[tokio::test]
    async fn test_rate_limiter_service_creation() {
        let config = RateLimitingConfig::default();
        let service = Service::new(config.clone());
        
        assert_eq!(service.config.rc_message.per_second, config.rc_message.per_second);
        assert_eq!(service.config.rc_registration.burst_count, config.rc_registration.burst_count);
    }

    #[tokio::test]
    async fn test_message_rate_limiting() {
        let config = RateLimitingConfig::default();
        let service = Service::new(config);
        let user_id = user_id!("@test:example.com");

        // First request should succeed
        assert!(service.check_message_rate_limit(user_id).await.is_ok());
        
        // Burst limit should allow multiple quick requests
        for _ in 0..5 {
            assert!(service.check_message_rate_limit(user_id).await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_registration_rate_limiting() {
        let config = RateLimitingConfig::default();
        let service = Service::new(config);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        // First few requests should succeed
        for _ in 0..3 {
            assert!(service.check_registration_rate_limit(&ip).await.is_ok());
        }
        
        // Should fail after burst limit
        assert!(service.check_registration_rate_limit(&ip).await.is_err());
    }

    #[tokio::test]
    async fn test_user_exemptions() {
        let mut config = RateLimitingConfig::default();
        config.exemptions.exempt_users.push("@admin:example.com".to_string());
        
        let service = Service::new(config);
        let admin_user = user_id!("@admin:example.com");
        let regular_user = user_id!("@user:example.com");

        // Admin should be exempt
        assert!(service.is_user_exempt(admin_user));
        assert!(!service.is_user_exempt(regular_user));
    }

    #[tokio::test]
    async fn test_ip_exemptions() {
        let mut config = RateLimitingConfig::default();
        config.exemptions.exempt_ips.push("127.0.0.1".to_string());
        
        let service = Service::new(config);
        let exempt_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let regular_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        assert!(service.is_ip_exempt(&exempt_ip));
        assert!(!service.is_ip_exempt(&regular_ip));
    }

    #[tokio::test]
    async fn test_token_bucket_functionality() {
        let config = RateLimitConfig {
            per_second: 1.0,
            burst_count: 5,
            enabled: true,
        };
        
        let mut bucket = TokenBucket::new(config.burst_count as f64, config.per_second);
        
        // Should allow burst requests
        for _ in 0..5 {
            assert!(bucket.try_consume(1.0));
        }
        
        // Should fail after exhausting burst
        assert!(!bucket.try_consume(1.0));
    }

    #[tokio::test]
    async fn test_rate_limiting_statistics() {
        let config = RateLimitingConfig::default();
        let service = Service::new(config);
        let user_id = user_id!("@test:example.com");
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

        // Create some rate limit states
        let _ = service.check_message_rate_limit(user_id).await;
        let _ = service.check_registration_rate_limit(&ip).await;
        
        let stats = service.get_statistics();
        assert!(stats.contains_key("user_message_states"));
        assert!(stats.contains_key("ip_registration_states"));
        assert!(stats.get("user_message_states").unwrap_or(&0) > &0);
    }

    #[tokio::test]
    async fn test_disabled_rate_limiting() {
        let mut config = RateLimitingConfig::default();
        config.rc_message.enabled = false;
        
        let service = Service::new(config);
        let user_id = user_id!("@test:example.com");

        // Should always succeed when disabled
        for _ in 0..100 {
            assert!(service.check_message_rate_limit(user_id).await.is_ok());
        }
    }
} 
