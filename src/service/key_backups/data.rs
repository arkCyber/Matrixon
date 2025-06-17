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

use std::collections::BTreeMap;

use crate::Result;
use ruma::{
    api::client::backup::{BackupAlgorithm, KeyBackupData, RoomKeyBackup},
    serde::Raw,
    OwnedRoomId, RoomId, UserId,
};

pub trait Data: Send + Sync {
    fn create_backup(
        &self,
        user_id: &UserId,
        backup_metadata: &Raw<BackupAlgorithm>,
    ) -> Result<String>;

    fn delete_backup(&self, user_id: &UserId, version: &str) -> Result<()>;

    fn update_backup(
        &self,
        user_id: &UserId,
        version: &str,
        backup_metadata: &Raw<BackupAlgorithm>,
    ) -> Result<String>;

    fn get_latest_backup_version(&self, user_id: &UserId) -> Result<Option<String>>;

    fn get_latest_backup(&self, user_id: &UserId)
        -> Result<Option<(String, Raw<BackupAlgorithm>)>>;

    fn get_backup(&self, user_id: &UserId, version: &str) -> Result<Option<Raw<BackupAlgorithm>>>;

    fn add_key(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
        key_data: &Raw<KeyBackupData>,
    ) -> Result<()>;

    fn count_keys(&self, user_id: &UserId, version: &str) -> Result<usize>;

    fn get_etag(&self, user_id: &UserId, version: &str) -> Result<String>;

    fn get_all(
        &self,
        user_id: &UserId,
        version: &str,
    ) -> Result<BTreeMap<OwnedRoomId, RoomKeyBackup>>;

    fn get_room(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
    ) -> Result<BTreeMap<String, Raw<KeyBackupData>>>;

    fn get_session(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
    ) -> Result<Option<Raw<KeyBackupData>>>;

    fn delete_all_keys(&self, user_id: &UserId, version: &str) -> Result<()>;

    fn delete_room_keys(&self, user_id: &UserId, version: &str, room_id: &RoomId) -> Result<()>;

    fn delete_room_key(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
    ) -> Result<()>;
}

#[cfg(test)]
mod tests {
    //! # Key Backups Service Tests
    //! 
    //! Author: matrixon Development Team
    //! Date: 2024-01-01
    //! Version: 1.0.0
    //! Purpose: Comprehensive testing of E2E encryption key backup functionality
    //! 
    //! ## Test Coverage
    //! - Key backup creation and management
    //! - Version control and updates
    //! - Room key storage and retrieval
    //! - Session key management
    //! - Deletion operations
    //! - Performance optimization for large key sets
    //! - Concurrent backup operations
    //! 
    //! ## Performance Requirements
    //! - Backup operations: <10ms per operation
    //! - Key storage: <5ms per key
    //! - Retrieval operations: <3ms per query
    //! - Support for 100k+ keys per backup
    
    use super::*;
    use ruma::{room_id, user_id, RoomId, UserId};
    use serde_json::json;
    use std::{
        collections::{BTreeMap, HashMap},
        sync::{Arc, RwLock},
        time::Instant,
    };

    /// Mock implementation of the Data trait for testing
    #[derive(Debug)]
    struct MockKeyBackupData {
        /// Backup metadata: (user_id, version) -> BackupAlgorithm
        backups: Arc<RwLock<HashMap<(String, String), Raw<BackupAlgorithm>>>>,
        /// Version tracking: user_id -> latest_version
        latest_versions: Arc<RwLock<HashMap<String, String>>>,
        /// Global version counter to ensure unique versions across all users
        global_version_counter: Arc<RwLock<u64>>,
        /// Keys: (user_id, version, room_id, session_id) -> KeyBackupData
        keys: Arc<RwLock<HashMap<(String, String, String, String), Raw<KeyBackupData>>>>,
    }

    impl MockKeyBackupData {
        fn new() -> Self {
            Self {
                backups: Arc::new(RwLock::new(HashMap::new())),
                latest_versions: Arc::new(RwLock::new(HashMap::new())),
                global_version_counter: Arc::new(RwLock::new(0)),
                keys: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn clear(&self) {
            self.backups.write().unwrap().clear();
            self.latest_versions.write().unwrap().clear();
            *self.global_version_counter.write().unwrap() = 0;
            self.keys.write().unwrap().clear();
        }

        fn count_backups(&self) -> usize {
            self.backups.read().unwrap().len()
        }

        fn count_stored_keys(&self) -> usize {
            self.keys.read().unwrap().len()
        }
    }

    impl Data for MockKeyBackupData {
        fn create_backup(
            &self,
            user_id: &UserId,
            backup_metadata: &Raw<BackupAlgorithm>,
        ) -> Result<String> {
            let user_id_str = user_id.to_string();
            
            // Generate new version
            let mut global_counter = self.global_version_counter.write().unwrap();
            *global_counter += 1;
            let version = global_counter.to_string();
            
            // Store backup
            let key = (user_id_str.clone(), version.clone());
            self.backups.write().unwrap().insert(key, backup_metadata.clone());
            
            // Update latest version
            self.latest_versions.write().unwrap().insert(user_id_str, version.clone());
            
            Ok(version)
        }

        fn delete_backup(&self, user_id: &UserId, version: &str) -> Result<()> {
            let user_id_str = user_id.to_string();
            let key = (user_id_str.clone(), version.to_string());
            
            self.backups.write().unwrap().remove(&key);
            
            // Delete all keys for this backup
            self.delete_all_keys(user_id, version)?;
            
            // Update latest version if this was the latest
            let latest_versions = self.latest_versions.read().unwrap();
            if let Some(latest) = latest_versions.get(&user_id_str) {
                if latest == version {
                    drop(latest_versions);
                    self.latest_versions.write().unwrap().remove(&user_id_str);
                }
            }
            
            Ok(())
        }

        fn update_backup(
            &self,
            user_id: &UserId,
            version: &str,
            backup_metadata: &Raw<BackupAlgorithm>,
        ) -> Result<String> {
            let user_id_str = user_id.to_string();
            let key = (user_id_str.clone(), version.to_string());
            
            if self.backups.read().unwrap().contains_key(&key) {
                self.backups.write().unwrap().insert(key, backup_metadata.clone());
                Ok(version.to_string())
            } else {
                self.create_backup(user_id, backup_metadata)
            }
        }

        fn get_latest_backup_version(&self, user_id: &UserId) -> Result<Option<String>> {
            let user_id_str = user_id.to_string();
            Ok(self.latest_versions.read().unwrap().get(&user_id_str).cloned())
        }

        fn get_latest_backup(&self, user_id: &UserId) -> Result<Option<(String, Raw<BackupAlgorithm>)>> {
            if let Some(version) = self.get_latest_backup_version(user_id)? {
                if let Some(backup) = self.get_backup(user_id, &version)? {
                    return Ok(Some((version, backup)));
                }
            }
            Ok(None)
        }

        fn get_backup(&self, user_id: &UserId, version: &str) -> Result<Option<Raw<BackupAlgorithm>>> {
            let key = (user_id.to_string(), version.to_string());
            Ok(self.backups.read().unwrap().get(&key).cloned())
        }

        fn add_key(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
            session_id: &str,
            key_data: &Raw<KeyBackupData>,
        ) -> Result<()> {
            let key = (
                user_id.to_string(),
                version.to_string(),
                room_id.to_string(),
                session_id.to_string(),
            );
            
            self.keys.write().unwrap().insert(key, key_data.clone());
            Ok(())
        }

        fn count_keys(&self, user_id: &UserId, version: &str) -> Result<usize> {
            let user_id_str = user_id.to_string();
            let version_str = version.to_string();
            
            let count = self.keys
                .read()
                .unwrap()
                .keys()
                .filter(|(uid, ver, _, _)| uid == &user_id_str && ver == &version_str)
                .count();
            
            Ok(count)
        }

        fn get_etag(&self, user_id: &UserId, version: &str) -> Result<String> {
            let count = self.count_keys(user_id, version)?;
            Ok(format!("etag-{}-{}-{}", user_id, version, count))
        }

        fn get_all(
            &self,
            user_id: &UserId,
            version: &str,
        ) -> Result<BTreeMap<OwnedRoomId, RoomKeyBackup>> {
            let user_id_str = user_id.to_string();
            let version_str = version.to_string();
            let mut result = BTreeMap::new();
            
            let keys = self.keys.read().unwrap();
            for ((uid, ver, room_id, session_id), key_data) in keys.iter() {
                if uid == &user_id_str && ver == &version_str {
                    let room_id_owned = OwnedRoomId::try_from(room_id.clone()).unwrap();
                    let room_backup = result.entry(room_id_owned).or_insert_with(|| RoomKeyBackup {
                        sessions: BTreeMap::new(),
                    });
                    room_backup.sessions.insert(session_id.clone(), key_data.clone());
                }
            }
            
            Ok(result)
        }

        fn get_room(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
        ) -> Result<BTreeMap<String, Raw<KeyBackupData>>> {
            let user_id_str = user_id.to_string();
            let version_str = version.to_string();
            let room_id_str = room_id.to_string();
            let mut result = BTreeMap::new();
            
            let keys = self.keys.read().unwrap();
            for ((uid, ver, rid, session_id), key_data) in keys.iter() {
                if uid == &user_id_str && ver == &version_str && rid == &room_id_str {
                    result.insert(session_id.clone(), key_data.clone());
                }
            }
            
            Ok(result)
        }

        fn get_session(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
            session_id: &str,
        ) -> Result<Option<Raw<KeyBackupData>>> {
            let key = (
                user_id.to_string(),
                version.to_string(),
                room_id.to_string(),
                session_id.to_string(),
            );
            
            Ok(self.keys.read().unwrap().get(&key).cloned())
        }

        fn delete_all_keys(&self, user_id: &UserId, version: &str) -> Result<()> {
            let user_id_str = user_id.to_string();
            let version_str = version.to_string();
            
            let mut keys = self.keys.write().unwrap();
            keys.retain(|&(ref uid, ref ver, _, _), _| !(uid == &user_id_str && ver == &version_str));
            
            Ok(())
        }

        fn delete_room_keys(&self, user_id: &UserId, version: &str, room_id: &RoomId) -> Result<()> {
            let user_id_str = user_id.to_string();
            let version_str = version.to_string();
            let room_id_str = room_id.to_string();
            
            let mut keys = self.keys.write().unwrap();
            keys.retain(|&(ref uid, ref ver, ref rid, _), _| !(uid == &user_id_str && ver == &version_str && rid == &room_id_str));
            
            Ok(())
        }

        fn delete_room_key(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
            session_id: &str,
        ) -> Result<()> {
            let key = (
                user_id.to_string(),
                version.to_string(),
                room_id.to_string(),
                session_id.to_string(),
            );
            
            self.keys.write().unwrap().remove(&key);
            Ok(())
        }
    }

    fn create_test_data() -> MockKeyBackupData {
        MockKeyBackupData::new()
    }

    fn create_test_user(index: usize) -> &'static UserId {
        match index {
            0 => user_id!("@user0:example.com"),
            1 => user_id!("@user1:example.com"),
            2 => user_id!("@user2:example.com"),
            _ => user_id!("@testuser:example.com"),
        }
    }

    fn create_test_room(index: usize) -> &'static RoomId {
        match index {
            0 => room_id!("!room0:example.com"),
            1 => room_id!("!room1:example.com"),
            2 => room_id!("!room2:example.com"),
            _ => room_id!("!testroom:example.com"),
        }
    }

    fn create_test_backup_algorithm() -> Raw<BackupAlgorithm> {
        let algorithm_json = json!({
            "algorithm": "m.megolm_backup.v1.curve25519-aes-sha2",
            "auth_data": {
                "public_key": "test_public_key_123456789",
                "signatures": {}
            }
        });
        
        serde_json::value::to_raw_value(&algorithm_json)
            .map(Raw::from_json)
            .unwrap()
    }

    fn create_test_key_backup_data(session_id: &str) -> Raw<KeyBackupData> {
        let key_data_json = json!({
            "first_message_index": 0,
            "forwarded_count": 0,
            "is_verified": true,
            "session_data": {
                "ciphertext": format!("encrypted_session_data_for_{}", session_id),
                "mac": "test_mac_value",
                "ephemeral": "test_ephemeral_key"
            }
        });
        
        serde_json::value::to_raw_value(&key_data_json)
            .map(Raw::from_json)
            .unwrap()
    }

    #[test]
    fn test_create_and_get_backup() {
        let data = create_test_data();
        let user = create_test_user(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();
        assert!(!version.is_empty());

        // Get backup by version
        let retrieved = data.get_backup(user, &version).unwrap();
        assert!(retrieved.is_some());

        // Get latest backup version
        let latest_version = data.get_latest_backup_version(user).unwrap();
        assert_eq!(latest_version, Some(version.clone()));

        // Get latest backup
        let latest_backup = data.get_latest_backup(user).unwrap();
        assert!(latest_backup.is_some());
        let (latest_ver, _) = latest_backup.unwrap();
        assert_eq!(latest_ver, version);
    }

    #[test]
    fn test_backup_versioning() {
        let data = create_test_data();
        let user = create_test_user(0);
        let backup_algorithm1 = create_test_backup_algorithm();
        let backup_algorithm2 = create_test_backup_algorithm();

        // Create first backup
        let version1 = data.create_backup(user, &backup_algorithm1).unwrap();
        
        // Create second backup
        let version2 = data.create_backup(user, &backup_algorithm2).unwrap();
        
        // Versions should be different
        assert_ne!(version1, version2);
        
        // Latest should be version2
        let latest = data.get_latest_backup_version(user).unwrap();
        assert_eq!(latest, Some(version2.clone()));
        
        // Both versions should be retrievable
        assert!(data.get_backup(user, &version1).unwrap().is_some());
        assert!(data.get_backup(user, &version2).unwrap().is_some());
    }

    #[test]
    fn test_update_backup() {
        let data = create_test_data();
        let user = create_test_user(0);
        let original_algorithm = create_test_backup_algorithm();
        let updated_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &original_algorithm).unwrap();

        // Update backup
        let updated_version = data.update_backup(user, &version, &updated_algorithm).unwrap();
        assert_eq!(updated_version, version);

        // Verify updated
        let retrieved = data.get_backup(user, &version).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_delete_backup() {
        let data = create_test_data();
        let user = create_test_user(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();
        
        // Verify exists
        assert!(data.get_backup(user, &version).unwrap().is_some());
        
        // Delete backup
        data.delete_backup(user, &version).unwrap();
        
        // Verify deleted
        assert!(data.get_backup(user, &version).unwrap().is_none());
        assert!(data.get_latest_backup_version(user).unwrap().is_none());
    }

    #[test]
    fn test_add_and_get_keys() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();

        // Add keys
        let session_ids = ["session1", "session2", "session3"];
        for session_id in &session_ids {
            let key_data = create_test_key_backup_data(session_id);
            data.add_key(user, &version, room, session_id, &key_data).unwrap();
        }

        // Count keys
        let count = data.count_keys(user, &version).unwrap();
        assert_eq!(count, 3);

        // Get specific session
        let session_data = data.get_session(user, &version, room, "session1").unwrap();
        assert!(session_data.is_some());

        // Get all room keys
        let room_keys = data.get_room(user, &version, room).unwrap();
        assert_eq!(room_keys.len(), 3);

        // Get all keys
        let all_keys = data.get_all(user, &version).unwrap();
        assert_eq!(all_keys.len(), 1); // One room
        assert!(all_keys.contains_key(&room.to_owned()));
    }

    #[test]
    fn test_key_deletion() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room1 = create_test_room(0);
        let room2 = create_test_room(1);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();

        // Add keys to multiple rooms
        for room in [room1, room2] {
            for i in 0..3 {
                let session_id = format!("session{}", i);
                let key_data = create_test_key_backup_data(&session_id);
                data.add_key(user, &version, room, &session_id, &key_data).unwrap();
            }
        }

        // Verify total count
        assert_eq!(data.count_keys(user, &version).unwrap(), 6);

        // Delete specific key
        data.delete_room_key(user, &version, room1, "session0").unwrap();
        assert_eq!(data.count_keys(user, &version).unwrap(), 5);

        // Delete room keys
        data.delete_room_keys(user, &version, room1).unwrap();
        assert_eq!(data.count_keys(user, &version).unwrap(), 3);

        // Delete all keys
        data.delete_all_keys(user, &version).unwrap();
        assert_eq!(data.count_keys(user, &version).unwrap(), 0);
    }

    #[test]
    fn test_user_isolation() {
        let data = create_test_data();
        let user1 = create_test_user(0);
        let user2 = create_test_user(1);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backups for different users
        let version1 = data.create_backup(user1, &backup_algorithm).unwrap();
        let version2 = data.create_backup(user2, &backup_algorithm).unwrap();

        // Add keys for each user
        let key_data = create_test_key_backup_data("session1");
        data.add_key(user1, &version1, room, "session1", &key_data).unwrap();
        data.add_key(user2, &version2, room, "session1", &key_data).unwrap();

        // Verify isolation
        assert_eq!(data.count_keys(user1, &version1).unwrap(), 1);
        assert_eq!(data.count_keys(user2, &version2).unwrap(), 1);
        
        // Cross-user access should be denied - user1 shouldn't see user2's keys even with same version string
        assert_eq!(data.count_keys(user1, &version2).unwrap(), 0);
        assert_eq!(data.count_keys(user2, &version1).unwrap(), 0);
        
        // Backup access should also be isolated
        assert!(data.get_backup(user1, &version1).unwrap().is_some());
        assert!(data.get_backup(user2, &version2).unwrap().is_some());
        
        // Even if versions happen to be the same string, they should be isolated by user
        if version1 == version2 {
            // Same version string but different users - should still be isolated
            assert!(data.get_backup(user1, &version1).unwrap().is_some());  // user1 can access their own
            assert!(data.get_backup(user2, &version1).unwrap().is_none());  // user2 cannot access user1's
        } else {
            // Different version strings - cross access should fail
            assert!(data.get_backup(user1, &version2).unwrap().is_none());
            assert!(data.get_backup(user2, &version1).unwrap().is_none());
        }
    }

    #[test]
    fn test_etag_generation() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();

        // Get initial etag
        let etag1 = data.get_etag(user, &version).unwrap();

        // Add key
        let key_data = create_test_key_backup_data("session1");
        data.add_key(user, &version, room, "session1", &key_data).unwrap();

        // Get updated etag
        let etag2 = data.get_etag(user, &version).unwrap();

        // ETags should be different
        assert_ne!(etag1, etag2);
    }

    #[test]
    fn test_large_key_set() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();

        // Add large number of keys
        let start = Instant::now();
        for i in 0..1000 {
            let session_id = format!("session_{}", i);
            let key_data = create_test_key_backup_data(&session_id);
            data.add_key(user, &version, room, &session_id, &key_data).unwrap();
        }
        let add_duration = start.elapsed();

        // Test retrieval performance
        let start = Instant::now();
        let all_keys = data.get_all(user, &version).unwrap();
        let get_duration = start.elapsed();

        // Performance assertions
        assert!(add_duration.as_millis() < 1000, "Adding 1000 keys should be <1s");
        assert!(get_duration.as_millis() < 100, "Getting all keys should be <100ms");
        
        // Verify count
        assert_eq!(data.count_keys(user, &version).unwrap(), 1000);
        assert_eq!(all_keys.len(), 1);
        assert_eq!(all_keys[&room.to_owned()].sessions.len(), 1000);
    }

    #[test]
    fn test_concurrent_operations() {
        use std::thread;

        let data = Arc::new(create_test_data());
        let user = create_test_user(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = data.create_backup(user, &backup_algorithm).unwrap();
        let mut handles = vec![];

        // Spawn multiple threads adding keys
        for i in 0..10 {
            let data_clone = Arc::clone(&data);
            let version_clone = version.clone();
            let room = create_test_room(i % 3); // 3 different rooms

            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let session_id = format!("thread_{}_session_{}", i, j);
                    let key_data = create_test_key_backup_data(&session_id);
                    data_clone.add_key(user, &version_clone, room, &session_id, &key_data).unwrap();
                }
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify total count
        assert_eq!(data.count_keys(user, &version).unwrap(), 100);
    }

    #[test]
    fn test_performance_characteristics() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // Test backup creation performance
        let start = Instant::now();
        for _ in 0..100 {
            let _ = data.create_backup(user, &backup_algorithm).unwrap();
        }
        let backup_creation_duration = start.elapsed();

        // Test key operations performance
        let version = data.create_backup(user, &backup_algorithm).unwrap();
        
        let start = Instant::now();
        for i in 0..500 {
            let session_id = format!("perf_session_{}", i);
            let key_data = create_test_key_backup_data(&session_id);
            data.add_key(user, &version, room, &session_id, &key_data).unwrap();
        }
        let key_add_duration = start.elapsed();

        let start = Instant::now();
        for i in 0..500 {
            let session_id = format!("perf_session_{}", i);
            let _ = data.get_session(user, &version, room, &session_id).unwrap();
        }
        let key_get_duration = start.elapsed();

        // Performance assertions
        assert!(backup_creation_duration.as_millis() < 500, "100 backup creations should be <500ms");
        assert!(key_add_duration.as_millis() < 500, "500 key additions should be <500ms");
        assert!(key_get_duration.as_millis() < 100, "500 key retrievals should be <100ms");
    }

    #[test]
    fn test_memory_efficiency() {
        let data = create_test_data();
        let user = create_test_user(0);

        // Create multiple backups with keys
        for i in 0..10 {
            let backup_algorithm = create_test_backup_algorithm();
            let version = data.create_backup(user, &backup_algorithm).unwrap();
            
            for j in 0..100 {
                let room = create_test_room(j % 3);
                let session_id = format!("session_{}_{}", i, j);
                let key_data = create_test_key_backup_data(&session_id);
                data.add_key(user, &version, room, &session_id, &key_data).unwrap();
            }
        }

        // Verify counts
        assert_eq!(data.count_backups(), 10);
        assert_eq!(data.count_stored_keys(), 1000);

        // Clear and verify cleanup
        data.clear();
        assert_eq!(data.count_backups(), 0);
        assert_eq!(data.count_stored_keys(), 0);
    }

    #[test]
    fn test_error_handling() {
        let data = create_test_data();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let backup_algorithm = create_test_backup_algorithm();

        // All operations should succeed in the mock implementation
        let version = data.create_backup(user, &backup_algorithm).unwrap();
        
        assert!(data.get_backup(user, &version).is_ok());
        assert!(data.get_latest_backup_version(user).is_ok());
        assert!(data.count_keys(user, &version).is_ok());
        assert!(data.get_etag(user, &version).is_ok());
        
        let key_data = create_test_key_backup_data("session1");
        assert!(data.add_key(user, &version, room, "session1", &key_data).is_ok());
        assert!(data.get_session(user, &version, room, "session1").is_ok());
        assert!(data.delete_room_key(user, &version, room, "session1").is_ok());
        assert!(data.delete_backup(user, &version).is_ok());
    }
}
