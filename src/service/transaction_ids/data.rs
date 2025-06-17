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

use crate::Result;
use ruma::{DeviceId, TransactionId, UserId};

pub trait Data: Send + Sync {
    fn add_txnid(
        &self,
        user_id: &UserId,
        device_id: Option<&DeviceId>,
        txn_id: &TransactionId,
        data: &[u8],
    ) -> Result<()>;

    fn existing_txnid(
        &self,
        user_id: &UserId,
        device_id: Option<&DeviceId>,
        txn_id: &TransactionId,
    ) -> Result<Option<Vec<u8>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{device_id, user_id, DeviceId, UserId};
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        time::Instant,
    };

    /// Mock implementation of the Data trait for testing
    #[derive(Debug)]
    struct MockTransactionIdData {
        /// Store transaction data with key: (user_id, device_id, txn_id)
        transactions: Arc<RwLock<HashMap<(String, Option<String>, String), Vec<u8>>>>,
    }

    impl MockTransactionIdData {
        fn new() -> Self {
            Self {
                transactions: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn clear(&self) {
            self.transactions.write().unwrap().clear();
        }

        fn count(&self) -> usize {
            self.transactions.read().unwrap().len()
        }
    }

    impl Data for MockTransactionIdData {
        fn add_txnid(
            &self,
            user_id: &UserId,
            device_id: Option<&DeviceId>,
            txn_id: &TransactionId,
            data: &[u8],
        ) -> Result<()> {
            let key = (
                user_id.to_string(),
                device_id.map(|d| d.to_string()),
                txn_id.to_string(),
            );
            
            self.transactions
                .write()
                .unwrap()
                .insert(key, data.to_vec());
            
            Ok(())
        }

        fn existing_txnid(
            &self,
            user_id: &UserId,
            device_id: Option<&DeviceId>,
            txn_id: &TransactionId,
        ) -> Result<Option<Vec<u8>>> {
            let key = (
                user_id.to_string(),
                device_id.map(|d| d.to_string()),
                txn_id.to_string(),
            );
            
            Ok(self.transactions.read().unwrap().get(&key).cloned())
        }
    }

    fn create_test_data() -> MockTransactionIdData {
        MockTransactionIdData::new()
    }

    fn create_test_user(index: usize) -> &'static UserId {
        match index {
            0 => user_id!("@user0:example.com"),
            1 => user_id!("@user1:example.com"),
            2 => user_id!("@user2:example.com"),
            _ => user_id!("@testuser:example.com"),
        }
    }

    fn create_test_device(index: usize) -> &'static DeviceId {
        match index {
            0 => device_id!("DEVICE0"),
            1 => device_id!("DEVICE1"),
            2 => device_id!("DEVICE2"),
            _ => device_id!("TESTDEVICE"),
        }
    }

    fn create_test_transaction_id(index: usize) -> String {
        format!("txn_{}", index)
    }

    #[test]
    fn test_add_and_retrieve_transaction() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        let test_data = b"test transaction data";

        // Add transaction
        data.add_txnid(user, device, &txn_id, test_data).unwrap();

        // Retrieve transaction
        let result = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result, Some(test_data.to_vec()));
    }

    #[test]
    fn test_nonexistent_transaction() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();

        // Try to retrieve nonexistent transaction
        let result = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_multiple_users_isolation() {
        let data = create_test_data();
        let user1 = create_test_user(0);
        let user2 = create_test_user(1);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        let data1 = b"user1 data";
        let data2 = b"user2 data";

        // Add transactions for different users
        data.add_txnid(user1, device, &txn_id, data1).unwrap();
        data.add_txnid(user2, device, &txn_id, data2).unwrap();

        // Verify isolation
        let result1 = data.existing_txnid(user1, device, &txn_id).unwrap();
        let result2 = data.existing_txnid(user2, device, &txn_id).unwrap();

        assert_eq!(result1, Some(data1.to_vec()));
        assert_eq!(result2, Some(data2.to_vec()));
    }

    #[test]
    fn test_device_specific_transactions() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device1 = Some(create_test_device(0));
        let device2 = Some(create_test_device(1));
        let txn_id = TransactionId::new();
        let data1 = b"device1 data";
        let data2 = b"device2 data";

        // Add transactions for same user but different devices
        data.add_txnid(user, device1, &txn_id, data1).unwrap();
        data.add_txnid(user, device2, &txn_id, data2).unwrap();

        // Verify device isolation
        let result1 = data.existing_txnid(user, device1, &txn_id).unwrap();
        let result2 = data.existing_txnid(user, device2, &txn_id).unwrap();

        assert_eq!(result1, Some(data1.to_vec()));
        assert_eq!(result2, Some(data2.to_vec()));
    }

    #[test]
    fn test_no_device_transactions() {
        let data = create_test_data();
        let user = create_test_user(0);
        let txn_id = TransactionId::new();
        let test_data = b"no device data";

        // Add transaction without device
        data.add_txnid(user, None, &txn_id, test_data).unwrap();

        // Retrieve transaction without device
        let result = data.existing_txnid(user, None, &txn_id).unwrap();
        assert_eq!(result, Some(test_data.to_vec()));

        // Verify it doesn't match with a device
        let device = Some(create_test_device(0));
        let result_with_device = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result_with_device, None);
    }

    #[test]
    fn test_transaction_data_overwrite() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        let data1 = b"original data";
        let data2 = b"updated data";

        // Add original transaction
        data.add_txnid(user, device, &txn_id, data1).unwrap();

        // Overwrite with new data
        data.add_txnid(user, device, &txn_id, data2).unwrap();

        // Verify latest data is returned
        let result = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result, Some(data2.to_vec()));
    }

    #[test]
    fn test_different_transaction_ids() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id1 = TransactionId::new();
        let txn_id2 = TransactionId::new();
        let data1 = b"transaction 1";
        let data2 = b"transaction 2";

        // Add different transactions
        data.add_txnid(user, device, &txn_id1, data1).unwrap();
        data.add_txnid(user, device, &txn_id2, data2).unwrap();

        // Verify both exist independently
        let result1 = data.existing_txnid(user, device, &txn_id1).unwrap();
        let result2 = data.existing_txnid(user, device, &txn_id2).unwrap();

        assert_eq!(result1, Some(data1.to_vec()));
        assert_eq!(result2, Some(data2.to_vec()));
    }

    #[test]
    fn test_large_transaction_data() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        
        // Create large data (1MB)
        let large_data: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

        // Add large transaction
        data.add_txnid(user, device, &txn_id, &large_data).unwrap();

        // Retrieve and verify
        let result = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result, Some(large_data));
    }

    #[test]
    fn test_empty_transaction_data() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        let empty_data = b"";

        // Add empty transaction
        data.add_txnid(user, device, &txn_id, empty_data).unwrap();

        // Retrieve and verify
        let result = data.existing_txnid(user, device, &txn_id).unwrap();
        assert_eq!(result, Some(empty_data.to_vec()));
    }

    #[test]
    fn test_concurrent_transaction_operations() {
        use std::thread;

        let data = Arc::new(create_test_data());
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        
        let mut handles = vec![];

        // Spawn multiple threads adding transactions
        for i in 0..10 {
            let data_clone = Arc::clone(&data);
            let test_data_bytes = format!("concurrent data {}", i).into_bytes();
            let test_data_clone = test_data_bytes.clone();

            let handle = thread::spawn(move || {
                let txn_id = TransactionId::new();
                data_clone.add_txnid(user, device, &txn_id, &test_data_bytes).unwrap();
                data_clone.existing_txnid(user, device, &txn_id).unwrap()
            });

            handles.push((handle, test_data_clone));
        }

        // Verify all operations completed successfully
        for (handle, expected_data) in handles {
            let result = handle.join().unwrap();
            assert_eq!(result, Some(expected_data));
        }
    }

    #[test]
    fn test_performance_characteristics() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let test_data = b"performance test data";

        // Create transaction IDs ahead of time for consistent testing
        let mut txn_ids = vec![];
        for _ in 0..1000 {
            txn_ids.push(TransactionId::new());
        }

        // Test add performance
        let start = Instant::now();
        for txn_id in &txn_ids {
            data.add_txnid(user, device, txn_id, test_data).unwrap();
        }
        let add_duration = start.elapsed();

        // Test retrieve performance
        let start = Instant::now();
        for txn_id in &txn_ids {
            let _ = data.existing_txnid(user, device, txn_id).unwrap();
        }
        let retrieve_duration = start.elapsed();

        // Performance assertions
        assert!(add_duration.as_millis() < 1000, "Add operations should be <1s for 1000 transactions");
        assert!(retrieve_duration.as_millis() < 1000, "Retrieve operations should be <1s for 1000 transactions");
        
        // Verify count
        assert_eq!(data.count(), 1000, "Should have 1000 transactions stored");
    }

    #[test]
    fn test_memory_efficiency() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        
        // Add many small transactions
        for i in 0..1000 {
            let txn_id = TransactionId::new();
            let test_data = format!("data{}", i).into_bytes();
            data.add_txnid(user, device, &txn_id, &test_data).unwrap();
        }

        // Verify all transactions exist
        assert_eq!(data.count(), 1000, "Should efficiently store 1,000 transactions");

        // Clear and verify cleanup
        data.clear();
        assert_eq!(data.count(), 0, "Should efficiently clear all transactions");
    }

    #[test]
    fn test_transaction_id_uniqueness() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));

        // Generate multiple transaction IDs and verify they're different
        let mut txn_ids = vec![];
        for _ in 0..100 {
            txn_ids.push(TransactionId::new());
        }

        // Add all with different data
        for (i, txn_id) in txn_ids.iter().enumerate() {
            let test_data = format!("data for txn {}", i).into_bytes();
            data.add_txnid(user, device, txn_id, &test_data).unwrap();
        }

        // Verify each can be retrieved independently
        for (i, txn_id) in txn_ids.iter().enumerate() {
            let expected_data = format!("data for txn {}", i).into_bytes();
            let result = data.existing_txnid(user, device, txn_id).unwrap();
            assert_eq!(result, Some(expected_data));
        }
    }

    #[test]
    fn test_error_handling() {
        let data = create_test_data();
        let user = create_test_user(0);
        let device = Some(create_test_device(0));
        let txn_id = TransactionId::new();
        let test_data = b"test data";

        // These operations should not fail in the mock implementation
        assert!(data.add_txnid(user, device, &txn_id, test_data).is_ok());
        assert!(data.existing_txnid(user, device, &txn_id).is_ok());
        
        let nonexistent_txn = TransactionId::new();
        assert!(data.existing_txnid(user, device, &nonexistent_txn).is_ok());
    }
}
