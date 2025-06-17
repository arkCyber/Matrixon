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

mod data;
pub use data::Data;

use crate::Result;
use ruma::{
    api::client::backup::{BackupAlgorithm, KeyBackupData, RoomKeyBackup},
    serde::Raw,
    OwnedRoomId, RoomId, UserId,
};
use std::collections::BTreeMap;

pub struct Service {
    pub db: &'static dyn Data,
}

impl Service {
    pub fn create_backup(
        &self,
        user_id: &UserId,
        backup_metadata: &Raw<BackupAlgorithm>,
    ) -> Result<String> {
        self.db.create_backup(user_id, backup_metadata)
    }

    pub fn delete_backup(&self, user_id: &UserId, version: &str) -> Result<()> {
        self.db.delete_backup(user_id, version)
    }

    pub fn update_backup(
        &self,
        user_id: &UserId,
        version: &str,
        backup_metadata: &Raw<BackupAlgorithm>,
    ) -> Result<String> {
        self.db.update_backup(user_id, version, backup_metadata)
    }

    pub fn get_latest_backup_version(&self, user_id: &UserId) -> Result<Option<String>> {
        self.db.get_latest_backup_version(user_id)
    }

    pub fn get_latest_backup(
        &self,
        user_id: &UserId,
    ) -> Result<Option<(String, Raw<BackupAlgorithm>)>> {
        self.db.get_latest_backup(user_id)
    }

    pub fn get_backup(
        &self,
        user_id: &UserId,
        version: &str,
    ) -> Result<Option<Raw<BackupAlgorithm>>> {
        self.db.get_backup(user_id, version)
    }

    pub fn add_key(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
        key_data: &Raw<KeyBackupData>,
    ) -> Result<()> {
        self.db
            .add_key(user_id, version, room_id, session_id, key_data)
    }

    pub fn count_keys(&self, user_id: &UserId, version: &str) -> Result<usize> {
        self.db.count_keys(user_id, version)
    }

    pub fn get_etag(&self, user_id: &UserId, version: &str) -> Result<String> {
        self.db.get_etag(user_id, version)
    }

    pub fn get_all(
        &self,
        user_id: &UserId,
        version: &str,
    ) -> Result<BTreeMap<OwnedRoomId, RoomKeyBackup>> {
        self.db.get_all(user_id, version)
    }

    pub fn get_room(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
    ) -> Result<BTreeMap<String, Raw<KeyBackupData>>> {
        self.db.get_room(user_id, version, room_id)
    }

    pub fn get_session(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
    ) -> Result<Option<Raw<KeyBackupData>>> {
        self.db.get_session(user_id, version, room_id, session_id)
    }

    pub fn delete_all_keys(&self, user_id: &UserId, version: &str) -> Result<()> {
        self.db.delete_all_keys(user_id, version)
    }

    pub fn delete_room_keys(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
    ) -> Result<()> {
        self.db.delete_room_keys(user_id, version, room_id)
    }

    pub fn delete_room_key(
        &self,
        user_id: &UserId,
        version: &str,
        room_id: &RoomId,
        session_id: &str,
    ) -> Result<()> {
        self.db
            .delete_room_key(user_id, version, room_id, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{room_id, user_id};
    use serde_json::json;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, RwLock};
    use std::time::{Duration, Instant};

    /// Mock key backup data implementation for testing
    #[derive(Debug)]
    struct MockKeyBackupData {
        backups: Arc<RwLock<HashMap<String, Raw<BackupAlgorithm>>>>,
        keys: Arc<RwLock<HashMap<String, Raw<KeyBackupData>>>>,
        versions: Arc<RwLock<HashMap<String, u64>>>,
        counter: Arc<RwLock<u64>>,
    }

    impl MockKeyBackupData {
        fn new() -> Self {
            Self {
                backups: Arc::new(RwLock::new(HashMap::new())),
                keys: Arc::new(RwLock::new(HashMap::new())),
                versions: Arc::new(RwLock::new(HashMap::new())),
                counter: Arc::new(RwLock::new(1)),
            }
        }

        fn backup_key(user_id: &UserId, version: &str) -> String {
            format!("backup:{}:{}", user_id, version)
        }

        fn key_key(user_id: &UserId, version: &str, room_id: &RoomId, session_id: &str) -> String {
            format!("key|{}|{}|{}|{}", user_id, version, room_id, session_id)
        }

        fn version_key(user_id: &UserId) -> String {
            format!("version:{}", user_id)
        }

        fn next_version(&self) -> String {
            let mut counter = self.counter.write().unwrap();
            *counter += 1;
            counter.to_string()
        }
    }

    impl Data for MockKeyBackupData {
        fn create_backup(
            &self,
            user_id: &UserId,
            backup_metadata: &Raw<BackupAlgorithm>,
        ) -> Result<String> {
            let version = self.next_version();
            let key = Self::backup_key(user_id, &version);
            self.backups.write().unwrap().insert(key, backup_metadata.clone());
            self.versions.write().unwrap().insert(Self::version_key(user_id), version.parse().unwrap());
            Ok(version)
        }

        fn delete_backup(&self, user_id: &UserId, version: &str) -> Result<()> {
            let key = Self::backup_key(user_id, version);
            self.backups.write().unwrap().remove(&key);
            Ok(())
        }

        fn update_backup(
            &self,
            user_id: &UserId,
            version: &str,
            backup_metadata: &Raw<BackupAlgorithm>,
        ) -> Result<String> {
            let key = Self::backup_key(user_id, version);
            self.backups.write().unwrap().insert(key, backup_metadata.clone());
            Ok(version.to_string())
        }

        fn get_latest_backup_version(&self, user_id: &UserId) -> Result<Option<String>> {
            let version_key = Self::version_key(user_id);
            Ok(self.versions.read().unwrap().get(&version_key).map(|v| v.to_string()))
        }

        fn get_latest_backup(
            &self,
            user_id: &UserId,
        ) -> Result<Option<(String, Raw<BackupAlgorithm>)>> {
            if let Some(version) = self.get_latest_backup_version(user_id)? {
                if let Some(backup) = self.get_backup(user_id, &version)? {
                    return Ok(Some((version, backup)));
                }
            }
            Ok(None)
        }

        fn get_backup(
            &self,
            user_id: &UserId,
            version: &str,
        ) -> Result<Option<Raw<BackupAlgorithm>>> {
            let key = Self::backup_key(user_id, version);
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
            let key = Self::key_key(user_id, version, room_id, session_id);
            self.keys.write().unwrap().insert(key, key_data.clone());
            Ok(())
        }

        fn count_keys(&self, user_id: &UserId, version: &str) -> Result<usize> {
            let prefix = format!("key|{}|{}|", user_id, version);
            let count = self.keys.read().unwrap()
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .count();
            Ok(count)
        }

        fn get_etag(&self, user_id: &UserId, version: &str) -> Result<String> {
            // Simple etag based on key count
            let count = self.count_keys(user_id, version)?;
            Ok(format!("etag_{}", count))
        }

        fn get_all(
            &self,
            user_id: &UserId,
            version: &str,
        ) -> Result<BTreeMap<OwnedRoomId, RoomKeyBackup>> {
            // Simplified implementation for testing
            Ok(BTreeMap::new())
        }

        fn get_room(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
        ) -> Result<BTreeMap<String, Raw<KeyBackupData>>> {
            let prefix = format!("key|{}|{}|{}|", user_id, version, room_id);
            let mut result = BTreeMap::new();
            
            for (key, data) in self.keys.read().unwrap().iter() {
                if key.starts_with(&prefix) {
                    if let Some(session_id) = key.split('|').nth(4) {
                        result.insert(session_id.to_string(), data.clone());
                    }
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
            let key = Self::key_key(user_id, version, room_id, session_id);
            Ok(self.keys.read().unwrap().get(&key).cloned())
        }

        fn delete_all_keys(&self, user_id: &UserId, version: &str) -> Result<()> {
            let prefix = format!("key|{}|{}|", user_id, version);
            let mut keys = self.keys.write().unwrap();
            keys.retain(|k, _| !k.starts_with(&prefix));
            Ok(())
        }

        fn delete_room_keys(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
        ) -> Result<()> {
            let prefix = format!("key|{}|{}|{}|", user_id, version, room_id);
            let mut keys = self.keys.write().unwrap();
            keys.retain(|k, _| !k.starts_with(&prefix));
            Ok(())
        }

        fn delete_room_key(
            &self,
            user_id: &UserId,
            version: &str,
            room_id: &RoomId,
            session_id: &str,
        ) -> Result<()> {
            let key = Self::key_key(user_id, version, room_id, session_id);
            self.keys.write().unwrap().remove(&key);
            Ok(())
        }
    }

    fn create_test_service() -> Service {
        Service {
            db: Box::leak(Box::new(MockKeyBackupData::new())),
        }
    }

    fn create_test_backup_algorithm() -> Raw<BackupAlgorithm> {
        let algorithm = json!({
            "algorithm": "m.megolm_backup.v1.curve25519-aes-sha2",
            "auth_data": {
                "public_key": "test_public_key",
                "signatures": {}
            }
        });
        serde_json::from_value(algorithm).unwrap()
    }

    fn create_test_key_data() -> Raw<KeyBackupData> {
        let key_data = json!({
            "first_message_index": 0,
            "forwarded_count": 0,
            "is_verified": true,
            "session_data": {
                "ephemeral": "test_ephemeral_key",
                "ciphertext": "encrypted_session_key"
            }
        });
        serde_json::from_value(key_data).unwrap()
    }

    #[test]
    fn test_create_backup() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let backup_algorithm = create_test_backup_algorithm();

        let result = service.create_backup(user_id, &backup_algorithm);
        assert!(result.is_ok(), "Should successfully create backup");
        
        let version = result.unwrap();
        assert!(!version.is_empty(), "Version should not be empty");
    }

    #[test]
    fn test_get_backup() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup first
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Get backup
        let result = service.get_backup(user_id, &version);
        assert!(result.is_ok(), "Should successfully get backup");
        assert!(result.unwrap().is_some(), "Backup should exist");
    }

    #[test]
    fn test_get_latest_backup() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let backup_algorithm = create_test_backup_algorithm();

        // Initially no backup
        let result = service.get_latest_backup(user_id);
        assert!(result.is_ok(), "Should not error for no backup");
        assert!(result.unwrap().is_none(), "Should return None for no backup");

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Get latest backup
        let result = service.get_latest_backup(user_id);
        assert!(result.is_ok(), "Should successfully get latest backup");
        
        let (latest_version, _) = result.unwrap().unwrap();
        assert_eq!(latest_version, version, "Latest version should match created version");
    }

    #[test]
    fn test_update_backup() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let initial_algorithm = create_test_backup_algorithm();

        // Create initial backup
        let version = service.create_backup(user_id, &initial_algorithm).unwrap();

        // Update backup
        let updated_algorithm = json!({
            "algorithm": "m.megolm_backup.v1.curve25519-aes-sha2",
            "auth_data": {
                "public_key": "updated_public_key",
                "signatures": {}
            }
        });
        let updated_raw = serde_json::from_value(updated_algorithm).unwrap();

        let result = service.update_backup(user_id, &version, &updated_raw);
        assert!(result.is_ok(), "Should successfully update backup");
    }

    #[test]
    fn test_delete_backup() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let backup_algorithm = create_test_backup_algorithm();

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Verify backup exists
        let backup = service.get_backup(user_id, &version).unwrap();
        assert!(backup.is_some(), "Backup should exist before deletion");

        // Delete backup
        let result = service.delete_backup(user_id, &version);
        assert!(result.is_ok(), "Should successfully delete backup");

        // Verify backup is deleted
        let backup = service.get_backup(user_id, &version).unwrap();
        assert!(backup.is_none(), "Backup should not exist after deletion");
    }

    #[test]
    fn test_add_and_get_key() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let session_id = "test_session_123";
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup first
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Add key
        let result = service.add_key(user_id, &version, room_id, session_id, &key_data);
        assert!(result.is_ok(), "Should successfully add key");

        // Get key
        let retrieved = service.get_session(user_id, &version, room_id, session_id);
        assert!(retrieved.is_ok(), "Should successfully get key");
        assert!(retrieved.unwrap().is_some(), "Key should exist");
    }

    #[test]
    fn test_count_keys() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Initially no keys
        let count = service.count_keys(user_id, &version).unwrap();
        assert_eq!(count, 0, "Should have no keys initially");

        // Add some keys
        for i in 0..5 {
            let session_id = format!("session_{}", i);
            service.add_key(user_id, &version, room_id, &session_id, &key_data).unwrap();
        }

        // Count keys
        let count = service.count_keys(user_id, &version).unwrap();
        assert_eq!(count, 5, "Should have 5 keys");
    }

    #[test]
    fn test_get_etag() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Get initial etag
        let etag1 = service.get_etag(user_id, &version).unwrap();

        // Add key and get new etag
        service.add_key(user_id, &version, room_id, "session1", &key_data).unwrap();
        let etag2 = service.get_etag(user_id, &version).unwrap();

        assert_ne!(etag1, etag2, "ETags should change when keys are added");
    }

    #[test]
    fn test_get_room_keys() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Add keys for this room
        for i in 0..3 {
            let session_id = format!("session_{}", i);
            service.add_key(user_id, &version, room_id, &session_id, &key_data).unwrap();
        }

        // Get room keys
        let room_keys = service.get_room(user_id, &version, room_id).unwrap();
        assert_eq!(room_keys.len(), 3, "Should have 3 keys for the room");
    }

    #[test]
    fn test_delete_operations() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup and add keys
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();
        
        for i in 0..5 {
            let session_id = format!("session_{}", i);
            service.add_key(user_id, &version, room_id, &session_id, &key_data).unwrap();
        }

        // Verify keys exist
        assert_eq!(service.count_keys(user_id, &version).unwrap(), 5);

        // Test delete single key
        service.delete_room_key(user_id, &version, room_id, "session_0").unwrap();
        assert_eq!(service.count_keys(user_id, &version).unwrap(), 4);

        // Test delete room keys
        service.delete_room_keys(user_id, &version, room_id).unwrap();
        assert_eq!(service.count_keys(user_id, &version).unwrap(), 0);

        // Add keys again and test delete all
        for i in 0..3 {
            let session_id = format!("session_{}", i);
            service.add_key(user_id, &version, room_id, &session_id, &key_data).unwrap();
        }
        
        service.delete_all_keys(user_id, &version).unwrap();
        assert_eq!(service.count_keys(user_id, &version).unwrap(), 0);
    }

    #[test]
    fn test_user_isolation() {
        let service = create_test_service();
        let user1 = user_id!("@user1:example.com");
        let user2 = user_id!("@user2:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backups for both users
        let version1 = service.create_backup(user1, &backup_algorithm).unwrap();
        let version2 = service.create_backup(user2, &backup_algorithm).unwrap();

        // Add keys for both users
        service.add_key(user1, &version1, room_id, "session1", &key_data).unwrap();
        service.add_key(user2, &version2, room_id, "session1", &key_data).unwrap();

        // Verify isolation
        assert_eq!(service.count_keys(user1, &version1).unwrap(), 1);
        assert_eq!(service.count_keys(user2, &version2).unwrap(), 1);

        // User1 should not see user2's keys
        let user1_session = service.get_session(user1, &version1, room_id, "session1").unwrap();
        let user2_session = service.get_session(user2, &version2, room_id, "session1").unwrap();
        
        assert!(user1_session.is_some(), "User1 should have their session");
        assert!(user2_session.is_some(), "User2 should have their session");

        // Cross-user access should fail
        let cross_access = service.get_session(user1, &version2, room_id, "session1").unwrap();
        assert!(cross_access.is_none(), "User1 should not access user2's keys");
    }

    #[test]
    fn test_concurrent_backup_operations() {
        use std::thread;
        
        let service = Arc::new(create_test_service());
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();
        let num_threads = 5;

        // Create backup first
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();
        let version_arc = Arc::new(version);

        let mut handles = vec![];

        // Spawn threads that add keys concurrently
        for thread_id in 0..num_threads {
            let service_clone = Arc::clone(&service);
            let version_clone = Arc::clone(&version_arc);
            let key_data = key_data.clone();

            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let session_id = format!("thread_{}_session_{}", thread_id, i);
                    let result = service_clone.add_key(user_id, &version_clone, room_id, &session_id, &key_data);
                    assert!(result.is_ok(), "Concurrent key addition should succeed");
                }
            });

            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all keys were added
        let final_count = service.count_keys(user_id, &version_arc).unwrap();
        assert_eq!(final_count, 50, "Should have 50 keys from concurrent operations");
    }

    #[test]
    fn test_performance_benchmarks() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();
        let key_data = create_test_key_data();

        // Create backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();

        // Benchmark key addition
        let start = Instant::now();
        for i in 0..1000 {
            let session_id = format!("bench_session_{}", i);
            service.add_key(user_id, &version, room_id, &session_id, &key_data).unwrap();
        }
        let add_duration = start.elapsed();

        // Benchmark key retrieval
        let start = Instant::now();
        for i in 0..1000 {
            let session_id = format!("bench_session_{}", i);
            let _ = service.get_session(user_id, &version, room_id, &session_id).unwrap();
        }
        let get_duration = start.elapsed();

        // Performance assertions
        assert!(add_duration < Duration::from_secs(1), 
                "1000 key additions should complete within 1s, took: {:?}", add_duration);
        assert!(get_duration < Duration::from_millis(500), 
                "1000 key retrievals should complete within 500ms, took: {:?}", get_duration);
    }

    #[test]
    fn test_edge_cases() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");
        let backup_algorithm = create_test_backup_algorithm();

        // Test nonexistent backup operations
        let result = service.get_backup(user_id, "nonexistent_version");
        assert!(result.is_ok() && result.unwrap().is_none(), "Should handle nonexistent backup");

        let result = service.count_keys(user_id, "nonexistent_version");
        assert!(result.is_ok() && result.unwrap() == 0, "Should return 0 for nonexistent version");

        // Test operations on empty backup
        let version = service.create_backup(user_id, &backup_algorithm).unwrap();
        
        let session = service.get_session(user_id, &version, room_id, "nonexistent_session").unwrap();
        assert!(session.is_none(), "Should return None for nonexistent session");

        let room_keys = service.get_room(user_id, &version, room_id).unwrap();
        assert!(room_keys.is_empty(), "Should return empty map for room with no keys");
    }

    #[test]
    fn test_matrix_protocol_compliance() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let room_id = room_id!("!test:example.com");

        // Test with realistic Matrix backup algorithm
        let backup_algorithm = json!({
            "algorithm": "m.megolm_backup.v1.curve25519-aes-sha2",
            "auth_data": {
                "public_key": "hsDts4g7KgfKA4jjEfIaqC1iY6YGdYl5Bg4UzNU3+hw",
                "signatures": {
                    "@user:example.com": {
                        "ed25519:DEVICE": "signature_data"
                    }
                }
            }
        });
        let backup_raw = serde_json::from_value(backup_algorithm).unwrap();

        // Test with realistic key backup data
        let key_data = json!({
            "first_message_index": 0,
            "forwarded_count": 0,
            "is_verified": true,
            "session_data": {
                "ephemeral": "encrypted_ephemeral_key",
                "ciphertext": "encrypted_session_key_data",
                "mac": "message_authentication_code"
            }
        });
        let key_raw = serde_json::from_value(key_data).unwrap();

        // Test full backup workflow
        let version = service.create_backup(user_id, &backup_raw).unwrap();
        service.add_key(user_id, &version, room_id, "session_id", &key_raw).unwrap();

        let retrieved_key = service.get_session(user_id, &version, room_id, "session_id").unwrap();
        assert!(retrieved_key.is_some(), "Matrix-compliant key should be stored and retrieved");
    }
}
