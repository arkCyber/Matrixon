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
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, instrument, warn};
use serde_json;

use crate::{
    config::{CaptchaConfig, CaptchaProvider},
    Error, Result,
};

/// CAPTCHA verification request data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaRequest {
    /// CAPTCHA response token from client
    pub response: String,
    /// Client IP address
    pub remote_ip: Option<String>,
    /// Additional challenge data
    pub challenge_data: Option<HashMap<String, String>>,
}

/// CAPTCHA verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaResponse {
    /// Verification success status
    pub success: bool,
    /// Verification score (for reCAPTCHA v3)
    pub score: Option<f32>,
    /// Error codes if verification failed
    #[serde(default)]
    pub error_codes: Vec<String>,
    /// Challenge timestamp
    pub challenge_ts: Option<String>,
    /// Hostname of the site where challenge was solved
    pub hostname: Option<String>,
}

/// Rate limiting state for IP addresses
#[derive(Debug, Clone)]
struct RateLimitState {
    attempts_this_minute: u32,
    attempts_this_hour: u32,
    last_attempt_time: Instant,
    consecutive_failures: u32,
}

/// CAPTCHA verification service
pub struct Service {
    config: CaptchaConfig,
    http_client: Client,
    rate_limit_cache: Arc<RwLock<HashMap<String, RateLimitState>>>,
    verification_cache: Arc<RwLock<HashMap<String, (bool, Instant)>>>,
}

impl Service {
    /// Create new CAPTCHA service with configuration
    pub fn new(config: CaptchaConfig) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(config.get_verification_timeout())
            .build()
            .map_err(|_| Error::BadConfig("Failed to create HTTP client"))?;

        Ok(Self {
            config,
            http_client,
            rate_limit_cache: Arc::new(RwLock::new(HashMap::new())),
            verification_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Verify CAPTCHA response
    #[instrument(level = "debug", skip(self, request))]
    pub async fn verify(&self, request: CaptchaRequest) -> Result<bool> {
        let start = Instant::now();
        
        if !self.config.enabled {
            debug!("🔧 CAPTCHA verification disabled, allowing request");
            return Ok(true);
        }

        // Check IP whitelist
        if let Some(ip) = &request.remote_ip {
            if self.config.is_ip_whitelisted(ip) {
                debug!("🔧 IP {} is whitelisted, bypassing CAPTCHA", ip);
                return Ok(true);
            }
        }

        // Check rate limiting
        if let Some(ip) = &request.remote_ip {
            if let Err(e) = self.check_rate_limit(ip).await {
                warn!("⚠️ Rate limit exceeded for IP {}: {}", ip, e);
                return Err(e);
            }
        }

        // Check cache for recent verification
        if self.config.performance.enable_caching {
            if let Some(cached_result) = self.check_verification_cache(&request.response).await {
                debug!("✅ Using cached CAPTCHA verification result");
                return Ok(cached_result);
            }
        }

        // Perform actual verification
        let verification_result = self.verify_with_provider(&request).await;
        
        // Update rate limiting state
        if let Some(ip) = &request.remote_ip {
            self.update_rate_limit_state(ip, verification_result.is_ok()).await;
        }

        // Cache successful verifications
        if verification_result.is_ok() && self.config.performance.enable_caching {
            self.cache_verification_result(&request.response, true).await;
        }

        let duration = start.elapsed();
        
        match &verification_result {
            Ok(success) => {
                info!(
                    "✅ CAPTCHA verification completed in {:?}: {}",
                    duration,
                    if *success { "SUCCESS" } else { "FAILED" }
                );
            }
            Err(e) => {
                error!("❌ CAPTCHA verification error in {:?}: {}", duration, e);
            }
        }

        verification_result
    }

    /// Check if request is within rate limits
    async fn check_rate_limit(&self, ip: &str) -> Result<()> {
        let mut cache = self.rate_limit_cache.write().unwrap();
        let now = Instant::now();
        
        let state = cache.entry(ip.to_string()).or_insert_with(|| RateLimitState {
            attempts_this_minute: 0,
            attempts_this_hour: 0,
            last_attempt_time: now,
            consecutive_failures: 0,
        });

        // Reset counters if enough time has passed
        if now.duration_since(state.last_attempt_time) > Duration::from_secs(60) {
            state.attempts_this_minute = 0;
        }
        
        if now.duration_since(state.last_attempt_time) > Duration::from_secs(3600) {
            state.attempts_this_hour = 0;
            state.consecutive_failures = 0;
        }

        // Check rate limits
        if state.attempts_this_minute >= self.config.rate_limit.max_attempts_per_minute {
            return Err(Error::BadRequestString(
                ruma::api::client::error::ErrorKind::LimitExceeded { retry_after: None },
                "Too many CAPTCHA attempts per minute",
            ));
        }

        if state.attempts_this_hour >= self.config.rate_limit.max_attempts_per_hour {
            return Err(Error::BadRequestString(
                ruma::api::client::error::ErrorKind::LimitExceeded { retry_after: None },
                "Too many CAPTCHA attempts per hour",
            ));
        }

        // Progressive delay for repeated failures
        if state.consecutive_failures > 0 {
            let delay_ms = (self.config.rate_limit.cooldown_seconds as f32 * 
                          self.config.rate_limit.progressive_delay.powi(state.consecutive_failures as i32)) as u64;
            
            if now.duration_since(state.last_attempt_time) < Duration::from_millis(delay_ms) {
                return Err(Error::BadRequestString(
                    ruma::api::client::error::ErrorKind::LimitExceeded { retry_after: None },
                    "Please wait before attempting CAPTCHA verification again",
                ));
            }
        }

        state.attempts_this_minute += 1;
        state.attempts_this_hour += 1;
        state.last_attempt_time = now;

        Ok(())
    }

    /// Update rate limiting state after verification
    async fn update_rate_limit_state(&self, ip: &str, success: bool) {
        let mut cache = self.rate_limit_cache.write().unwrap();
        
        if let Some(state) = cache.get_mut(ip) {
            if success {
                state.consecutive_failures = 0;
            } else {
                state.consecutive_failures += 1;
            }
        }
    }

    /// Check verification cache for recent results
    async fn check_verification_cache(&self, response_token: &str) -> Option<bool> {
        let cache = self.verification_cache.read().unwrap();
        
        if let Some((result, timestamp)) = cache.get(response_token) {
            if timestamp.elapsed() < self.config.get_cache_ttl() {
                return Some(*result);
            }
        }
        
        None
    }

    /// Cache verification result
    async fn cache_verification_result(&self, response_token: &str, result: bool) {
        let mut cache = self.verification_cache.write().unwrap();
        cache.insert(response_token.to_string(), (result, Instant::now()));
        
        // Cleanup old entries (simple implementation)
        if cache.len() > 10000 {
            cache.retain(|_, (_, timestamp)| {
                timestamp.elapsed() < self.config.get_cache_ttl()
            });
        }
    }

    /// Perform verification with the configured provider
    async fn verify_with_provider(&self, request: &CaptchaRequest) -> Result<bool> {
        let provider_config = self.config.provider_config.as_ref()
            .ok_or_else(|| Error::BadConfig("CAPTCHA provider configuration missing"))?;

        let verification_url = self.config.get_verification_url()
            .ok_or_else(|| Error::BadConfig("CAPTCHA verification URL not configured"))?;

        // Build verification request parameters
        let mut params = HashMap::new();
        params.insert("secret".to_string(), provider_config.secret_key.clone());
        params.insert("response".to_string(), request.response.clone());
        
        if let Some(ip) = &request.remote_ip {
            params.insert("remoteip".to_string(), ip.clone());
        }

        // Add provider-specific parameters
        for (key, value) in &provider_config.extra_params {
            params.insert(key.clone(), value.clone());
        }

        if self.config.debug_logging {
            debug!("🔧 Sending CAPTCHA verification request to: {}", verification_url);
        }

        // Perform HTTP request with retries
        let mut last_error = None;
        
        for attempt in 1..=self.config.performance.retry_attempts {
            match self.send_verification_request(verification_url, &params).await {
                Ok(response) => {
                    if self.config.debug_logging {
                        debug!("🔧 CAPTCHA provider response: {:?}", response);
                    }
                    
                    return self.process_verification_response(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.config.performance.retry_attempts {
                        warn!("⚠️ CAPTCHA verification attempt {} failed, retrying...", attempt);
                        sleep(Duration::from_millis(100 * attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::BadServerResponse("CAPTCHA verification failed after all retries")
        }))
    }

    /// Send HTTP verification request
    async fn send_verification_request(
        &self,
        url: &str,
        params: &HashMap<String, String>,
    ) -> Result<CaptchaResponse> {
        let http_response = self.http_client.post(url).form(params).send();
        let response = timeout(self.config.get_verification_timeout(), http_response)
            .await
            .map_err(|_| Error::BadServerResponse("CAPTCHA verification request timeout"))?
            .map_err(|_| Error::BadServerResponse("CAPTCHA verification request failed"))?;

        if !response.status().is_success() {
            error!("❌ CAPTCHA provider returned error status: {}", response.status());
            return Err(Error::BadServerResponse("CAPTCHA provider returned error status"));
        }

        let response_text = response.text().await
            .map_err(|e| {
                error!("❌ Failed to read CAPTCHA response text: {}", e);
                Error::BadServerResponse("Failed to read CAPTCHA provider response")
            })?;

        let captcha_response: CaptchaResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                error!("❌ Failed to parse CAPTCHA response as JSON: {}", e);
                Error::BadServerResponse("Invalid CAPTCHA provider response")
            })?;

        Ok(captcha_response)
    }

    /// Process verification response based on provider and strictness
    fn process_verification_response(&self, response: CaptchaResponse) -> Result<bool> {
        if !response.success {
            if self.config.debug_logging {
                debug!("❌ CAPTCHA verification failed: {:?}", response.error_codes);
            }
            return Ok(false);
        }

        // Additional checks based on provider type
        match self.config.provider {
            CaptchaProvider::RecaptchaV3 => {
                if let Some(score) = response.score {
                    let threshold = self.config.provider_config.as_ref()
                        .map(|c| c.min_score)
                        .unwrap_or(0.5);
                    
                    let success = score >= threshold;
                    
                    if self.config.debug_logging {
                        debug!("🔧 reCAPTCHA v3 score: {} (threshold: {})", score, threshold);
                    }
                    
                    return Ok(success);
                } else {
                    warn!("⚠️ reCAPTCHA v3 response missing score");
                    return Ok(false);
                }
            }
            _ => {
                // For other providers, success flag is sufficient
                return Ok(true);
            }
        }
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        
        let rate_limit_cache = self.rate_limit_cache.read().unwrap();
        let verification_cache = self.verification_cache.read().unwrap();
        
        stats.insert("rate_limited_ips".to_string(), rate_limit_cache.len() as u64);
        stats.insert("cached_verifications".to_string(), verification_cache.len() as u64);
        
        stats
    }

    /// Cleanup expired cache entries
    pub async fn cleanup_cache(&self) {
        let mut rate_limit_cache = self.rate_limit_cache.write().unwrap();
        let mut verification_cache = self.verification_cache.write().unwrap();
        
        let now = Instant::now();
        
        // Cleanup rate limit cache (entries older than 1 hour)
        rate_limit_cache.retain(|_, state| {
            now.duration_since(state.last_attempt_time) < Duration::from_secs(3600)
        });
        
        // Cleanup verification cache
        verification_cache.retain(|_, (_, timestamp)| {
            timestamp.elapsed() < self.config.get_cache_ttl()
        });
        
        debug!("🔧 CAPTCHA cache cleanup completed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
            use crate::config::{CaptchaProviderConfig, CaptchaProvider};
    use std::collections::HashMap;

    fn create_test_config() -> CaptchaConfig {
        CaptchaConfig {
            enabled: true,
            provider: CaptchaProvider::HCaptcha,
            provider_config: Some(CaptchaProviderConfig {
                site_key: "test_site_key".to_string(),
                secret_key: "test_secret_key".to_string(),
                verification_url: None,
                min_score: 0.5,
                extra_params: HashMap::new(),
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_service_creation() {
        let config = create_test_config();
        let service = Service::new(config);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_disabled_captcha() {
        let mut config = create_test_config();
        config.enabled = false;
        
        let service = Service::new(config).unwrap();
        let request = CaptchaRequest {
            response: "test_response".to_string(),
            remote_ip: Some("127.0.0.1".to_string()),
            challenge_data: None,
        };
        
        let result = service.verify(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_ip_whitelisting() {
        let mut config = create_test_config();
        config.security.enable_ip_whitelist = true;
        config.security.whitelisted_ips.push("127.0.0.1".to_string());
        
        let service = Service::new(config).unwrap();
        let request = CaptchaRequest {
            response: "test_response".to_string(),
            remote_ip: Some("127.0.0.1".to_string()),
            challenge_data: None,
        };
        
        let result = service.verify(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut config = create_test_config();
        config.rate_limit.max_attempts_per_minute = 1;
        
        let service = Service::new(config).unwrap();
        
        // First request should be allowed to check rate limit
        let result = service.check_rate_limit("192.168.1.1").await;
        assert!(result.is_ok());
        
        // Second request should be rate limited
        let result = service.check_rate_limit("192.168.1.1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let config = create_test_config();
        let service = Service::new(config).unwrap();
        
        // Cache a result
        service.cache_verification_result("test_token", true).await;
        
        // Check if cached result is returned
        let cached = service.check_verification_cache("test_token").await;
        assert_eq!(cached, Some(true));
        
        // Check non-existent token
        let not_cached = service.check_verification_cache("non_existent").await;
        assert_eq!(not_cached, None);
    }

    #[tokio::test]
    async fn test_recaptcha_v3_processing() {
        let mut config = create_test_config();
        config.provider = CaptchaProvider::RecaptchaV3;
        config.provider_config.as_mut().unwrap().min_score = 0.7;
        
        let service = Service::new(config).unwrap();
        
        // Test response with high score
        let response = CaptchaResponse {
            success: true,
            score: Some(0.8),
            error_codes: vec![],
            challenge_ts: None,
            hostname: None,
        };
        
        let result = service.process_verification_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        
        // Test response with low score
        let response = CaptchaResponse {
            success: true,
            score: Some(0.3),
            error_codes: vec![],
            challenge_ts: None,
            hostname: None,
        };
        
        let result = service.process_verification_response(response);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_stats_collection() {
        let config = create_test_config();
        let service = Service::new(config).unwrap();
        
        let stats = service.get_stats().await;
        assert!(stats.contains_key("rate_limited_ips"));
        assert!(stats.contains_key("cached_verifications"));
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let config = create_test_config();
        let service = Service::new(config).unwrap();
        
        // Add some test data to caches
        service.cache_verification_result("test1", true).await;
        service.cache_verification_result("test2", false).await;
        
        // Should not panic
        service.cleanup_cache().await;
        
        let stats = service.get_stats().await;
        assert!(stats.contains_key("cached_verifications"));
    }
} 
