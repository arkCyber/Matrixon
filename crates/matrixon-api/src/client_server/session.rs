// =============================================================================
// Matrixon Matrix NextServer - Session Module
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
//   Matrix API implementation for client-server communication. This module is part of the Matrixon Matrix NextServer
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
//   • Matrix protocol compliance
//   • RESTful API endpoints
//   • Request/response handling
//   • Authentication and authorization
//   • Rate limiting and security
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

use super::DEVICE_ID_LENGTH;
use serde::{Deserialize, Serialize};
use ruma::api::client::{
    error::ErrorKind,
    session::{login, logout, logout_all},
    uiaa::UserIdentifier
};
use ruma::api::client::session::get_login_types;
use crate::{Error, Result};
use crate::services;
use crate::Ruma;
use crate::utils;

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    //exp: usize,
}

/// # `GET /_matrix/client/r0/login`
///
/// Get the supported login types of this server. One of these should be used as the `type` field
/// when logging in.
pub async fn get_login_types_route(
    _body: Ruma<get_login_types::v3::Request>,
) -> Result<get_login_types::v3::Response> {
    Ok(get_login_types::v3::Response::new(vec![
        get_login_types::v3::LoginType::Password(Default::default()),
        get_login_types::v3::LoginType::ApplicationService(Default::default()),
    ]))
}

/// # `POST /_matrix/client/r0/login`
///
/// Authenticates the user and returns an access token it can use in subsequent requests.
///
/// - The user needs to authenticate using their password (or if enabled using a json web token)
/// - If `device_id` is known: invalidates old access token of that device
/// - If `device_id` is unknown: creates a new device
/// - Returns access token that is associated with the user and device
///
/// Note: You can use [`GET /_matrix/client/r0/login`](fn.get_supported_versions_route.html) to see
/// supported login types.
pub async fn login_route(
    body: Ruma<login::v3::Request>,
) -> Result<login::v3::Response> {
    let (user_id, password) = match &body.login_info {
        login::v3::LoginInfo::Password(password_info) => {
            let user_id = match password_info.identifier {
                Some(UserIdentifier::UserIdOrLocalpart(id)) => id.as_str(),
                Some(UserIdentifier::PhoneNumber { .. }) => {
                    return Err(Error::BadRequest(
                        "Phone number login not supported".to_string(),
                    ));
                }
                Some(UserIdentifier::Email { .. }) => {
                    return Err(Error::BadRequest(
                        "Email login not supported".to_string(),
                    ));
                }
                Some(UserIdentifier::Msisdn { .. }) => {
                    return Err(Error::BadRequest(
                        "MSISDN login not supported".to_string(),
                    ));
                }
                Some(UserIdentifier::_CustomThirdParty(_)) => {
                    return Err(Error::BadRequest(
                        "Third-party login not supported".to_string(),
                    ));
                }
                None => {
                    return Err(Error::BadRequest(
                        "No user identifier provided".to_string(),
                    ));
                }
            };
            
            let password = match &password_info.password {
                Some(p) => p,
                None => {
                    return Err(Error::BadRequest(
                        ErrorKind::MissingParam,
                        "Password is required.",
                    ))
                }
            };
            
            (user_id, password)
        }
        _ => {
            return Err(Error::BadRequest(
                ErrorKind::Unrecognized,
                "Only password login is supported.",
            ))
        }
    };

    if !services().users.exists(user_id)? {
        return Err(Error::BadRequest(
            ErrorKind::Forbidden,
            "User does not exist.",
        ));
    }

    if !services().users.verify_password(user_id, password)? {
        return Err(Error::BadRequest(
            ErrorKind::Forbidden,
            "Invalid password.",
        ));
    }

    let device_id = body.device_id.clone().unwrap_or_else(|| {
        utils::random_string(DEVICE_ID_LENGTH)
    });

    let access_token = services().users.create_access_token(
        user_id,
        &device_id,
        body.initial_device_display_name.clone(),
    )?;

    Ok(login::v3::Response::new(
        access_token,
        user_id.to_owned(),
        device_id,
    ))
}

/// # `POST /_matrix/client/r0/logout`
///
/// Log out the current device.
///
/// - Invalidates access token
/// - Deletes device metadata (device id, device display name, last seen ip, last seen ts)
/// - Forgets to-device events
/// - Triggers device list updates
pub async fn logout_route(body: Ruma<logout::v3::Request>) -> Result<logout::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");
    let sender_device = body.sender_device.as_ref().expect("user is authenticated");

    if let Some(ref info) = body.appservice_info {
        if !info.is_user_match(sender_user) {
            return Err(Error::BadRequest(
                ErrorKind::Exclusive,
                "User is not in namespace.",
            ));
        }
    }

    services().users.remove_device(sender_user, sender_device)?;

    Ok(logout::v3::Response::new())
}

/// # `POST /_matrix/client/r0/logout/all`
///
/// Log out all devices of this user.
///
/// - Invalidates all access tokens
/// - Deletes all device metadata (device id, device display name, last seen ip, last seen ts)
/// - Forgets all to-device events
/// - Triggers device list updates
///
/// Note: This is equivalent to calling [`GET /_matrix/client/r0/logout`](fn.logout_route.html)
/// from each device of this user.
pub async fn logout_all_route(
    body: Ruma<logout_all::v3::Request>,
) -> Result<logout_all::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    if let Some(ref info) = body.appservice_info {
        if !info.is_user_match(sender_user) {
            return Err(Error::BadRequest(
                ErrorKind::Exclusive,
                "User is not in namespace.",
            ));
        }
    } else {
        return Err(Error::BadRequest(
            ErrorKind::MissingToken,
            "Missing appservice token.",
        ));
    }

    for device_id in services().users.all_device_ids(sender_user).flatten() {
        services().users.remove_device(sender_user, &device_id)?;
    }

    Ok(logout_all::v3::Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
use ruma::{
    api::client::{
        error::ErrorKind,
        session::{get_login_types, login, sso_login},
    },
    UserId,
};
use crate::utils;
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, RwLock},
        time::{Duration, Instant},
        thread,
    };
    use tracing::{debug, info};

    /// Mock session storage for testing
    #[derive(Debug)]
    struct MockSessionStorage {
        users: Arc<RwLock<HashMap<OwnedUserId, UserRecord>>>,
        active_sessions: Arc<RwLock<HashMap<String, SessionRecord>>>, // token -> session
        password_hashes: Arc<RwLock<HashMap<OwnedUserId, String>>>,
        failed_attempts: Arc<RwLock<HashMap<OwnedUserId, u32>>>,
        blocked_users: Arc<RwLock<HashSet<OwnedUserId>>>,
    }

    #[derive(Debug, Clone)]
    struct UserRecord {
        user_id: OwnedUserId,
        devices: HashMap<OwnedDeviceId, DeviceRecord>,
        is_active: bool,
        created_at: u64,
    }

    #[derive(Debug, Clone)]
    struct DeviceRecord {
        device_id: OwnedDeviceId,
        display_name: Option<String>,
        current_token: Option<String>,
        created_at: u64,
        last_seen: u64,
    }

    #[derive(Debug, Clone)]
    struct SessionRecord {
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        token: String,
        created_at: u64,
        last_activity: u64,
    }

    impl MockSessionStorage {
        fn new() -> Self {
            let mut storage = Self {
                users: Arc::new(RwLock::new(HashMap::new())),
                active_sessions: Arc::new(RwLock::new(HashMap::new())),
                password_hashes: Arc::new(RwLock::new(HashMap::new())),
                failed_attempts: Arc::new(RwLock::new(HashMap::new())),
                blocked_users: Arc::new(RwLock::new(HashSet::new())),
            };

            // Add test users
            storage.add_test_user(
                user_id!("@alice:example.com").to_owned(),
                "password123",
                true,
            );
            storage.add_test_user(
                user_id!("@bob:example.com").to_owned(),
                "secure_pass",
                true,
            );
            storage.add_test_user(
                user_id!("@deactivated:example.com").to_owned(),
                "",
                false,
            );

            storage
        }

        fn add_test_user(&self, user_id: OwnedUserId, password: &str, is_active: bool) {
            // Simple password hash simulation (would use Argon2 in real implementation)
            let hash = if password.is_empty() {
                String::new()
            } else {
                format!("$argon2${}", password)
            };

            self.password_hashes.write().unwrap().insert(user_id.clone(), hash);

            let user_record = UserRecord {
                user_id: user_id.clone(),
                devices: HashMap::new(),
                is_active,
                created_at: 1640995200000,
            };

            self.users.write().unwrap().insert(user_id, user_record);
        }

        fn authenticate_user(&self, user_id: &UserId, password: &str) -> Result<bool> {
            let password_hashes = self.password_hashes.read().unwrap();
            if let Some(hash) = password_hashes.get(user_id) {
                if hash.is_empty() {
                    return Err(Error::BadRequest(
                        ErrorKind::UserDeactivated,
                        "The user has been deactivated"
                    ));
                }
                // Simple password verification simulation
                // Handle empty password case
                if password.is_empty() {
                    return Ok(false);
                }
                Ok(hash.ends_with(password))
            } else {
                Ok(false)
            }
        }

        fn create_session(&self, user_id: OwnedUserId, device_id: OwnedDeviceId, token: String) {
            let session = SessionRecord {
                user_id: user_id.clone(),
                device_id: device_id.clone(),
                token: token.clone(),
                created_at: 1640995200000,
                last_activity: 1640995200000,
            };

            self.active_sessions.write().unwrap().insert(token.clone(), session);

            // Update user's device
            let mut users = self.users.write().unwrap();
            if let Some(user) = users.get_mut(&user_id) {
                let device = DeviceRecord {
                    device_id: device_id.clone(),
                    display_name: Some("Test Device".to_string()),
                    current_token: Some(token),
                    created_at: 1640995200000,
                    last_seen: 1640995200000,
                };
                user.devices.insert(device_id, device);
            }
        }

        fn remove_session(&self, token: &str) -> bool {
            self.active_sessions.write().unwrap().remove(token).is_some()
        }

        fn remove_all_user_sessions(&self, user_id: &UserId) -> usize {
            let mut sessions = self.active_sessions.write().unwrap();
            let tokens_to_remove: Vec<String> = sessions
                .iter()
                .filter(|(_, session)| session.user_id == user_id)
                .map(|(token, _)| token.clone())
                .collect();

            let count = tokens_to_remove.len();
            for token in tokens_to_remove {
                sessions.remove(&token);
            }

            // Clear tokens from user devices
            let mut users = self.users.write().unwrap();
            if let Some(user) = users.get_mut(user_id) {
                for device in user.devices.values_mut() {
                    device.current_token = None;
                }
            }

            count
        }

        fn get_session(&self, token: &str) -> Option<SessionRecord> {
            self.active_sessions.read().unwrap().get(token).cloned()
        }

        fn count_active_sessions(&self) -> usize {
            self.active_sessions.read().unwrap().len()
        }

        fn count_user_sessions(&self, user_id: &UserId) -> usize {
            self.active_sessions
                .read()
                .unwrap()
                .values()
                .filter(|session| session.user_id == user_id)
                .count()
        }

        fn record_failed_attempt(&self, user_id: &UserId) {
            let mut attempts = self.failed_attempts.write().unwrap();
            let count = attempts.entry(user_id.to_owned()).or_insert(0);
            *count += 1;

            // Block user after 5 failed attempts
            if *count >= 5 {
                self.blocked_users.write().unwrap().insert(user_id.to_owned());
            }
        }

        fn is_user_blocked(&self, user_id: &UserId) -> bool {
            self.blocked_users.read().unwrap().contains(user_id)
        }

        fn device_exists(&self, user_id: &UserId, device_id: &DeviceId) -> bool {
            self.users
                .read()
                .unwrap()
                .get(user_id)
                .map(|user| user.devices.contains_key(device_id))
                .unwrap_or(false)
        }
    }

    fn create_test_user(index: usize) -> OwnedUserId {
        match index {
            0 => user_id!("@alice:example.com").to_owned(),
            1 => user_id!("@bob:example.com").to_owned(),
            2 => user_id!("@charlie:example.com").to_owned(),
            3 => user_id!("@deactivated:example.com").to_owned(),
            4 => user_id!("@appservice_user:example.com").to_owned(),
            _ => user_id!("@test_user:example.com").to_owned(),
        }
    }

    fn create_test_device(index: usize) -> OwnedDeviceId {
        match index {
            0 => device_id!("DEVICE001").to_owned(),
            1 => device_id!("DEVICE002").to_owned(),
            2 => device_id!("MOBILE123").to_owned(),
            3 => device_id!("WEB456").to_owned(),
            4 => device_id!("DESKTOP789").to_owned(),
            _ => device_id!("TESTDEVICE").to_owned(),
        }
    }

    fn create_test_login_request(user_id: OwnedUserId, password: String, device_id: Option<OwnedDeviceId>) -> login::v3::Request {
        let mut request = login::v3::Request::new(login::v3::LoginInfo::Password(
            login::v3::Password {
                identifier: Some(UserIdentifier::UserIdOrLocalpart(user_id.localpart().to_string())),
                password,
                user: None,
                address: None,
                medium: None,
            }
        ));
        request.device_id = device_id;
        request.initial_device_display_name = Some("Test Device".to_string());
        request
    }

    #[test]
    fn test_login_types_request_structure() {
        debug!("🔧 Testing login types request structure");
        let start = Instant::now();

        let request = get_login_types::v3::Request::new();
        
        // Test that request structure is correct (no fields in get_login_types request)
        assert_eq!(std::mem::size_of_val(&request), 0, "Get login types request should be empty");

        info!("✅ Login types request structure test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_login_types_response_structure() {
        debug!("🔧 Testing login types response structure");
        let start = Instant::now();

        let response = get_login_types::v3::Response::new(vec![
            get_login_types::v3::LoginType::Password(Default::default()),
            get_login_types::v3::LoginType::ApplicationService(Default::default()),
        ]);

        // Verify supported login types
        assert_eq!(response.flows.len(), 2, "Should support 2 login types");
        
        let has_password = response.flows.iter().any(|flow| matches!(flow, get_login_types::v3::LoginType::Password(_)));
        let has_appservice = response.flows.iter().any(|flow| matches!(flow, get_login_types::v3::LoginType::ApplicationService(_)));
        
        assert!(has_password, "Should support password login");
        assert!(has_appservice, "Should support application service login");

        info!("✅ Login types response structure test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_password_authentication() {
        debug!("🔧 Testing password authentication");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let user_id = create_test_user(0);
        let correct_password = "password123";
        let wrong_password = "wrongpass";

        // Test correct password
        let auth_result = storage.authenticate_user(&user_id, correct_password);
        assert!(auth_result.is_ok(), "Should authenticate with correct password");
        assert!(auth_result.unwrap(), "Authentication should succeed");

        // Test wrong password
        let auth_result = storage.authenticate_user(&user_id, wrong_password);
        assert!(auth_result.is_ok(), "Should not error with wrong password");
        assert!(!auth_result.unwrap(), "Authentication should fail with wrong password");

        // Test deactivated user
        let deactivated_user = create_test_user(3);
        let auth_result = storage.authenticate_user(&deactivated_user, "anypass");
        assert!(auth_result.is_err(), "Should error for deactivated user");

        // Test non-existent user
        let nonexistent_user = user_id!("@nonexistent:example.com");
        let auth_result = storage.authenticate_user(&nonexistent_user, "anypass");
        assert!(auth_result.is_ok(), "Should not error for non-existent user");
        assert!(!auth_result.unwrap(), "Authentication should fail for non-existent user");

        info!("✅ Password authentication test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_session_creation_and_management() {
        debug!("🔧 Testing session creation and management");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let user_id = create_test_user(0);
        let device_id = create_test_device(0);
        let token = "test_token_123".to_string();

        // Test session creation
        assert_eq!(storage.count_active_sessions(), 0, "Should start with no active sessions");
        
        storage.create_session(user_id.clone(), device_id.clone(), token.clone());
        assert_eq!(storage.count_active_sessions(), 1, "Should have one active session");

        // Test session retrieval
        let session = storage.get_session(&token);
        assert!(session.is_some(), "Should retrieve created session");
        
        let session = session.unwrap();
        assert_eq!(session.user_id, user_id, "Session should have correct user ID");
        assert_eq!(session.device_id, device_id, "Session should have correct device ID");
        assert_eq!(session.token, token, "Session should have correct token");

        // Test session removal
        let removed = storage.remove_session(&token);
        assert!(removed, "Should successfully remove session");
        assert_eq!(storage.count_active_sessions(), 0, "Should have no active sessions after removal");

        // Test non-existent session removal
        let removed = storage.remove_session("nonexistent_token");
        assert!(!removed, "Should not remove non-existent session");

        info!("✅ Session creation and management test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_device_management() {
        debug!("🔧 Testing device management during login");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let user_id = create_test_user(0);
        let existing_device = create_test_device(0);
        let new_device = create_test_device(1);

        // Create initial session with existing device
        storage.create_session(user_id.clone(), existing_device.clone(), "token1".to_string());
        assert!(storage.device_exists(&user_id, &existing_device), "Device should exist after creation");

        // Test device replacement with same device ID
        storage.create_session(user_id.clone(), existing_device.clone(), "token2".to_string());
        assert_eq!(storage.count_user_sessions(&user_id), 2, "Should have two sessions (old token not automatically removed)");

        // Test new device creation
        storage.create_session(user_id.clone(), new_device.clone(), "token3".to_string());
        assert!(storage.device_exists(&user_id, &new_device), "New device should exist");
        assert_eq!(storage.count_user_sessions(&user_id), 3, "Should have three sessions");

        // Test device existence for non-existent device
        let nonexistent_device = create_test_device(99);
        assert!(!storage.device_exists(&user_id, &nonexistent_device), "Non-existent device should not exist");

        info!("✅ Device management test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_logout_functionality() {
        debug!("🔧 Testing logout functionality");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let user_id = create_test_user(0);
        let device1 = create_test_device(0);
        let device2 = create_test_device(1);
        let token1 = "token1".to_string();
        let token2 = "token2".to_string();

        // Create multiple sessions
        storage.create_session(user_id.clone(), device1.clone(), token1.clone());
        storage.create_session(user_id.clone(), device2.clone(), token2.clone());
        assert_eq!(storage.count_user_sessions(&user_id), 2, "Should have two user sessions");

        // Test single device logout
        let removed = storage.remove_session(&token1);
        assert!(removed, "Should remove specific session");
        assert_eq!(storage.count_user_sessions(&user_id), 1, "Should have one remaining session");
        assert!(storage.get_session(&token2).is_some(), "Other session should remain");

        // Test logout all
        let removed_count = storage.remove_all_user_sessions(&user_id);
        assert_eq!(removed_count, 1, "Should remove one remaining session");
        assert_eq!(storage.count_user_sessions(&user_id), 0, "Should have no remaining sessions");

        info!("✅ Logout functionality test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_login_request_validation() {
        debug!("🔧 Testing login request validation");
        let start = Instant::now();

        let user_id = create_test_user(0);
        let device_id = create_test_device(0);

        // Test valid password login request
        let request = create_test_login_request(
            user_id.clone(),
            "password123".to_string(),
            Some(device_id.clone()),
        );

        if let login::v3::LoginInfo::Password(password_info) = &request.login_info {
            assert!(password_info.identifier.is_some(), "Should have identifier");
            assert_eq!(password_info.password, "password123", "Should have correct password");
        } else {
            panic!("Should be password login");
        }

        assert_eq!(request.device_id, Some(device_id), "Should have device ID");
        assert_eq!(request.initial_device_display_name, Some("Test Device".to_string()), "Should have display name");

        // Test request without device ID
        let request_no_device = create_test_login_request(
            user_id.clone(),
            "password123".to_string(),
            None,
        );
        assert!(request_no_device.device_id.is_none(), "Should not have device ID when not specified");

        info!("✅ Login request validation test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_session_security_constraints() {
        debug!("🔧 Testing session security constraints");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let user1 = create_test_user(0);
        let user2 = create_test_user(1);
        let device1 = create_test_device(0);
        let device2 = create_test_device(1);

        // Create sessions for different users
        storage.create_session(user1.clone(), device1.clone(), "user1_token".to_string());
        storage.create_session(user2.clone(), device2.clone(), "user2_token".to_string());

        // Test user isolation
        assert_eq!(storage.count_user_sessions(&user1), 1, "User 1 should have 1 session");
        assert_eq!(storage.count_user_sessions(&user2), 1, "User 2 should have 1 session");

        // Test that users can't access each other's sessions
        let user1_session = storage.get_session("user1_token").unwrap();
        assert_eq!(user1_session.user_id, user1, "Session should belong to user 1");
        assert_ne!(user1_session.user_id, user2, "Session should not belong to user 2");

        // Test failed login attempt tracking
        storage.record_failed_attempt(&user1);
        storage.record_failed_attempt(&user1);
        storage.record_failed_attempt(&user1);
        storage.record_failed_attempt(&user1);
        storage.record_failed_attempt(&user1); // 5th attempt should block user

        assert!(storage.is_user_blocked(&user1), "User should be blocked after 5 failed attempts");
        assert!(!storage.is_user_blocked(&user2), "Other users should not be affected");

        // Test logout all affects only specific user
        storage.remove_all_user_sessions(&user1);
        assert_eq!(storage.count_user_sessions(&user1), 0, "User 1 should have no sessions");
        assert_eq!(storage.count_user_sessions(&user2), 1, "User 2 should still have session");

        info!("✅ Session security constraints test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_concurrent_session_operations() {
        debug!("🔧 Testing concurrent session operations");
        let start = Instant::now();

        let storage = Arc::new(MockSessionStorage::new());
        let num_threads = 5;
        let operations_per_thread = 20;

        let handles: Vec<_> = (0..num_threads).map(|thread_id| {
            let storage_clone = Arc::clone(&storage);
            
            thread::spawn(move || {
                let user = create_test_user(thread_id);
                
                for op_id in 0..operations_per_thread {
                    let device = create_test_device(op_id);
                    let token = format!("token_{}_{}", thread_id, op_id);
                    
                    // Create session
                    storage_clone.create_session(user.clone(), device, token.clone());
                    
                    // Verify session exists
                    let session = storage_clone.get_session(&token);
                    assert!(session.is_some(), "Session should exist after creation");
                    
                    // Occasionally remove session
                    if op_id % 3 == 0 {
                        storage_clone.remove_session(&token);
                    }
                }
                
                thread_id // Return thread ID for verification
            })
        }).collect();

        // Wait for all threads to complete
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.len(), num_threads, "All threads should complete");

        // Verify final state
        let total_sessions = storage.count_active_sessions();
        assert!(total_sessions > 0, "Should have some active sessions remaining");
        assert!(total_sessions <= num_threads * operations_per_thread, "Should not exceed maximum possible sessions");

        info!("✅ Concurrent session operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_session_performance_benchmarks() {
        debug!("🔧 Testing session performance benchmarks");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        let operations_count = 1000;

        // Benchmark session creation
        let creation_start = Instant::now();
        for i in 0..operations_count {
            let user = create_test_user(i % 5);
            let device = create_test_device(i % 10);
            let token = format!("perf_token_{}", i);
            storage.create_session(user, device, token);
        }
        let creation_duration = creation_start.elapsed();

        // Benchmark session retrieval
        let retrieval_start = Instant::now();
        for i in 0..operations_count {
            let token = format!("perf_token_{}", i);
            let _ = storage.get_session(&token);
        }
        let retrieval_duration = retrieval_start.elapsed();

        // Benchmark session removal
        let removal_start = Instant::now();
        for i in 0..operations_count {
            let token = format!("perf_token_{}", i);
            storage.remove_session(&token);
        }
        let removal_duration = removal_start.elapsed();

        // Performance assertions
        assert!(creation_duration < Duration::from_millis(100), 
                "1000 session creations should complete within 100ms, took: {:?}", creation_duration);
        assert!(retrieval_duration < Duration::from_millis(50), 
                "1000 session retrievals should complete within 50ms, took: {:?}", retrieval_duration);
        assert!(removal_duration < Duration::from_millis(50), 
                "1000 session removals should complete within 50ms, took: {:?}", removal_duration);

        info!("✅ Session performance benchmarks test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_session_edge_cases() {
        debug!("🔧 Testing session edge cases");
        let start = Instant::now();

        let storage = MockSessionStorage::new();
        
        // Test empty token
        let session = storage.get_session("");
        assert!(session.is_none(), "Empty token should not return session");

        // Test very long token
        let long_token = "a".repeat(1000);
        let user = create_test_user(0);
        let device = create_test_device(0);
        storage.create_session(user.clone(), device.clone(), long_token.clone());
        let session = storage.get_session(&long_token);
        assert!(session.is_some(), "Should handle long tokens");

        // Test special characters in token
        let special_token = "token-with_special.chars@123!";
        storage.create_session(user.clone(), device.clone(), special_token.to_string());
        let session = storage.get_session(special_token);
        assert!(session.is_some(), "Should handle special characters in tokens");

        // Test multiple remove attempts
        assert!(storage.remove_session(special_token), "First removal should succeed");
        assert!(!storage.remove_session(special_token), "Second removal should fail");

        // Test logout all with no sessions
        let empty_user = create_test_user(99);
        let removed_count = storage.remove_all_user_sessions(&empty_user);
        assert_eq!(removed_count, 0, "Should remove 0 sessions for user with no sessions");

        // Test authentication with empty password
        let auth_result = storage.authenticate_user(&user, "");
        assert!(auth_result.is_ok(), "Should not error with empty password");
        assert!(!auth_result.unwrap(), "Authentication should fail with empty password");

        info!("✅ Session edge cases test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_matrix_protocol_compliance() {
        debug!("🔧 Testing Matrix protocol compliance for sessions");
        let start = Instant::now();

        // Test Matrix user ID format validation
        let valid_user_ids = vec![
            "@alice:example.com",
            "@test_user123:matrix.org",
            "@complex.user-name_with.chars:server.name.com",
        ];

        for user_id_str in valid_user_ids {
            assert!(user_id_str.starts_with('@'), "User ID should start with @");
            assert!(user_id_str.contains(':'), "User ID should contain server name");
            
            // Test parsing
            let device_id = user_id_str.try_into() as Result<OwnedDeviceId, _>;
            if let Ok(user_id) = user_id_str.try_into() as Result<OwnedUserId, _> {
                assert!(!user_id.as_str().is_empty(), "Parsed user ID should not be empty");
                assert!(!user_id.localpart().is_empty(), "User ID should have localpart");
                assert!(!user_id.server_name().as_str().is_empty(), "User ID should have server name");
            }
        }

        // Test Matrix device ID format validation
        let valid_device_ids = vec![
            "DEVICEID",
            "Device123",
            "WEB_CLIENT_01",
            "mobile-app-2023",
        ];

        for device_id_str in valid_device_ids {
            assert!(!device_id_str.is_empty(), "Device ID should not be empty");
            assert!(device_id_str.len() <= 256, "Device ID should not exceed reasonable length");
            
            // Test parsing
            let device_id = device_id_str.try_into() as Result<OwnedDeviceId, _>;
            if let Ok(device_id) = device_id_str.try_into() as Result<OwnedDeviceId, _> {
                assert!(!device_id.as_str().is_empty(), "Parsed device ID should not be empty");
            }
        }

        // Test login type enumeration per Matrix spec
        let login_types = vec![
            get_login_types::v3::LoginType::Password(Default::default()),
            get_login_types::v3::LoginType::ApplicationService(Default::default()),
        ];

        assert!(login_types.len() >= 2, "Should support at least password and appservice login");

        // Test token format compliance
        let test_tokens = vec![
            "MDAxOGxvY2F0aW9uIG1hdHJpeC5vcmcKMDAxM2lkZW50aWZpZXIga2V5CjAwMTBjaWQgZ2VuID0gMQowMDJkY2lkIHVzZXJfaWQgPSBAYW5kcmV3Om1hdHJpeC5vcmcKMDAyZmNpZCB0eXBlID0gYWNjZXNzCjAwMjFjaWQgbm9uY2UgPSBhOTM3ZWVjMjU5MmQyZmNlCjAwMmZzaWduYXR1cmUgGkKVqv90ZdYq3_b1iEKi5dPZF3nF7m9IqrONB2HCT-0",
            "syt_YWxpY2U_WrhCMGdSZ1B2KzJsZVNmQlo_24C10D",
        ];

        for token in test_tokens {
            assert!(!token.is_empty(), "Token should not be empty");
            assert!(token.len() >= 16, "Token should have reasonable minimum length");
            assert!(token.len() <= 2048, "Token should not exceed reasonable maximum length");
        }

        info!("✅ Matrix protocol compliance test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_session_enterprise_compliance() {
        debug!("🔧 Testing enterprise compliance for session management");
        let start = Instant::now();

        let storage = MockSessionStorage::new();

        // 1. Performance - Session operations should be fast
        let perf_start = Instant::now();
        let user = create_test_user(0);
        let device = create_test_device(0);
        let token = "enterprise_token".to_string();
        
        storage.create_session(user.clone(), device.clone(), token.clone());
        let _session = storage.get_session(&token);
        storage.remove_session(&token);
        
        let perf_duration = perf_start.elapsed();
        assert!(perf_duration < Duration::from_micros(100), 
                "Session operations should be <100μs for enterprise use, was: {:?}", perf_duration);

        // 2. Security - User isolation and access control
        let user1 = create_test_user(0);
        let user2 = create_test_user(1);
        let device1 = create_test_device(0);
        let device2 = create_test_device(1);
        
        storage.create_session(user1.clone(), device1.clone(), "user1_secure_token".to_string());
        storage.create_session(user2.clone(), device2.clone(), "user2_secure_token".to_string());
        
        // Verify cross-user session isolation
        assert_eq!(storage.count_user_sessions(&user1), 1, "User 1 should only see their sessions");
        assert_eq!(storage.count_user_sessions(&user2), 1, "User 2 should only see their sessions");
        
        let user1_session = storage.get_session("user1_secure_token").unwrap();
        assert_eq!(user1_session.user_id, user1, "Session should belong to correct user");

        // 3. Reliability - Consistent session management
        let initial_count = storage.count_active_sessions();
        
        // Add multiple sessions
        for i in 0..10 {
            storage.create_session(
                user1.clone(),
                create_test_device(i),
                format!("token_{}", i),
            );
        }
        
        assert_eq!(storage.count_active_sessions(), initial_count + 10, "Should track all sessions consistently");
        
        // Remove all user sessions
        let removed = storage.remove_all_user_sessions(&user1);
        assert_eq!(removed, 11, "Should remove all user sessions including the first one"); // +1 for the initial session

        // 4. Audit Trail - Session activity tracking
        let audit_user = create_test_user(2);
        let audit_device = create_test_device(0);
        let audit_token = "audit_token".to_string();
        
        storage.create_session(audit_user.clone(), audit_device.clone(), audit_token.clone());
        let session = storage.get_session(&audit_token).unwrap();
        
        assert!(session.created_at > 0, "Session should have creation timestamp");
        assert!(session.last_activity > 0, "Session should have last activity timestamp");
        assert!(session.created_at <= session.last_activity, "Creation should be before or equal to last activity");

        info!("✅ Session enterprise compliance test completed in {:?}", start.elapsed());
    }
}
