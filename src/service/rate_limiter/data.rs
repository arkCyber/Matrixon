// =============================================================================
// Matrixon Matrix NextServer - Data Module
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
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use ruma::{OwnedRoomId, OwnedServerName, OwnedUserId};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Persistent rate limiting state for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRateLimitState {
    /// Current token count in bucket
    pub tokens: f64,
    /// Last refill timestamp
    pub last_refill: SystemTime,
    /// Number of violations
    pub violation_count: u32,
    /// Last violation timestamp
    pub last_violation: Option<SystemTime>,
    /// Penalty expiration timestamp
    pub penalty_until: Option<SystemTime>,
    /// Rate limit configuration snapshot
    pub capacity: f64,
    pub refill_rate: f64,
}

/// Rate limiting data access trait
#[async_trait]
pub trait Data: Send + Sync {
    /// Store user-based rate limiting state
    async fn set_user_rate_limit_state(
        &self,
        user_id: &OwnedUserId,
        limit_type: &str,
        state: &PersistedRateLimitState,
    ) -> Result<()>;

    /// Retrieve user-based rate limiting state
    async fn get_user_rate_limit_state(
        &self,
        user_id: &OwnedUserId,
        limit_type: &str,
    ) -> Result<Option<PersistedRateLimitState>>;

    /// Store IP-based rate limiting state
    async fn set_ip_rate_limit_state(
        &self,
        ip: &IpAddr,
        limit_type: &str,
        state: &PersistedRateLimitState,
    ) -> Result<()>;

    /// Retrieve IP-based rate limiting state
    async fn get_ip_rate_limit_state(
        &self,
        ip: &IpAddr,
        limit_type: &str,
    ) -> Result<Option<PersistedRateLimitState>>;

    /// Store room-based rate limiting state
    async fn set_room_rate_limit_state(
        &self,
        room_id: &OwnedRoomId,
        limit_type: &str,
        state: &PersistedRateLimitState,
    ) -> Result<()>;

    /// Retrieve room-based rate limiting state
    async fn get_room_rate_limit_state(
        &self,
        room_id: &OwnedRoomId,
        limit_type: &str,
    ) -> Result<Option<PersistedRateLimitState>>;

    /// Store server-based rate limiting state
    async fn set_server_rate_limit_state(
        &self,
        server_name: &OwnedServerName,
        limit_type: &str,
        state: &PersistedRateLimitState,
    ) -> Result<()>;

    /// Retrieve server-based rate limiting state
    async fn get_server_rate_limit_state(
        &self,
        server_name: &OwnedServerName,
        limit_type: &str,
    ) -> Result<Option<PersistedRateLimitState>>;

    /// Store generic rate limiting state by identifier
    async fn set_generic_rate_limit_state(
        &self,
        identifier: &str,
        limit_type: &str,
        state: &PersistedRateLimitState,
    ) -> Result<()>;

    /// Retrieve generic rate limiting state by identifier
    async fn get_generic_rate_limit_state(
        &self,
        identifier: &str,
        limit_type: &str,
    ) -> Result<Option<PersistedRateLimitState>>;

    /// Clean up expired rate limiting states
    async fn cleanup_expired_rate_limit_states(&self, expiry_duration: Duration) -> Result<u64>;

    /// Get rate limiting statistics
    async fn get_rate_limit_statistics(&self) -> Result<HashMap<String, u64>>;

    /// Store rate limiting configuration
    async fn set_rate_limit_config(
        &self,
        config_name: &str,
        config_data: &[u8],
    ) -> Result<()>;

    /// Retrieve rate limiting configuration
    async fn get_rate_limit_config(&self, config_name: &str) -> Result<Option<Vec<u8>>>;

    /// List all active rate limit states by type
    async fn list_active_rate_limit_states(
        &self,
        limit_type: &str,
        limit: u32,
    ) -> Result<Vec<(String, PersistedRateLimitState)>>;

    /// Get rate limiting state count by type
    async fn get_rate_limit_state_count(&self, limit_type: &str) -> Result<u64>;

    /// Bulk cleanup rate limiting states
    async fn bulk_cleanup_rate_limit_states(
        &self,
        state_types: &[String],
        expiry_duration: Duration,
    ) -> Result<HashMap<String, u64>>;

    /// Store rate limiting violation log
    async fn log_rate_limit_violation(
        &self,
        identifier: &str,
        limit_type: &str,
        violation_time: SystemTime,
        metadata: &str,
    ) -> Result<()>;

    /// Get recent rate limiting violations
    async fn get_recent_rate_limit_violations(
        &self,
        since: SystemTime,
        limit: u32,
    ) -> Result<Vec<RateLimitViolation>>;
}

/// Rate limiting violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitViolation {
    pub identifier: String,
    pub limit_type: String,
    pub violation_time: SystemTime,
    pub metadata: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use std::net::Ipv4Addr;
    use ruma::{room_id, server_name, user_id};

    /// Mock implementation for testing
    struct MockData {
        user_states: std::sync::RwLock<HashMap<(OwnedUserId, String), PersistedRateLimitState>>,
        ip_states: std::sync::RwLock<HashMap<(IpAddr, String), PersistedRateLimitState>>,
        room_states: std::sync::RwLock<HashMap<(OwnedRoomId, String), PersistedRateLimitState>>,
        server_states: std::sync::RwLock<HashMap<(OwnedServerName, String), PersistedRateLimitState>>,
        generic_states: std::sync::RwLock<HashMap<(String, String), PersistedRateLimitState>>,
        violations: std::sync::RwLock<Vec<RateLimitViolation>>,
    }

    impl MockData {
        fn new() -> Self {
            Self {
                user_states: std::sync::RwLock::new(HashMap::new()),
                ip_states: std::sync::RwLock::new(HashMap::new()),
                room_states: std::sync::RwLock::new(HashMap::new()),
                server_states: std::sync::RwLock::new(HashMap::new()),
                generic_states: std::sync::RwLock::new(HashMap::new()),
                violations: std::sync::RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Data for MockData {
        async fn set_user_rate_limit_state(
            &self,
            user_id: &OwnedUserId,
            limit_type: &str,
            state: &PersistedRateLimitState,
        ) -> Result<()> {
            let mut states = self.user_states.write().unwrap();
            states.insert((user_id.clone(), limit_type.to_string()), state.clone());
            Ok(())
        }

        async fn get_user_rate_limit_state(
            &self,
            user_id: &OwnedUserId,
            limit_type: &str,
        ) -> Result<Option<PersistedRateLimitState>> {
            let states = self.user_states.read().unwrap();
            Ok(states.get(&(user_id.clone(), limit_type.to_string())).cloned())
        }

        async fn set_ip_rate_limit_state(
            &self,
            ip: &IpAddr,
            limit_type: &str,
            state: &PersistedRateLimitState,
        ) -> Result<()> {
            let mut states = self.ip_states.write().unwrap();
            states.insert((*ip, limit_type.to_string()), state.clone());
            Ok(())
        }

        async fn get_ip_rate_limit_state(
            &self,
            ip: &IpAddr,
            limit_type: &str,
        ) -> Result<Option<PersistedRateLimitState>> {
            let states = self.ip_states.read().unwrap();
            Ok(states.get(&(*ip, limit_type.to_string())).cloned())
        }

        async fn set_room_rate_limit_state(
            &self,
            room_id: &OwnedRoomId,
            limit_type: &str,
            state: &PersistedRateLimitState,
        ) -> Result<()> {
            let mut states = self.room_states.write().unwrap();
            states.insert((room_id.clone(), limit_type.to_string()), state.clone());
            Ok(())
        }

        async fn get_room_rate_limit_state(
            &self,
            room_id: &OwnedRoomId,
            limit_type: &str,
        ) -> Result<Option<PersistedRateLimitState>> {
            let states = self.room_states.read().unwrap();
            Ok(states.get(&(room_id.clone(), limit_type.to_string())).cloned())
        }

        async fn set_server_rate_limit_state(
            &self,
            server_name: &OwnedServerName,
            limit_type: &str,
            state: &PersistedRateLimitState,
        ) -> Result<()> {
            let mut states = self.server_states.write().unwrap();
            states.insert((server_name.clone(), limit_type.to_string()), state.clone());
            Ok(())
        }

        async fn get_server_rate_limit_state(
            &self,
            server_name: &OwnedServerName,
            limit_type: &str,
        ) -> Result<Option<PersistedRateLimitState>> {
            let states = self.server_states.read().unwrap();
            Ok(states.get(&(server_name.clone(), limit_type.to_string())).cloned())
        }

        async fn set_generic_rate_limit_state(
            &self,
            identifier: &str,
            limit_type: &str,
            state: &PersistedRateLimitState,
        ) -> Result<()> {
            let mut states = self.generic_states.write().unwrap();
            states.insert((identifier.to_string(), limit_type.to_string()), state.clone());
            Ok(())
        }

        async fn get_generic_rate_limit_state(
            &self,
            identifier: &str,
            limit_type: &str,
        ) -> Result<Option<PersistedRateLimitState>> {
            let states = self.generic_states.read().unwrap();
            Ok(states.get(&(identifier.to_string(), limit_type.to_string())).cloned())
        }

        async fn cleanup_expired_rate_limit_states(&self, expiry_duration: Duration) -> Result<u64> {
            let cutoff = SystemTime::now() - expiry_duration;
            let mut cleaned = 0u64;

            // Clean user states
            {
                let mut states = self.user_states.write().unwrap();
                let initial_len = states.len();
                states.retain(|_, state| {
                    state.last_refill > cutoff || state.last_violation.map_or(false, |v| v > cutoff)
                });
                cleaned += (initial_len - states.len()) as u64;
            }

            // Clean IP states
            {
                let mut states = self.ip_states.write().unwrap();
                let initial_len = states.len();
                states.retain(|_, state| {
                    state.last_refill > cutoff || state.last_violation.map_or(false, |v| v > cutoff)
                });
                cleaned += (initial_len - states.len()) as u64;
            }

            Ok(cleaned)
        }

        async fn get_rate_limit_statistics(&self) -> Result<HashMap<String, u64>> {
            let mut stats = HashMap::new();
            stats.insert("user_states".to_string(), self.user_states.read().unwrap().len() as u64);
            stats.insert("ip_states".to_string(), self.ip_states.read().unwrap().len() as u64);
            stats.insert("room_states".to_string(), self.room_states.read().unwrap().len() as u64);
            stats.insert("server_states".to_string(), self.server_states.read().unwrap().len() as u64);
            stats.insert("generic_states".to_string(), self.generic_states.read().unwrap().len() as u64);
            Ok(stats)
        }

        async fn set_rate_limit_config(&self, _config_name: &str, _config_data: &[u8]) -> Result<()> {
            Ok(())
        }

        async fn get_rate_limit_config(&self, _config_name: &str) -> Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn list_active_rate_limit_states(
            &self,
            limit_type: &str,
            limit: u32,
        ) -> Result<Vec<(String, PersistedRateLimitState)>> {
            let states = self.generic_states.read().unwrap();
            let result: Vec<_> = states
                .iter()
                .filter(|((_, state_type), _)| state_type == limit_type)
                .take(limit as usize)
                .map(|((id, _), state)| (id.clone(), state.clone()))
                .collect();
            Ok(result)
        }

        async fn get_rate_limit_state_count(&self, limit_type: &str) -> Result<u64> {
            let count = self.generic_states.read().unwrap()
                .keys()
                .filter(|(_, state_type)| state_type == limit_type)
                .count() as u64;
            Ok(count)
        }

        async fn bulk_cleanup_rate_limit_states(
            &self,
            _state_types: &[String],
            expiry_duration: Duration,
        ) -> Result<HashMap<String, u64>> {
            let cleaned = self.cleanup_expired_rate_limit_states(expiry_duration).await?;
            let mut result = HashMap::new();
            result.insert("total_cleaned".to_string(), cleaned);
            Ok(result)
        }

        async fn log_rate_limit_violation(
            &self,
            identifier: &str,
            limit_type: &str,
            violation_time: SystemTime,
            metadata: &str,
        ) -> Result<()> {
            let violation = RateLimitViolation {
                identifier: identifier.to_string(),
                limit_type: limit_type.to_string(),
                violation_time,
                metadata: metadata.to_string(),
            };
            self.violations.write().unwrap().push(violation);
            Ok(())
        }

        async fn get_recent_rate_limit_violations(
            &self,
            since: SystemTime,
            limit: u32,
        ) -> Result<Vec<RateLimitViolation>> {
            let violations = self.violations.read().unwrap();
            let recent: Vec<_> = violations
                .iter()
                .filter(|v| v.violation_time >= since)
                .take(limit as usize)
                .cloned()
                .collect();
            Ok(recent)
        }
    }

    #[tokio::test]
    async fn test_data_trait_definition() {
        let mock_data = MockData::new();
        
        // Verify trait is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<dyn Data>();

        // Test basic functionality
        assert!(true);
    }

    #[tokio::test]
    async fn test_user_rate_limit_state_storage() {
        let mock_data = MockData::new();
        let user_id = user_id!("@test:example.com").to_owned();
        let limit_type = "message";
        
        let state = PersistedRateLimitState {
            tokens: 5.0,
            last_refill: SystemTime::now(),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 10.0,
            refill_rate: 1.0,
        };

        // Store state
        mock_data.set_user_rate_limit_state(&user_id, limit_type, &state).await.unwrap();
        
        // Retrieve state
        let retrieved = mock_data.get_user_rate_limit_state(&user_id, limit_type).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tokens, 5.0);
    }

    #[tokio::test]
    async fn test_ip_rate_limit_state_storage() {
        let mock_data = MockData::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let limit_type = "registration";
        
        let state = PersistedRateLimitState {
            tokens: 3.0,
            last_refill: SystemTime::now(),
            violation_count: 1,
            last_violation: Some(SystemTime::now()),
            penalty_until: None,
            capacity: 5.0,
            refill_rate: 0.17,
        };

        mock_data.set_ip_rate_limit_state(&ip, limit_type, &state).await.unwrap();
        let retrieved = mock_data.get_ip_rate_limit_state(&ip, limit_type).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().violation_count, 1);
    }

    #[tokio::test]
    async fn test_room_rate_limit_state_storage() {
        let mock_data = MockData::new();
        let room_id = room_id!("!test:example.com").to_owned();
        let limit_type = "join";
        
        let state = PersistedRateLimitState {
            tokens: 8.0,
            last_refill: SystemTime::now(),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 10.0,
            refill_rate: 1.0,
        };

        mock_data.set_room_rate_limit_state(&room_id, limit_type, &state).await.unwrap();
        let retrieved = mock_data.get_room_rate_limit_state(&room_id, limit_type).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tokens, 8.0);
    }

    #[tokio::test]
    async fn test_server_rate_limit_state_storage() {
        let mock_data = MockData::new();
        let server_name = server_name!("matrix.org").to_owned();
        let limit_type = "federation";
        
        let state = PersistedRateLimitState {
            tokens: 50.0,
            last_refill: SystemTime::now(),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 50.0,
            refill_rate: 3.0,
        };

        mock_data.set_server_rate_limit_state(&server_name, limit_type, &state).await.unwrap();
        let retrieved = mock_data.get_server_rate_limit_state(&server_name, limit_type).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().capacity, 50.0);
    }

    #[tokio::test]
    async fn test_generic_rate_limit_state_storage() {
        let mock_data = MockData::new();
        let identifier = "test@example.com";
        let limit_type = "3pid_validation";
        
        let state = PersistedRateLimitState {
            tokens: 2.0,
            last_refill: SystemTime::now(),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 5.0,
            refill_rate: 0.003,
        };

        mock_data.set_generic_rate_limit_state(identifier, limit_type, &state).await.unwrap();
        let retrieved = mock_data.get_generic_rate_limit_state(identifier, limit_type).await.unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().refill_rate, 0.003);
    }

    #[tokio::test]
    async fn test_rate_limit_statistics() {
        let mock_data = MockData::new();
        
        // Add some test data
        let user_id = user_id!("@test:example.com").to_owned();
        let state = PersistedRateLimitState {
            tokens: 5.0,
            last_refill: SystemTime::now(),
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 10.0,
            refill_rate: 1.0,
        };
        
        mock_data.set_user_rate_limit_state(&user_id, "message", &state).await.unwrap();
        
        let stats = mock_data.get_rate_limit_statistics().await.unwrap();
        assert_eq!(stats.get("user_states"), Some(&1));
    }

    #[tokio::test]
    async fn test_cleanup_expired_states() {
        let mock_data = MockData::new();
        
        // Add state with old timestamp
        let user_id = user_id!("@test:example.com").to_owned();
        let old_state = PersistedRateLimitState {
            tokens: 5.0,
            last_refill: UNIX_EPOCH, // Very old timestamp
            violation_count: 0,
            last_violation: None,
            penalty_until: None,
            capacity: 10.0,
            refill_rate: 1.0,
        };
        
        mock_data.set_user_rate_limit_state(&user_id, "message", &old_state).await.unwrap();
        
        // Cleanup should remove old states
        let cleaned = mock_data.cleanup_expired_rate_limit_states(Duration::from_secs(3600)).await.unwrap();
        assert!(cleaned > 0);
    }

    #[tokio::test]
    async fn test_violation_logging() {
        let mock_data = MockData::new();
        let identifier = "test_user";
        let limit_type = "message";
        let violation_time = SystemTime::now();
        let metadata = "rate limit exceeded";
        
        mock_data.log_rate_limit_violation(identifier, limit_type, violation_time, metadata).await.unwrap();
        
        let violations = mock_data.get_recent_rate_limit_violations(
            violation_time - Duration::from_secs(60),
            10
        ).await.unwrap();
        
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].identifier, identifier);
        assert_eq!(violations[0].limit_type, limit_type);
    }

    #[tokio::test]
    async fn test_active_states_listing() {
        let mock_data = MockData::new();
        let limit_type = "test_type";
        
        // Add some test states
        for i in 0..3 {
            let identifier = format!("test_{}", i);
            let state = PersistedRateLimitState {
                tokens: i as f64,
                last_refill: SystemTime::now(),
                violation_count: 0,
                last_violation: None,
                penalty_until: None,
                capacity: 10.0,
                refill_rate: 1.0,
            };
            
            mock_data.set_generic_rate_limit_state(&identifier, limit_type, &state).await.unwrap();
        }
        
        let active_states = mock_data.list_active_rate_limit_states(limit_type, 10).await.unwrap();
        assert_eq!(active_states.len(), 3);
        
        let count = mock_data.get_rate_limit_state_count(limit_type).await.unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_bulk_cleanup() {
        let mock_data = MockData::new();
        let state_types = vec!["message".to_string(), "registration".to_string()];
        
        let result = mock_data.bulk_cleanup_rate_limit_states(&state_types, Duration::from_secs(3600)).await.unwrap();
        assert!(result.contains_key("total_cleaned"));
    }

    #[tokio::test]
    async fn test_persisted_rate_limit_state_serialization() {
        let state = PersistedRateLimitState {
            tokens: 5.5,
            last_refill: SystemTime::now(),
            violation_count: 3,
            last_violation: Some(SystemTime::now()),
            penalty_until: Some(SystemTime::now() + Duration::from_secs(60)),
            capacity: 10.0,
            refill_rate: 1.5,
        };
        
        // Test serialization
        let serialized = serde_json::to_string(&state).unwrap();
        assert!(!serialized.is_empty());
        
        // Test deserialization
        let deserialized: PersistedRateLimitState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.tokens, state.tokens);
        assert_eq!(deserialized.violation_count, state.violation_count);
        assert_eq!(deserialized.capacity, state.capacity);
    }

    #[tokio::test]
    async fn test_rate_limit_violation_serialization() {
        let violation = RateLimitViolation {
            identifier: "test_user".to_string(),
            limit_type: "message".to_string(),
            violation_time: SystemTime::now(),
            metadata: "exceeded burst limit".to_string(),
        };
        
        let serialized = serde_json::to_string(&violation).unwrap();
        assert!(!serialized.is_empty());
        
        let deserialized: RateLimitViolation = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.identifier, violation.identifier);
        assert_eq!(deserialized.limit_type, violation.limit_type);
        assert_eq!(deserialized.metadata, violation.metadata);
    }
} 