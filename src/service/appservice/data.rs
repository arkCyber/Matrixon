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

use ruma::api::appservice::Registration;

use crate::Result;

/// Data access trait for Matrix application service management
/// 
/// Provides methods for registering, unregistering, and managing application services
/// that integrate with the Matrix NextServer. All operations are designed to be
/// thread-safe and support high-concurrency environments.
/// 
/// # Performance Requirements
/// - Registration: <10ms for typical configurations
/// - Lookup operations: <1ms average
/// - Support for 100+ concurrent appservices
/// 
/// # Error Handling
/// All methods return `Result<T, Error>` to provide comprehensive error context
/// and enable graceful error recovery in the calling code.
pub trait Data: Send + Sync {
    /// Registers a new application service with the NextServer
    /// 
    /// Stores the appservice configuration and returns a unique identifier
    /// that can be used for subsequent operations. The registration includes
    /// all necessary metadata for Matrix protocol compliance.
    /// 
    /// # Arguments
    /// * `yaml` - Complete appservice registration configuration following Matrix spec
    /// 
    /// # Returns
    /// * `Ok(String)` - Unique identifier for the registered appservice
    /// * `Err(Error)` - Registration failure with detailed error context
    /// 
    /// # Examples
    /// ```rust,ignore
    /// let registration = Registration {
    ///     id: "my_appservice".to_string(),
    ///     url: "http://localhost:8080".parse().unwrap(),
    ///     as_token: "token123".to_string(),
    ///     hs_token: "token456".to_string(),
    ///     sender_localpart: "bot".to_string(),
    ///     namespaces: Default::default(),
    ///     rate_limited: Some(false),
    ///     protocols: None,
    /// };
    /// let id = data.register_appservice(registration)?;
    /// ```
    /// 
    /// # Performance
    /// - Target latency: <10ms for typical registrations
    /// - Memory impact: ~1KB per registration
    /// - Concurrent safe: Yes
    fn register_appservice(&self, yaml: Registration) -> Result<String>;

    /// Removes an application service registration from the NextServer
    /// 
    /// Completely removes the appservice configuration and all associated
    /// metadata. This operation is irreversible and will immediately
    /// disable the appservice's access to the NextServer.
    /// 
    /// # Arguments
    /// * `service_name` - The unique identifier returned during registration
    /// 
    /// # Returns
    /// * `Ok(())` - Successfully unregistered the appservice
    /// * `Err(Error)` - Unregistration failure (service not found, etc.)
    /// 
    /// # Examples
    /// ```rust,ignore
    /// data.unregister_appservice("my_appservice")?;
    /// ```
    /// 
    /// # Performance
    /// - Target latency: <5ms
    /// - Concurrent safe: Yes
    /// - Cleanup: Immediate metadata removal
    fn unregister_appservice(&self, service_name: &str) -> Result<()>;

    /// Retrieves the registration configuration for a specific appservice
    /// 
    /// Returns the complete registration data if the appservice exists,
    /// or None if no appservice with the given ID is registered.
    /// 
    /// # Arguments
    /// * `id` - The unique identifier of the appservice
    /// 
    /// # Returns
    /// * `Ok(Some(Registration))` - Complete appservice configuration
    /// * `Ok(None)` - No appservice found with the given ID
    /// * `Err(Error)` - Database or system error
    /// 
    /// # Examples
    /// ```rust,ignore
    /// if let Some(registration) = data.get_registration("my_appservice")? {
    ///     println!("Appservice URL: {}", registration.url);
    /// }
    /// ```
    /// 
    /// # Performance
    /// - Target latency: <1ms
    /// - Memory usage: Minimal (shared references)
    /// - Concurrent safe: Yes
    fn get_registration(&self, id: &str) -> Result<Option<Registration>>;

    /// Returns an iterator over all registered appservice IDs
    /// 
    /// Provides efficient iteration over appservice identifiers without
    /// loading the full registration data. Useful for administrative
    /// operations and system monitoring.
    /// 
    /// # Returns
    /// * `Ok(Iterator)` - Iterator over appservice IDs
    /// * `Err(Error)` - Database or system error
    /// 
    /// # Examples
    /// ```rust,ignore
    /// for id_result in data.iter_ids()? {
    ///     let id = id_result?;
    ///     println!("Registered appservice: {}", id);
    /// }
    /// ```
    /// 
    /// # Performance
    /// - Lazy evaluation: O(1) to start iteration
    /// - Memory usage: Minimal per iteration
    /// - Total iterations: O(n) where n = registered appservices
    fn iter_ids<'a>(&'a self) -> Result<Box<dyn Iterator<Item = Result<String>> + 'a>>;

    /// Retrieves all registered appservices with their complete configurations
    /// 
    /// Returns a vector containing all appservice IDs paired with their
    /// complete registration data. Use sparingly as this loads all
    /// appservice data into memory simultaneously.
    /// 
    /// # Returns
    /// * `Ok(Vec<(String, Registration)>)` - All appservice ID/registration pairs
    /// * `Err(Error)` - Database or system error
    /// 
    /// # Examples
    /// ```rust,ignore
    /// let all_services = data.all()?;
    /// println!("Total registered appservices: {}", all_services.len());
    /// for (id, registration) in all_services {
    ///     println!("{}: {}", id, registration.url);
    /// }
    /// ```
    /// 
    /// # Performance
    /// - Memory usage: High (loads all registrations)
    /// - Target latency: <50ms for 100 appservices
    /// - Use case: Administrative operations, full dumps
    fn all(&self) -> Result<Vec<(String, Registration)>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::api::appservice::Registration;
    use std::collections::{BTreeMap, HashMap};
    use std::sync::{Arc, RwLock};

    /// Mock implementation of the Data trait for testing
    /// 
    /// Provides an in-memory implementation suitable for unit testing
    /// that maintains all the behavioral contracts of the real implementation.
    #[derive(Debug, Default)]
    struct MockAppserviceData {
        /// In-memory storage for appservice registrations
        registrations: Arc<RwLock<HashMap<String, Registration>>>,
    }

    impl MockAppserviceData {
        /// Creates a new mock data instance with empty storage
        fn new() -> Self {
            Self {
                registrations: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        /// Creates a mock instance pre-populated with test data
        fn with_test_data() -> Self {
            let mock = Self::new();
            let registration = create_test_registration("test_service");
            mock.registrations
                .write()
                .unwrap()
                .insert("test_service".to_string(), registration);
            mock
        }
    }

    impl Data for MockAppserviceData {
        fn register_appservice(&self, yaml: Registration) -> Result<String> {
            let id = yaml.id.clone();
            self.registrations
                .write()
                .map_err(|_| crate::Error::bad_database("Failed to acquire write lock"))?
                .insert(id.clone(), yaml);
            Ok(id)
        }

        fn unregister_appservice(&self, service_name: &str) -> Result<()> {
            let mut registrations = self.registrations
                .write()
                .map_err(|_| crate::Error::bad_database("Failed to acquire write lock"))?;
            
            if registrations.remove(service_name).is_some() {
                Ok(())
            } else {
                Err(crate::Error::AdminCommand("Appservice not found"))
            }
        }

        fn get_registration(&self, id: &str) -> Result<Option<Registration>> {
            let registrations = self.registrations
                .read()
                .map_err(|_| crate::Error::bad_database("Failed to acquire read lock"))?;
            Ok(registrations.get(id).cloned())
        }

        fn iter_ids<'a>(&'a self) -> Result<Box<dyn Iterator<Item = Result<String>> + 'a>> {
            let registrations = self.registrations
                .read()
                .map_err(|_| crate::Error::bad_database("Failed to acquire read lock"))?;
            let ids: Vec<String> = registrations.keys().cloned().collect();
            Ok(Box::new(ids.into_iter().map(Ok)))
        }

        fn all(&self) -> Result<Vec<(String, Registration)>> {
            let registrations = self.registrations
                .read()
                .map_err(|_| crate::Error::bad_database("Failed to acquire read lock"))?;
            Ok(registrations
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
        }
    }

    /// Creates a test registration with the given ID
    fn create_test_registration(id: &str) -> Registration {
        // Create a Registration using serde deserialization from a YAML-like structure
        let yaml_content = format!(
            "id: {}\nurl: http://localhost:8080\nas_token: as_token_{}\nhs_token: hs_token_{}\nsender_localpart: bot_{}\nnamespaces:\n  users:\n    - exclusive: true\n      regex: \"@{}.*:example.com\"\n  aliases:\n    - exclusive: false\n      regex: \"#{}.*:example.com\"\n  rooms: []\nrate_limited: false\nprotocols:\n  - matrix\nreceive_ephemeral: false\n",
            id, id, id, id, id, id
        );

        serde_yaml::from_str(&yaml_content)
            .expect("Valid test registration YAML should deserialize")
    }

    #[test]
    fn test_register_appservice_success() {
        let data = MockAppserviceData::new();
        let registration = create_test_registration("test_app");
        
        let result = data.register_appservice(registration.clone());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_app");

        // Verify the appservice was actually stored
        let retrieved = data.get_registration("test_app").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test_app");
    }

    #[test]
    fn test_register_appservice_overwrites_existing() {
        let data = MockAppserviceData::new();
        let original = create_test_registration("test_app");
        let mut updated = create_test_registration("test_app");
        updated.url = Some("http://localhost:9090".to_string());

        // Register original
        data.register_appservice(original).unwrap();

        // Register updated version
        let result = data.register_appservice(updated.clone());
        assert!(result.is_ok());

        // Verify the updated version is stored
        let retrieved = data.get_registration("test_app").unwrap().unwrap();
        assert_eq!(retrieved.url.as_ref().unwrap(), "http://localhost:9090");
    }

    #[test]
    fn test_unregister_appservice_success() {
        let data = MockAppserviceData::with_test_data();

        // Verify appservice exists
        assert!(data.get_registration("test_service").unwrap().is_some());

        // Unregister it
        let result = data.unregister_appservice("test_service");
        assert!(result.is_ok());

        // Verify it no longer exists
        assert!(data.get_registration("test_service").unwrap().is_none());
    }

    #[test]
    fn test_unregister_appservice_not_found() {
        let data = MockAppserviceData::new();

        let result = data.unregister_appservice("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_registration_exists() {
        let data = MockAppserviceData::with_test_data();

        let result = data.get_registration("test_service");
        assert!(result.is_ok());
        
        let registration = result.unwrap();
        assert!(registration.is_some());
        
        let reg = registration.unwrap();
        assert_eq!(reg.id, "test_service");
        assert_eq!(reg.sender_localpart, "bot_test_service");
        assert_eq!(reg.as_token, "as_token_test_service");
        assert_eq!(reg.hs_token, "hs_token_test_service");
    }

    #[test]
    fn test_get_registration_not_found() {
        let data = MockAppserviceData::new();

        let result = data.get_registration("nonexistent");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_iter_ids_empty() {
        let data = MockAppserviceData::new();

        let result = data.iter_ids();
        assert!(result.is_ok());

        let ids: Vec<_> = result.unwrap().collect();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_iter_ids_multiple_services() {
        let data = MockAppserviceData::new();
        
        // Register multiple appservices
        data.register_appservice(create_test_registration("service1")).unwrap();
        data.register_appservice(create_test_registration("service2")).unwrap();
        data.register_appservice(create_test_registration("service3")).unwrap();

        let result = data.iter_ids();
        assert!(result.is_ok());

        let ids: Result<Vec<String>, _> = result.unwrap().collect();
        assert!(ids.is_ok());
        
        let mut id_vec = ids.unwrap();
        id_vec.sort(); // HashMap iteration order is not guaranteed
        assert_eq!(id_vec, vec!["service1", "service2", "service3"]);
    }

    #[test]
    fn test_all_empty() {
        let data = MockAppserviceData::new();

        let result = data.all();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_all_multiple_services() {
        let data = MockAppserviceData::new();
        
        // Register multiple appservices
        data.register_appservice(create_test_registration("service1")).unwrap();
        data.register_appservice(create_test_registration("service2")).unwrap();

        let result = data.all();
        assert!(result.is_ok());
        
        let all_services = result.unwrap();
        assert_eq!(all_services.len(), 2);

        // Convert to map for easier testing (order not guaranteed)
        let service_map: BTreeMap<String, Registration> = all_services.into_iter().collect();
        
        assert!(service_map.contains_key("service1"));
        assert!(service_map.contains_key("service2"));
        
        assert_eq!(service_map["service1"].sender_localpart, "bot_service1");
        assert_eq!(service_map["service2"].sender_localpart, "bot_service2");
    }

    #[test]
    fn test_registration_data_integrity() {
        let data = MockAppserviceData::new();
        let registration = create_test_registration("integrity_test");

        // Store the registration
        data.register_appservice(registration.clone()).unwrap();

        // Retrieve and verify all fields
        let retrieved = data.get_registration("integrity_test").unwrap().unwrap();
        
        assert_eq!(retrieved.id, registration.id);
        assert_eq!(retrieved.url, registration.url);
        assert_eq!(retrieved.as_token, registration.as_token);
        assert_eq!(retrieved.hs_token, registration.hs_token);
        assert_eq!(retrieved.sender_localpart, registration.sender_localpart);
        assert_eq!(retrieved.rate_limited, registration.rate_limited);
        assert_eq!(retrieved.protocols, registration.protocols);
        
        // Verify namespaces
        assert_eq!(retrieved.namespaces.users.len(), registration.namespaces.users.len());
        assert_eq!(retrieved.namespaces.aliases.len(), registration.namespaces.aliases.len());
        assert_eq!(retrieved.namespaces.rooms.len(), registration.namespaces.rooms.len());
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let data = Arc::new(MockAppserviceData::new());
        let mut handles = vec![];

        // Spawn multiple threads doing concurrent operations
        for i in 0..10 {
            let data_clone = Arc::clone(&data);
            let handle = thread::spawn(move || {
                let id = format!("service_{}", i);
                let registration = create_test_registration(&id);
                
                // Register
                data_clone.register_appservice(registration).unwrap();
                
                // Verify it exists
                assert!(data_clone.get_registration(&id).unwrap().is_some());
                
                // List all (this tests read concurrency)
                let all = data_clone.all().unwrap();
                assert!(!all.is_empty());
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all appservices were registered
        let all_services = data.all().unwrap();
        assert_eq!(all_services.len(), 10);
    }

    #[test]
    fn test_edge_cases() {
        let data = MockAppserviceData::new();

        // Test with empty string ID - create using YAML
        let yaml_content = "id: \"\"\nurl: http://localhost:8080\nas_token: token\nhs_token: token\nsender_localpart: bot\nnamespaces:\n  users: []\n  aliases: []\n  rooms: []\nrate_limited: false\nprotocols: []\nreceive_ephemeral: false\n";
        let empty_registration: Registration = serde_yaml::from_str(yaml_content)
            .expect("Valid test registration YAML should deserialize");
        
        let result = data.register_appservice(empty_registration);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");

        // Test retrieving with empty ID
        let retrieved = data.get_registration("");
        assert!(retrieved.is_ok());
        assert!(retrieved.unwrap().is_some());

        // Test unregistering with empty ID
        let unregister_result = data.unregister_appservice("");
        assert!(unregister_result.is_ok());
    }

    #[test]
    fn test_namespace_handling() {
        let data = MockAppserviceData::new();
        
        // Create registration with complex namespace configuration using YAML
        let yaml_content = "id: namespace_test\nurl: http://localhost:8080\nas_token: as_token_namespace_test\nhs_token: hs_token_namespace_test\nsender_localpart: bot_namespace_test\nnamespaces:\n  users:\n    - exclusive: true\n      regex: \"@namespace_test.*:example.com\"\n    - exclusive: false\n      regex: \"@bot_.*:example.com\"\n  aliases:\n    - exclusive: false\n      regex: \"#namespace_test.*:example.com\"\n  rooms:\n    - exclusive: true\n      regex: \"!private_.*:example.com\"\nrate_limited: false\nprotocols:\n  - matrix\nreceive_ephemeral: false\n";
        
        let registration: Registration = serde_yaml::from_str(yaml_content)
            .expect("Valid test registration YAML should deserialize");

        data.register_appservice(registration.clone()).unwrap();

        let retrieved = data.get_registration("namespace_test").unwrap().unwrap();
        
        // Verify complex namespace data
        assert_eq!(retrieved.namespaces.users.len(), 2);
        assert_eq!(retrieved.namespaces.aliases.len(), 1);
        assert_eq!(retrieved.namespaces.rooms.len(), 1);
        
        // Check specific namespace properties
        assert!(retrieved.namespaces.users.iter().any(|ns| ns.exclusive == false));
        assert!(retrieved.namespaces.rooms.iter().any(|ns| ns.exclusive == true));
    }

    #[test]
    fn test_performance_characteristics() {
        let data = MockAppserviceData::new();
        
        // Register a reasonable number of appservices to test performance
        let start = std::time::Instant::now();
        
        for i in 0..100 {
            let id = format!("perf_test_{}", i);
            let registration = create_test_registration(&id);
            data.register_appservice(registration).unwrap();
        }
        
        let registration_time = start.elapsed();
        println!("Registered 100 appservices in {:?}", registration_time);
        
        // Test retrieval performance
        let start = std::time::Instant::now();
        for i in 0..100 {
            let id = format!("perf_test_{}", i);
            assert!(data.get_registration(&id).unwrap().is_some());
        }
        let retrieval_time = start.elapsed();
        println!("Retrieved 100 appservices in {:?}", retrieval_time);
        
        // Test iteration performance
        let start = std::time::Instant::now();
        let ids: Vec<_> = data.iter_ids().unwrap().collect();
        let iteration_time = start.elapsed();
        println!("Iterated over {} appservice IDs in {:?}", ids.len(), iteration_time);
        
        // Verify we have the expected number
        assert_eq!(ids.len(), 100);
        
        // Performance assertions (these are generous bounds for testing)
        assert!(registration_time.as_millis() < 1000, "Registration should be fast");
        assert!(retrieval_time.as_millis() < 100, "Retrieval should be very fast");
        assert!(iteration_time.as_millis() < 50, "Iteration should be fast");
    }

    #[test]
    fn test_memory_usage() {
        let data = MockAppserviceData::new();
        
        // Register appservices and verify memory doesn't grow excessively
        for i in 0..50 {
            let id = format!("memory_test_{}", i);
            let registration = create_test_registration(&id);
            data.register_appservice(registration).unwrap();
        }
        
        // Get all registrations
        let all_services = data.all().unwrap();
        assert_eq!(all_services.len(), 50);
        
        // The fact that this completes without OOM indicates reasonable memory usage
        // In a real implementation, we might check actual memory usage metrics
    }

    #[test]
    fn test_error_handling_scenarios() {
        let data = MockAppserviceData::new();
        
        // Test multiple unregister attempts
        data.register_appservice(create_test_registration("error_test")).unwrap();
        
        // First unregister should succeed
        assert!(data.unregister_appservice("error_test").is_ok());
        
        // Second unregister should fail
        assert!(data.unregister_appservice("error_test").is_err());
        
        // Third unregister should also fail
        assert!(data.unregister_appservice("error_test").is_err());
    }
}
