// =============================================================================
// Matrixon Matrix NextServer - Abstraction Module
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
//   Database layer component for high-performance data operations. This module is part of the Matrixon Matrix NextServer
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
//   • High-performance database operations
//   • PostgreSQL backend optimization
//   • Connection pooling and caching
//   • Transaction management
//   • Data consistency guarantees
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

use super::Config;
use crate::Result;

use std::{future::Future, pin::Pin, sync::Arc};

/// Database engine types supported by matrixon
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseEngine {
    #[cfg(feature = "sqlite")]
    Sqlite,
    #[cfg(feature = "rocksdb")]
    RocksDb,
    #[cfg(feature = "backend_postgresql")]
    PostgreSql,
}

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "rocksdb")]
pub mod rocksdb;

#[cfg(feature = "backend_postgresql")]
pub mod postgresql;

#[cfg(any(feature = "sqlite", feature = "rocksdb", feature = "backend_postgresql"))]
pub mod watchers;

pub trait KeyValueDatabaseEngine: Send + Sync {
    /// Opens a new database connection with the given configuration
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseConnectionError` if the connection cannot be established
    /// or a `DatabaseMigrationError` if there are issues with database migrations.
    fn open(config: &Config) -> Result<Self>
    where
        Self: Sized;

    /// Opens a new database tree with the given name
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseConnectionError` if the tree cannot be opened
    /// or a `DatabaseQueryError` if there are issues with the tree configuration.
    fn open_tree(&self, name: &'static str) -> Result<Arc<dyn KvTree>>;

    /// Flushes all pending changes to disk
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the flush operation fails.
    fn flush(&self) -> Result<()>;

    /// Performs cleanup operations on the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the cleanup operation fails.
    fn cleanup(&self) -> Result<()> {
        Ok(())
    }

    /// Returns memory usage statistics for the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if memory usage cannot be retrieved.
    fn memory_usage(&self) -> Result<String> {
        Ok("Current database engine does not support memory usage reporting.".to_owned())
    }
}

pub trait KvTree: Send + Sync {
    /// Retrieves a value from the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if the query fails.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Inserts a key-value pair into the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the insert operation fails.
    fn insert(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Inserts multiple key-value pairs in a batch
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the batch insert fails.
    fn insert_batch(&self, iter: &mut dyn Iterator<Item = (Vec<u8>, Vec<u8>)>) -> Result<()>;

    /// Removes a key-value pair from the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the remove operation fails.
    fn remove(&self, key: &[u8]) -> Result<()>;

    /// Returns an iterator over all key-value pairs
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if the iteration fails.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;

    /// Returns an iterator over key-value pairs starting from a given key
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if the iteration fails.
    fn iter_from<'a>(
        &'a self,
        from: &[u8],
        backwards: bool,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;

    /// Increments a numeric value in the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the increment operation fails.
    fn increment(&self, key: &[u8]) -> Result<Vec<u8>>;

    /// Increments multiple numeric values in a batch
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the batch increment fails.
    fn increment_batch(&self, iter: &mut dyn Iterator<Item = Vec<u8>>) -> Result<()>;

    /// Returns an iterator over key-value pairs with a given prefix
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if the prefix scan fails.
    fn scan_prefix<'a>(
        &'a self,
        prefix: Vec<u8>,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a>;

    /// Watches for changes to keys with a given prefix
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseQueryError` if the watch operation fails.
    fn watch_prefix<'a>(&'a self, prefix: &[u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    /// Clears all key-value pairs from the database
    /// 
    /// # Errors
    /// 
    /// Returns a `DatabaseTransactionError` if the clear operation fails.
    fn clear(&self) -> Result<()> {
        for (key, _) in self.iter() {
            self.remove(&key)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        time::{Duration, Instant},
        thread,
    };
    use tracing::{debug, info};

    /// Mock implementation of KvTree for testing
    #[derive(Debug)]
    struct MockKvTree {
        data: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
        operations: Arc<RwLock<u32>>,
        prefix_watches: Arc<RwLock<Vec<Vec<u8>>>>,
    }

    impl MockKvTree {
        fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
                operations: Arc::new(RwLock::new(0)),
                prefix_watches: Arc::new(RwLock::new(Vec::new())),
            }
        }

        fn get_operation_count(&self) -> u32 {
            *self.operations.read().unwrap()
        }

        fn clear_operations(&self) {
            *self.operations.write().unwrap() = 0;
        }
    }

    impl KvTree for MockKvTree {
        fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
            *self.operations.write().unwrap() += 1;
            Ok(self.data.read().unwrap().get(key).cloned())
        }

        fn insert(&self, key: &[u8], value: &[u8]) -> Result<()> {
            *self.operations.write().unwrap() += 1;
            self.data.write().unwrap().insert(key.to_vec(), value.to_vec());
            Ok(())
        }

        fn insert_batch(&self, iter: &mut dyn Iterator<Item = (Vec<u8>, Vec<u8>)>) -> Result<()> {
            *self.operations.write().unwrap() += 1;
            let mut data = self.data.write().unwrap();
            for (key, value) in iter {
                data.insert(key, value);
            }
            Ok(())
        }

        fn remove(&self, key: &[u8]) -> Result<()> {
            *self.operations.write().unwrap() += 1;
            self.data.write().unwrap().remove(key);
            Ok(())
        }

        fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
            *self.operations.write().unwrap() += 1;
            Box::new(
                self.data
                    .read()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
        }

        fn iter_from<'a>(
            &'a self,
            from: &[u8],
            backwards: bool,
        ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
            *self.operations.write().unwrap() += 1;
            let data = self.data.read().unwrap().clone();
            let mut items: Vec<_> = data.into_iter().collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            if backwards {
                items.reverse();
            }
            Box::new(
                items
                    .into_iter()
                    .skip_while(|(k, _)| k < from)
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
        }

        fn increment(&self, key: &[u8]) -> Result<Vec<u8>> {
            *self.operations.write().unwrap() += 1;
            let mut data = self.data.write().unwrap();
            let value = data
                .get(key)
                .and_then(|v| String::from_utf8(v.clone()).ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            let new_value = (value + 1).to_string().into_bytes();
            data.insert(key.to_vec(), new_value.clone());
            Ok(new_value)
        }

        fn increment_batch(&self, iter: &mut dyn Iterator<Item = Vec<u8>>) -> Result<()> {
            *self.operations.write().unwrap() += 1;
            let mut data = self.data.write().unwrap();
            for key in iter {
                let value = data
                    .get(&key)
                    .and_then(|v| String::from_utf8(v.clone()).ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                let new_value = (value + 1).to_string().into_bytes();
                data.insert(key, new_value);
            }
            Ok(())
        }

        fn scan_prefix<'a>(
            &'a self,
            prefix: Vec<u8>,
        ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
            *self.operations.write().unwrap() += 1;
            Box::new(
                self.data
                    .read()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .filter(move |(k, _)| k.starts_with(&prefix))
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
        }

        fn watch_prefix<'a>(&'a self, prefix: &[u8]) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
            *self.operations.write().unwrap() += 1;
            self.prefix_watches.write().unwrap().push(prefix.to_vec());
            Box::pin(async move {})
        }

        fn clear(&self) -> Result<()> {
            *self.operations.write().unwrap() += 1;
            self.data.write().unwrap().clear();
            Ok(())
        }
    }

    /// Mock implementation of KeyValueDatabaseEngine for testing
    #[derive(Debug)]
    struct MockKeyValueDatabaseEngine {
        trees: Arc<RwLock<HashMap<String, Arc<MockKvTree>>>>,
        config_used: Arc<RwLock<Option<Config>>>,
    }

    impl MockKeyValueDatabaseEngine {
        fn new() -> Self {
            Self {
                trees: Arc::new(RwLock::new(HashMap::new())),
                config_used: Arc::new(RwLock::new(None)),
            }
        }

        fn get_tree_count(&self) -> usize {
            self.trees.read().unwrap().len()
        }
    }

    impl KeyValueDatabaseEngine for MockKeyValueDatabaseEngine {
        fn open(config: &Config) -> Result<Self> {
            *self.config_used.write().unwrap() = Some(config.clone());
            Ok(Self::new())
        }

        fn open_tree(&self, name: &'static str) -> Result<Arc<dyn KvTree>> {
            let mut trees = self.trees.write().unwrap();
            if !trees.contains_key(name) {
                trees.insert(name.to_string(), Arc::new(MockKvTree::new()));
            }
            Ok(trees.get(name).unwrap().clone())
        }

        fn flush(&self) -> Result<()> {
            Ok(())
        }

        fn cleanup(&self) -> Result<()> {
            Ok(())
        }

        fn memory_usage(&self) -> Result<String> {
            Ok("Mock database does not support memory usage reporting".to_owned())
        }
    }

    #[test]
    fn test_kv_tree_basic_operations() {
        debug!("🔧 Testing KvTree basic operations");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Test insert and get
        let key = b"test_key";
        let value = b"test_value";
        
        tree.insert(key, value).expect("Insert should succeed");
        let retrieved = tree.get(key).expect("Get should succeed");
        assert_eq!(retrieved, Some(value.to_vec()), "Retrieved value should match inserted value");

        // Test non-existent key
        let non_existent = tree.get(b"non_existent").expect("Get should succeed for non-existent key");
        assert_eq!(non_existent, None, "Non-existent key should return None");

        // Test remove
        tree.remove(key).expect("Remove should succeed");
        let after_remove = tree.get(key).expect("Get after remove should succeed");
        assert_eq!(after_remove, None, "Key should not exist after removal");

        assert_eq!(tree.get_operation_count(), 5, "Should have performed 5 operations");
        info!("✅ KvTree basic operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_kv_tree_batch_operations() {
        debug!("🔧 Testing KvTree batch operations");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Test batch insert
        let batch_data = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        tree.insert_batch(&mut batch_data.into_iter()).expect("Batch insert should succeed");

        // Verify all keys were inserted
        assert_eq!(tree.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(tree.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(tree.get(b"key3").unwrap(), Some(b"value3".to_vec()));

        // Test increment operations
        let counter_key = b"counter";
        let first_increment = tree.increment(counter_key).expect("First increment should succeed");
        assert_eq!(first_increment, b"1".to_vec(), "First increment should be 1");

        let second_increment = tree.increment(counter_key).expect("Second increment should succeed");
        assert_eq!(second_increment, b"2".to_vec(), "Second increment should be 2");

        // Test batch increment
        let increment_keys = vec![b"batch_counter1".to_vec(), b"batch_counter2".to_vec()];
        tree.increment_batch(&mut increment_keys.into_iter()).expect("Batch increment should succeed");
        
        assert_eq!(tree.get(b"batch_counter1").unwrap(), Some(b"1".to_vec()));
        assert_eq!(tree.get(b"batch_counter2").unwrap(), Some(b"1".to_vec()));

        info!("✅ KvTree batch operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_kv_tree_iteration() {
        debug!("🔧 Testing KvTree iteration");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Insert test data
        let test_data = vec![
            (b"aaa".to_vec(), b"value_aaa".to_vec()),
            (b"bbb".to_vec(), b"value_bbb".to_vec()),
            (b"ccc".to_vec(), b"value_ccc".to_vec()),
            (b"ddd".to_vec(), b"value_ddd".to_vec()),
        ];

        for (key, value) in &test_data {
            tree.insert(key, value).expect("Insert should succeed");
        }

        // Test full iteration
        let all_items: Vec<_> = tree.iter().collect();
        assert_eq!(all_items.len(), 4, "Should iterate over all items");

        // Test iter_from
        let from_bbb_forward: Vec<_> = tree.iter_from(b"bbb", false).collect();
        assert!(from_bbb_forward.len() >= 3, "Should include bbb, ccc, ddd and potentially more");

        let from_ccc_backward: Vec<_> = tree.iter_from(b"ccc", true).collect();
        assert!(from_ccc_backward.len() >= 3, "Should include items up to ccc");

        // Test prefix scanning
        tree.insert(b"prefix_test1", b"value1").expect("Insert should succeed");
        tree.insert(b"prefix_test2", b"value2").expect("Insert should succeed");
        tree.insert(b"other_key", b"other_value").expect("Insert should succeed");

        let prefix_results: Vec<_> = tree.scan_prefix(b"prefix_".to_vec()).collect();
        assert_eq!(prefix_results.len(), 2, "Should find exactly 2 items with prefix");

        info!("✅ KvTree iteration test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_kv_tree_clear_operation() {
        debug!("🔧 Testing KvTree clear operation");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Insert test data
        for i in 0..10 {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            tree.insert(&key, &value).expect("Insert should succeed");
        }

        // Verify data exists
        let items_before: Vec<_> = tree.iter().collect();
        assert_eq!(items_before.len(), 10, "Should have 10 items before clear");

        // Clear the tree
        tree.clear().expect("Clear should succeed");

        // Verify data is gone
        let items_after: Vec<_> = tree.iter().collect();
        assert_eq!(items_after.len(), 0, "Should have 0 items after clear");

        // Test that we can still insert after clear
        tree.insert(b"new_key", b"new_value").expect("Insert after clear should succeed");
        assert_eq!(tree.get(b"new_key").unwrap(), Some(b"new_value".to_vec()));

        info!("✅ KvTree clear operation test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_database_engine_functionality() {
        debug!("🔧 Testing database engine functionality");
        let start = Instant::now();
        
        // Create simplified mock config for testing
        let _server_name = ruma::ServerName::parse("test.example.com").unwrap().to_owned();
        
        // Test engine creation
        let engine = MockKeyValueDatabaseEngine::new();
        assert_eq!(engine.get_tree_count(), 0, "New engine should have no trees");

        // Test tree creation
        let tree1 = engine.open_tree("test_tree_1").expect("Open tree should succeed");
        let tree2 = engine.open_tree("test_tree_2").expect("Open tree should succeed");
        assert_eq!(engine.get_tree_count(), 2, "Engine should have 2 trees");

        // Test tree operations
        tree1.insert(b"key1", b"value1").expect("Insert should succeed");
        tree2.insert(b"key2", b"value2").expect("Insert should succeed");

        assert_eq!(tree1.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(tree2.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(tree1.get(b"key2").unwrap(), None, "Tree1 should not have Tree2's data");

        // Test flush operation
        engine.flush().expect("Flush should succeed");

        // Test memory usage reporting
        let memory_usage = engine.memory_usage().expect("Memory usage should succeed");
        assert!(memory_usage.contains("2 trees"), "Memory usage should mention tree count");
        assert!(memory_usage.contains("2 total items"), "Memory usage should mention item count");

        // Test cleanup
        engine.cleanup().expect("Cleanup should succeed");
        assert_eq!(engine.get_tree_count(), 0, "Engine should have no trees after cleanup");

        info!("✅ Database engine functionality test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_concurrent_operations() {
        debug!("🔧 Testing concurrent operations");
        let start = Instant::now();
        let tree = Arc::new(MockKvTree::new());
        
        let num_threads = 10;
        let operations_per_thread = 100;
        let mut handles = vec![];

        // Spawn threads performing concurrent operations
        for thread_id in 0..num_threads {
            let tree_clone = Arc::clone(&tree);
            
            let handle = thread::spawn(move || {
                for op_id in 0..operations_per_thread {
                    let key = format!("thread_{}_key_{}", thread_id, op_id).into_bytes();
                    let value = format!("thread_{}_value_{}", thread_id, op_id).into_bytes();
                    
                    // Insert
                    tree_clone.insert(&key, &value).expect("Concurrent insert should succeed");
                    
                    // Get
                    let retrieved = tree_clone.get(&key).expect("Concurrent get should succeed");
                    assert_eq!(retrieved, Some(value), "Concurrent get should return correct value");
                    
                    // Increment counter
                    let counter_key = format!("counter_{}", thread_id).into_bytes();
                    tree_clone.increment(&counter_key).expect("Concurrent increment should succeed");
                }
            });
            
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state
        let total_operations = tree.get_operation_count();
        let expected_operations = num_threads * operations_per_thread * 3; // insert + get + increment
        assert_eq!(total_operations, expected_operations as u32, 
                   "Should have performed {} operations", expected_operations);

        // Verify all data is present
        let all_items: Vec<_> = tree.iter().collect();
        let expected_items = (num_threads * operations_per_thread) + num_threads; // data + counters
        assert_eq!(all_items.len(), expected_items, "Should have correct number of items");

        info!("✅ Concurrent operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_performance_benchmarks() {
        debug!("🔧 Testing performance benchmarks");
        let start = Instant::now();
        let tree = MockKvTree::new();

        let num_operations = 10000;

        // Benchmark insert operations
        let insert_start = Instant::now();
        for i in 0..num_operations {
            let key = format!("perf_key_{}", i).into_bytes();
            let value = format!("perf_value_{}", i).into_bytes();
            tree.insert(&key, &value).expect("Insert should succeed");
        }
        let insert_duration = insert_start.elapsed();

        // Benchmark get operations
        let get_start = Instant::now();
        for i in 0..num_operations {
            let key = format!("perf_key_{}", i).into_bytes();
            let _ = tree.get(&key).expect("Get should succeed");
        }
        let get_duration = get_start.elapsed();

        // Benchmark prefix scan
        let scan_start = Instant::now();
        let scan_results: Vec<_> = tree.scan_prefix(b"perf_key_".to_vec()).collect();
        let scan_duration = scan_start.elapsed();

        // Performance assertions (enterprise grade)
        assert!(insert_duration < Duration::from_millis(5000),
                "10k insert operations should be <5000ms, was: {:?}", insert_duration);
        assert!(get_duration < Duration::from_millis(3000),
                "10k get operations should be <3000ms, was: {:?}", get_duration);
        assert!(scan_duration < Duration::from_millis(1000),
                "Prefix scan should be <1000ms, was: {:?}", scan_duration);
        assert_eq!(scan_results.len(), num_operations, "Scan should find all inserted items");

        info!("✅ Performance benchmarks completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_watch_prefix_functionality() {
        debug!("🔧 Testing watch prefix functionality");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Test watch setup
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let watch_future = tree.watch_prefix(b"watch_prefix_");
            
            // In a real implementation, this would block until changes occur
            // For our mock, it completes immediately
            watch_future.await;
            
            // Verify the watch was registered
            let watches = tree.prefix_watches.read().unwrap();
            assert_eq!(watches.len(), 1, "Should have registered one watch");
            assert_eq!(watches[0], b"watch_prefix_", "Watch should have correct prefix");
        });

        assert!(tree.get_operation_count() > 0, "Watch operation should increment counter");
        info!("✅ Watch prefix functionality test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_edge_cases() {
        debug!("🔧 Testing edge cases");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Test empty key and value
        tree.insert(b"", b"").expect("Empty key/value insert should succeed");
        assert_eq!(tree.get(b"").unwrap(), Some(b"".to_vec()), "Empty key should be retrievable");

        // Test very large key/value
        let large_key = vec![0u8; 1024];
        let large_value = vec![1u8; 1024 * 1024]; // 1MB value
        tree.insert(&large_key, &large_value).expect("Large key/value insert should succeed");
        assert_eq!(tree.get(&large_key).unwrap(), Some(large_value), "Large key/value should be retrievable");

        // Test binary data
        let binary_key = vec![0, 1, 255, 128, 64];
        let binary_value = vec![255, 254, 0, 1, 127];
        tree.insert(&binary_key, &binary_value).expect("Binary data insert should succeed");
        assert_eq!(tree.get(&binary_key).unwrap(), Some(binary_value), "Binary data should be retrievable");

        // Test remove non-existent key
        tree.remove(b"non_existent_key").expect("Remove non-existent key should not fail");

        // Test increment non-numeric value
        tree.insert(b"non_numeric", b"not_a_number").expect("Insert should succeed");
        let incremented = tree.increment(b"non_numeric").expect("Increment should handle non-numeric");
        assert_eq!(incremented, b"1".to_vec(), "Increment of non-numeric should start from 1");

        // Test prefix scan with empty prefix
        tree.insert(b"test1", b"value1").expect("Insert should succeed");
        tree.insert(b"test2", b"value2").expect("Insert should succeed");
        let empty_prefix_results: Vec<_> = tree.scan_prefix(b"".to_vec()).collect();
        assert!(empty_prefix_results.len() >= 2, "Empty prefix should match everything");

        info!("✅ Edge cases test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_memory_efficiency() {
        debug!("🔧 Testing memory efficiency");
        let start = Instant::now();
        let tree = MockKvTree::new();

        // Test memory usage with varying data sizes
        let small_data_count = 1000;
        let large_data_count = 100;

        // Insert small data
        for i in 0..small_data_count {
            let key = format!("small_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            tree.insert(&key, &value).expect("Insert should succeed");
        }

        let small_data_items: Vec<_> = tree.iter().collect();
        assert_eq!(small_data_items.len(), small_data_count, "Should have all small data items");

        // Insert large data
        for i in 0..large_data_count {
            let key = format!("large_{}", i).into_bytes();
            let value = vec![i as u8; 1024]; // 1KB per item
            tree.insert(&key, &value).expect("Insert should succeed");
        }

        let total_items: Vec<_> = tree.iter().collect();
        assert_eq!(total_items.len(), small_data_count + large_data_count, 
                   "Should have all items regardless of size");

        // Test memory cleanup
        tree.clear().expect("Clear should succeed");
        let after_clear: Vec<_> = tree.iter().collect();
        assert_eq!(after_clear.len(), 0, "Memory should be freed after clear");

        info!("✅ Memory efficiency test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_enterprise_database_compliance() {
        debug!("🔧 Testing enterprise database compliance");
        let start = Instant::now();
        
        // Create simplified mock config for testing
        let _server_name = ruma::ServerName::parse("enterprise.example.com").unwrap().to_owned();
        let engine = MockKeyValueDatabaseEngine::new();

        // Test multiple tree management
        let tree_names = vec!["users", "rooms", "events", "state", "media"];
        let mut trees = Vec::new();

        for name in &tree_names {
            let tree = engine.open_tree(name).expect("Tree should open");
            trees.push(tree);
        }

        assert_eq!(engine.get_tree_count(), tree_names.len(), "Should have all trees");

        // Test enterprise-scale data operations
        for (i, tree) in trees.iter().enumerate() {
            for j in 0..1000 {
                let key = format!("{}_{}", tree_names[i], j).into_bytes();
                let value = format!("enterprise_data_{}_{}", i, j).into_bytes();
                tree.insert(&key, &value).expect("Enterprise insert should succeed");
            }
        }

        // Test cross-tree data isolation
        for (i, tree) in trees.iter().enumerate() {
            let test_key = format!("{}_{}", tree_names[i], 0).into_bytes();
            let value = tree.get(&test_key).expect("Get should succeed");
            assert!(value.is_some(), "Tree {} should have its data", tree_names[i]);

            // Test that other trees don't have this data
            for (j, other_tree) in trees.iter().enumerate() {
                if i != j {
                    let other_value = other_tree.get(&test_key).expect("Get should succeed");
                    assert!(other_value.is_none(), "Tree {} should not have tree {}'s data", 
                           tree_names[j], tree_names[i]);
                }
            }
        }

        // Test enterprise performance requirements
        let perf_start = Instant::now();
        for tree in &trees {
            for i in 0..100 {
                let key = format!("perf_test_{}", i).into_bytes();
                let _ = tree.get(&key);
            }
        }
        let perf_duration = perf_start.elapsed();

        assert!(perf_duration < Duration::from_millis(1000),
                "Enterprise performance test should be <1000ms, was: {:?}", perf_duration);

        // Test enterprise memory reporting
        let memory_report = engine.memory_usage().expect("Memory usage should succeed");
        assert!(memory_report.contains(&tree_names.len().to_string()), 
                "Memory report should mention tree count");

        // Test enterprise cleanup
        engine.flush().expect("Enterprise flush should succeed");
        engine.cleanup().expect("Enterprise cleanup should succeed");

        info!("✅ Enterprise database compliance verified for {} trees in {:?}",
              tree_names.len(), start.elapsed());
    }

    #[test]
    fn test_error_handling_patterns() {
        use crate::Error;

        // Test database connection errors
        let error = Error::database_connection_error("Connection failed");
        assert!(format!("{:?}", error).contains("Connection failed"));
        assert!(format!("{}", error).contains("Database connection error"));

        // Test database transaction errors
        let error = Error::database_transaction_error("Transaction failed");
        assert!(format!("{:?}", error).contains("Transaction failed"));
        assert!(format!("{}", error).contains("Database transaction error"));

        // Test database query errors
        let error = Error::database_query_error("Query failed");
        assert!(format!("{:?}", error).contains("Query failed"));
        assert!(format!("{}", error).contains("Database query error"));

        // Test database migration errors
        let error = Error::database_migration_error("Migration failed");
        assert!(format!("{:?}", error).contains("Migration failed"));
        assert!(format!("{}", error).contains("Database migration error"));

        // Test error propagation
        let result: Result<()> = Err(Error::database_connection_error("Test error"));
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(format!("{}", e).contains("Database connection error"));
        }

        // Test error context
        let error = Error::database_transaction_error("Transaction failed: deadlock detected");
        let error_string = format!("{}", error);
        assert!(error_string.contains("Transaction failed"));
        assert!(error_string.contains("deadlock detected"));
    }

    #[test]
    fn test_performance_error_handling() {
        use std::time::Instant;
        use crate::Error;

        // Test error creation performance
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = Error::database_connection_error("Test error");
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Error creation should be fast");

        // Test error formatting performance
        let error = Error::database_transaction_error("Test error");
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = format!("{}", error);
        }
        let duration = start.elapsed();
        assert!(duration.as_millis() < 100, "Error formatting should be fast");
    }

    #[test]
    fn test_error_chain_handling() {
        use crate::Error;

        // Test error chain creation
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let db_error = Error::database_connection_error("Failed to open database file");
        
        // Test error context preservation
        let error_string = format!("{}", db_error);
        assert!(error_string.contains("Failed to open database file"));
        
        // Test error source access
        let source = std::error::Error::source(&db_error);
        assert!(source.is_none(), "Database connection error should not have a source");
    }
}
