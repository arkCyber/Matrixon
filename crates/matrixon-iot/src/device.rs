//! # Device Management Module
//!
//! Comprehensive IoT device lifecycle management for Matrixon.
//! Handles device registration, authentication, monitoring, and communication.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{IoTError, DeviceType, DeviceCapability, HardwareInfo, 
           PowerSource, IoTConfig, ProtocolType};

// =============================================================================
// Device Status and Configuration
// =============================================================================

/// Device connection status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    /// Device is offline/disconnected
    Offline,
    /// Device is online and connected
    Connected,
    /// Device is connecting/initializing
    Connecting,
    /// Device is in sleep/low-power mode
    Sleeping,
    /// Device has encountered an error
    Error(String),
    /// Device is undergoing maintenance
    Maintenance,
    /// Device is updating firmware
    Updating,
    /// Device is being decommissioned
    Decommissioning,
}

/// Device configuration for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// Unique device identifier
    pub device_id: String,
    /// Human-readable device name  
    pub name: String,
    /// Device type classification
    pub device_type: DeviceType,
    /// Communication protocol
    pub protocol: ProtocolType,
    /// Device location coordinates (lat, lon)
    pub location: Option<(f64, f64)>,
    /// Authentication token
    pub auth_token: Option<String>,
    /// Device metadata
    pub metadata: HashMap<String, String>,
    /// Device capabilities
    pub capabilities: Vec<DeviceCapability>,
    /// Firmware version
    pub firmware_version: String,
    /// Hardware information
    pub hardware_info: HardwareInfo,
    /// Matrix room ID for device communication
    pub matrix_room_id: Option<String>,
    /// Device configuration parameters
    pub config_params: HashMap<String, serde_json::Value>,
}

impl DeviceConfig {
    /// Create a new device configuration
    pub fn new(device_id: &str, protocol: ProtocolType) -> Self {
        DeviceConfig {
            device_id: device_id.to_string(),
            name: device_id.to_string(),
            device_type: DeviceType::Sensor,
            protocol,
            location: None,
            auth_token: None,
            metadata: HashMap::new(),
            capabilities: Vec::new(),
            firmware_version: "1.0.0".to_string(),
            hardware_info: HardwareInfo::default(),
            matrix_room_id: None,
            config_params: HashMap::new(),
        }
    }
    
    /// Set device name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    
    /// Set device type
    pub fn with_device_type(mut self, device_type: DeviceType) -> Self {
        self.device_type = device_type;
        self
    }
    
    /// Set device location
    pub fn with_location(mut self, latitude: f64, longitude: f64) -> Self {
        self.location = Some((latitude, longitude));
        self
    }
    
    /// Set authentication token
    pub fn with_authentication(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }
    
    /// Add device capability
    pub fn with_capability(mut self, capability: DeviceCapability) -> Self {
        self.capabilities.push(capability);
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Set firmware version
    pub fn with_firmware_version(mut self, version: &str) -> Self {
        self.firmware_version = version.to_string();
        self
    }
    
    /// Set Matrix room ID
    pub fn with_matrix_room(mut self, room_id: &str) -> Self {
        self.matrix_room_id = Some(room_id.to_string());
        self
    }
}

// =============================================================================
// Device Manager Implementation
// =============================================================================

/// Device manager for handling device lifecycle
pub struct DeviceManager {
    /// Registered devices storage
    devices: Arc<RwLock<HashMap<String, DeviceInfo>>>,
    
    /// Device configurations
    device_configs: Arc<RwLock<HashMap<String, DeviceConfig>>>,
    
    /// Configuration
    config: IoTConfig,
    
    /// Device statistics
    stats: Arc<RwLock<DeviceManagerStats>>,
}

/// Simple device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub protocol: ProtocolType,
    pub status: DeviceStatus,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Device manager statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceManagerStats {
    /// Total registered devices
    pub total_devices: usize,
    
    /// Currently connected devices
    pub connected_devices: usize,
    
    /// Devices by status
    pub devices_by_status: HashMap<String, usize>,
    
    /// Devices by type
    pub devices_by_type: HashMap<String, usize>,
    
    /// Total messages processed
    pub total_messages: u64,
    
    /// Error count
    pub error_count: u64,
    
    /// Average connection time
    pub avg_connection_time: Duration,
}

impl DeviceManager {
    /// Create a new device manager
    #[instrument(level = "info")]
    pub async fn new(config: &IoTConfig) -> Result<Self, IoTError> {
        info!("🔧 Initializing Device Manager");
        
        Ok(DeviceManager {
            devices: Arc::new(RwLock::new(HashMap::new())),
            device_configs: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            stats: Arc::new(RwLock::new(DeviceManagerStats::default())),
        })
    }
    
    /// Register a new device
    #[instrument(level = "debug", skip(self))]
    pub async fn register_device(&self, device_config: DeviceConfig) -> Result<String, IoTError> {
        info!("📱 Registering device: {}", device_config.device_id);
        
        let device_id = device_config.device_id.clone();
        
        // Validate device configuration
        self.validate_device_config(&device_config)?;
        
        // Create device info
        let device_info = DeviceInfo {
            device_id: device_id.clone(),
            name: device_config.name.clone(),
            device_type: device_config.device_type.clone(),
            protocol: device_config.protocol.clone(),
            status: DeviceStatus::Offline,
            registered_at: Utc::now(),
            last_seen: Utc::now(),
        };
        
        // Store device and update stats
        {
            let mut devices = self.devices.write().await;
            let device_type_key = format!("{:?}", device_info.device_type);
            let status_key = format!("{:?}", device_info.status);
            
            devices.insert(device_id.clone(), device_info);
            
            let mut stats = self.stats.write().await;
            stats.total_devices += 1;
            *stats.devices_by_type.entry(device_type_key).or_insert(0) += 1;
            *stats.devices_by_status.entry(status_key).or_insert(0) += 1;
        }
        
        {
            let mut configs = self.device_configs.write().await;
            configs.insert(device_id.clone(), device_config);
        }
        
        info!("✅ Device registered successfully: {}", device_id);
        Ok(device_id)
    }
    
    /// Get device by ID
    #[instrument(level = "debug", skip(self))]
    pub async fn get_device(&self, device_id: &str) -> Result<DeviceInfo, IoTError> {
        let devices = self.devices.read().await;
        devices.get(device_id)
            .cloned()
            .ok_or_else(|| IoTError::DeviceConnectionFailed {
                device_id: device_id.to_string(),
            })
    }
    
    /// Update device status
    #[instrument(level = "debug", skip(self))]
    pub async fn update_device_status(&self, device_id: &str, new_status: DeviceStatus) -> Result<(), IoTError> {
        info!("🔄 Updating device status: {} -> {:?}", device_id, new_status);
        
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            let old_status = device.status.clone();
            device.status = new_status.clone();
            device.last_seen = Utc::now();
            
            // Update statistics
            drop(devices); // Release lock before acquiring stats lock
            let mut stats = self.stats.write().await;
            
            // Decrement old status count
            let old_status_key = format!("{:?}", old_status);
            if let Some(count) = stats.devices_by_status.get_mut(&old_status_key) {
                *count = count.saturating_sub(1);
            }
            
            // Increment new status count
            let new_status_key = format!("{:?}", new_status);
            *stats.devices_by_status.entry(new_status_key).or_insert(0) += 1;
            
            // Update connected devices count
            match new_status {
                DeviceStatus::Connected => stats.connected_devices += 1,
                _ => {
                    if matches!(old_status, DeviceStatus::Connected) {
                        stats.connected_devices = stats.connected_devices.saturating_sub(1);
                    }
                }
            }
        } else {
            return Err(IoTError::DeviceConnectionFailed {
                device_id: device_id.to_string(),
            });
        }
        
        info!("✅ Device status updated successfully: {}", device_id);
        Ok(())
    }
    
    /// Remove device
    #[instrument(level = "debug", skip(self))]
    pub async fn remove_device(&self, device_id: &str) -> Result<(), IoTError> {
        info!("🗑️ Removing device: {}", device_id);
        
        // Get device before removal
        let device = self.get_device(device_id).await?;
        
        // Remove from storage
        {
            let mut devices = self.devices.write().await;
            devices.remove(device_id);
        }
        
        {
            let mut configs = self.device_configs.write().await;
            configs.remove(device_id);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_devices = stats.total_devices.saturating_sub(1);
            
            let device_type_key = format!("{:?}", device.device_type);
            if let Some(count) = stats.devices_by_type.get_mut(&device_type_key) {
                *count = count.saturating_sub(1);
            }
            
            let status_key = format!("{:?}", device.status);
            if let Some(count) = stats.devices_by_status.get_mut(&status_key) {
                *count = count.saturating_sub(1);
            }
            
            if matches!(device.status, DeviceStatus::Connected) {
                stats.connected_devices = stats.connected_devices.saturating_sub(1);
            }
        }
        
        info!("✅ Device removed successfully: {}", device_id);
        Ok(())
    }
    
    /// List all devices
    pub async fn list_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }
    
    /// Get device manager statistics
    pub async fn get_statistics(&self) -> DeviceManagerStats {
        self.stats.read().await.clone()
    }
    
    /// Process device heartbeat
    #[instrument(level = "debug", skip(self))]
    pub async fn process_heartbeat(&self, device_id: &str) -> Result<(), IoTError> {
        info!("💓 Processing heartbeat for device: {}", device_id);
        
        // Update last seen timestamp
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            device.last_seen = Utc::now();
        }
        
        info!("✅ Heartbeat processed for device: {}", device_id);
        Ok(())
    }

    /// Bind device to Matrix room
    #[instrument(level = "debug", skip(self))]
    pub async fn bind_to_room(&self, device_id: &str, room_id: &str) -> Result<(), IoTError> {
        info!("🔗 Binding device {} to room {}", device_id, room_id);
        
        let mut configs = self.device_configs.write().await;
        if let Some(config) = configs.get_mut(device_id) {
            config.matrix_room_id = Some(room_id.to_string());
            info!("✅ Device bound to room successfully");
            Ok(())
        } else {
            Err(IoTError::DeviceConnectionFailed {
                device_id: device_id.to_string(),
            })
        }
    }

    /// Unbind device from Matrix room
    #[instrument(level = "debug", skip(self))]
    pub async fn unbind_from_room(&self, device_id: &str) -> Result<(), IoTError> {
        info!("🔗 Unbinding device {} from room", device_id);
        
        let mut configs = self.device_configs.write().await;
        if let Some(config) = configs.get_mut(device_id) {
            config.matrix_room_id = None;
            info!("✅ Device unbound from room successfully");
            Ok(())
        } else {
            Err(IoTError::DeviceConnectionFailed {
                device_id: device_id.to_string(),
            })
        }
    }

    /// Get Matrix room ID for device
    #[instrument(level = "debug", skip(self))]
    pub async fn get_device_room(&self, device_id: &str) -> Result<Option<String>, IoTError> {
        let configs = self.device_configs.read().await;
        if let Some(config) = configs.get(device_id) {
            Ok(config.matrix_room_id.clone())
        } else {
            Err(IoTError::DeviceConnectionFailed {
                device_id: device_id.to_string(),
            })
        }
    }
    
    /// Validate device configuration
    fn validate_device_config(&self, config: &DeviceConfig) -> Result<(), IoTError> {
        if config.device_id.is_empty() {
            return Err(IoTError::ConfigurationError {
                parameter: "device_id cannot be empty".to_string(),
            });
        }
        
        if config.name.is_empty() {
            return Err(IoTError::ConfigurationError {
                parameter: "device name cannot be empty".to_string(),
            });
        }
        
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProtocolType;
    
    #[tokio::test]
    async fn test_device_manager_creation() {
        let config = IoTConfig::default();
        let manager = DeviceManager::new(&config).await;
        assert!(manager.is_ok());
    }
    
    #[test]
    fn test_device_config_builder() {
        let config = DeviceConfig::new("test001", ProtocolType::MQTT)
            .with_name("Test Device")
            .with_device_type(DeviceType::Sensor)
            .with_location(40.7128, -74.0060)
            .with_authentication("token123")
            .with_capability(DeviceCapability::TemperatureSensing);
            
        assert_eq!(config.device_id, "test001");
        assert_eq!(config.name, "Test Device");
        assert_eq!(config.device_type, DeviceType::Sensor);
        assert_eq!(config.location, Some((40.7128, -74.0060)));
        assert_eq!(config.auth_token, Some("token123".to_string()));
        assert!(config.capabilities.contains(&DeviceCapability::TemperatureSensing));
    }
    
    #[test]
    fn test_device_status() {
        let status = DeviceStatus::Connected;
        assert_eq!(status, DeviceStatus::Connected);
        
        let error_status = DeviceStatus::Error("Connection timeout".to_string());
        match error_status {
            DeviceStatus::Error(msg) => assert_eq!(msg, "Connection timeout"),
            _ => panic!("Expected error status"),
        }
    }

    #[tokio::test]
    async fn test_room_binding() {
        let config = IoTConfig::default();
        let manager = DeviceManager::new(&config).await.unwrap();
        
        // Register test device
        let device_id = "test-room-binding";
        let device_config = DeviceConfig::new(device_id, ProtocolType::Matrix)
            .with_name("Test Room Binding");
            
        manager.register_device(device_config).await.unwrap();
        
        // Test binding to room
        let room_id = "!testroom:matrix.org";
        manager.bind_to_room(device_id, room_id).await.unwrap();
        
        // Verify room binding
        let bound_room = manager.get_device_room(device_id).await.unwrap();
        assert_eq!(bound_room, Some(room_id.to_string()));
        
        // Test unbinding from room
        manager.unbind_from_room(device_id).await.unwrap();
        
        // Verify unbinding
        let unbound_room = manager.get_device_room(device_id).await.unwrap();
        assert_eq!(unbound_room, None);
    }

    #[tokio::test]
    async fn test_nonexistent_device_room_ops() {
        let config = IoTConfig::default();
        let manager = DeviceManager::new(&config).await.unwrap();
        
        // Test operations on non-existent device
        let result = manager.bind_to_room("nonexistent", "!room:matrix.org").await;
        assert!(matches!(result, Err(IoTError::DeviceConnectionFailed { .. })));
        
        let result = manager.unbind_from_room("nonexistent").await;
        assert!(matches!(result, Err(IoTError::DeviceConnectionFailed { .. })));
        
        let result = manager.get_device_room("nonexistent").await;
        assert!(matches!(result, Err(IoTError::DeviceConnectionFailed { .. })));
    }
}
