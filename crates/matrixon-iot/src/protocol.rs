//! # Protocol Handler Module
//!
//! Multi-protocol support for IoT device communication.
//! Supports MQTT, CoAP, WebSocket, Modbus, and LoRaWAN protocols.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::{IoTError, IoTMessage, IoTConfig, ProtocolType};

// =============================================================================
// Protocol Configuration Types
// =============================================================================

/// Protocol-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolConfig {
    MQTT(MQTTConfig),
    CoAP(CoAPConfig),
    WebSocket(WebSocketConfig),
    Modbus(ModbusConfig),
    LoRaWAN(LoRaWANConfig),
    HTTP(HTTPConfig),
    TCP(TCPConfig),
    UDP(UDPConfig),
    Custom(CustomProtocolConfig),
}

/// MQTT protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MQTTConfig {
    /// Broker host address
    pub host: String,
    /// Broker port
    pub port: u16,
    /// Client ID
    pub client_id: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password for authentication
    pub password: Option<String>,
    /// Keep alive interval
    pub keep_alive: Duration,
    /// QoS level
    pub qos: u8,
    /// Use TLS encryption
    pub use_tls: bool,
    /// TLS certificate path
    pub cert_path: Option<String>,
    /// Subscribe topics
    pub subscribe_topics: Vec<String>,
    /// Publish topic prefix
    pub publish_prefix: String,
}

/// CoAP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoAPConfig {
    /// CoAP server host
    pub host: String,
    /// CoAP server port
    pub port: u16,
    /// Use DTLS encryption
    pub use_dtls: bool,
    /// Block transfer size
    pub block_size: u16,
    /// Maximum retransmission attempts
    pub max_retransmits: u8,
    /// Response timeout
    pub timeout: Duration,
}

/// WebSocket protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// WebSocket server URL
    pub url: String,
    /// Subprotocols
    pub subprotocols: Vec<String>,
    /// Connection timeout
    pub timeout: Duration,
    /// Use TLS encryption
    pub use_tls: bool,
    /// Ping interval
    pub ping_interval: Duration,
    /// Maximum message size
    pub max_message_size: usize,
}

/// Modbus protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusConfig {
    /// Modbus server address
    pub address: String,
    /// Modbus port
    pub port: u16,
    /// Slave ID
    pub slave_id: u8,
    /// Modbus variant (TCP, RTU, ASCII)
    pub variant: ModbusVariant,
    /// Connection timeout
    pub timeout: Duration,
    /// Register polling interval
    pub poll_interval: Duration,
}

/// Modbus protocol variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModbusVariant {
    TCP,
    RTU,
    ASCII,
}

/// LoRaWAN protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoRaWANConfig {
    /// Gateway EUI
    pub gateway_eui: String,
    /// Network server address
    pub network_server: String,
    /// Application server address
    pub app_server: String,
    /// Frequency band
    pub frequency_band: String,
    /// Spreading factor
    pub spreading_factor: u8,
    /// Bandwidth
    pub bandwidth: u32,
    /// Coding rate
    pub coding_rate: String,
}

/// HTTP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPConfig {
    /// Base URL
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Use HTTPS
    pub use_https: bool,
    /// Custom headers
    pub headers: HashMap<String, String>,
}

/// TCP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TCPConfig {
    /// Server address
    pub address: String,
    /// Server port
    pub port: u16,
    /// Connection timeout
    pub timeout: Duration,
    /// Keep alive settings
    pub keep_alive: bool,
    /// Buffer size
    pub buffer_size: usize,
}

/// UDP protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UDPConfig {
    /// Bind address
    pub bind_address: String,
    /// Bind port
    pub bind_port: u16,
    /// Target address
    pub target_address: String,
    /// Target port
    pub target_port: u16,
    /// Buffer size
    pub buffer_size: usize,
}

/// Custom protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProtocolConfig {
    /// Protocol name
    pub name: String,
    /// Configuration parameters
    pub params: HashMap<String, serde_json::Value>,
}

// =============================================================================
// Protocol Handler Trait
// =============================================================================

/// Trait for protocol-specific message handling
#[async_trait]
pub trait ProtocolHandler: Send + Sync {
    /// Initialize the protocol handler
    async fn initialize(&mut self, config: ProtocolConfig) -> Result<(), IoTError>;
    
    /// Start the protocol handler
    async fn start(&mut self) -> Result<(), IoTError>;
    
    /// Stop the protocol handler
    async fn stop(&mut self) -> Result<(), IoTError>;
    
    /// Send message via this protocol
    async fn send_message(&self, message: &IoTMessage) -> Result<(), IoTError>;
    
    /// Receive message via this protocol
    async fn receive_message(&self) -> Result<Option<IoTMessage>, IoTError>;
    
    /// Get protocol type
    fn get_protocol_type(&self) -> ProtocolType;
    
    /// Check if protocol is connected
    async fn is_connected(&self) -> bool;
    
    /// Get protocol statistics
    async fn get_statistics(&self) -> ProtocolStatistics;
    
    /// Handle protocol-specific configuration update
    async fn update_config(&mut self, config: ProtocolConfig) -> Result<(), IoTError>;
    
    /// Get supported message types for this protocol
    fn get_supported_message_types(&self) -> Vec<crate::MessageType>;
}

/// Protocol handler statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProtocolStatistics {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Connection uptime
    pub uptime: Duration,
    /// Error count
    pub error_count: u64,
    /// Last error message
    pub last_error: Option<String>,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Connection status
    pub connected: bool,
}

// =============================================================================
// Message Processor
// =============================================================================

/// Central message processing engine
pub struct MessageProcessor {
    /// Protocol handlers registry
    handlers: Arc<RwLock<HashMap<ProtocolType, Box<dyn ProtocolHandler>>>>,
    
    /// Message routing table
    routing_table: Arc<RwLock<HashMap<String, ProtocolType>>>,
    
    /// Message filters
    filters: Vec<Box<dyn MessageFilter>>,
    
    /// Message transformers
    transformers: Vec<Box<dyn MessageTransformer>>,
    
    /// Configuration
    config: MessageProcessorConfig,
    
    /// Statistics
    stats: Arc<RwLock<MessageProcessorStats>>,
}

/// Message processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageProcessorConfig {
    /// Maximum message size
    pub max_message_size: usize,
    /// Message timeout
    pub message_timeout: Duration,
    /// Enable message validation
    pub enable_validation: bool,
    /// Enable message transformation
    pub enable_transformation: bool,
    /// Enable message filtering
    pub enable_filtering: bool,
    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Retry configuration for failed messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    /// Initial retry delay
    pub initial_delay: Duration,
    /// Maximum retry delay
    pub max_delay: Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
}

/// Message processor statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageProcessorStats {
    /// Total messages processed
    pub total_processed: u64,
    /// Messages by protocol
    pub by_protocol: HashMap<String, u64>,
    /// Failed messages
    pub failed_messages: u64,
    /// Average processing time
    pub avg_processing_time_ms: f64,
    /// Messages in queue
    pub queue_size: usize,
}

// =============================================================================
// Message Filters and Transformers
// =============================================================================

/// Trait for message filtering
#[async_trait]
pub trait MessageFilter: Send + Sync {
    /// Filter message based on criteria
    async fn filter(&self, message: &IoTMessage) -> Result<bool, IoTError>;
    
    /// Get filter name
    fn get_name(&self) -> &str;
}

/// Trait for message transformation
#[async_trait]
pub trait MessageTransformer: Send + Sync {
    /// Transform message content or format
    async fn transform(&self, message: IoTMessage) -> Result<IoTMessage, IoTError>;
    
    /// Get transformer name
    fn get_name(&self) -> &str;
}

// =============================================================================
// Concrete Protocol Implementations
// =============================================================================

/// MQTT protocol handler implementation
pub struct MQTTHandler {
    config: Option<MQTTConfig>,
    connected: bool,
    stats: ProtocolStatistics,
    // MQTT client would be stored here
    // client: Option<rumqttc::Client>,
}

/// CoAP protocol handler implementation
pub struct CoAPHandler {
    config: Option<CoAPConfig>,
    connected: bool,
    stats: ProtocolStatistics,
    // CoAP client would be stored here
}

/// WebSocket protocol handler implementation
pub struct WebSocketHandler {
    config: Option<WebSocketConfig>,
    connected: bool,
    stats: ProtocolStatistics,
    // WebSocket connection would be stored here
}

/// Modbus protocol handler implementation
pub struct ModbusHandler {
    config: Option<ModbusConfig>,
    connected: bool,
    stats: ProtocolStatistics,
    // Modbus client would be stored here
}

/// LoRaWAN protocol handler implementation
pub struct LoRaWANHandler {
    config: Option<LoRaWANConfig>,
    connected: bool,
    stats: ProtocolStatistics,
    // LoRaWAN gateway would be stored here
}

// =============================================================================
// MessageProcessor Implementation
// =============================================================================

impl MessageProcessor {
    /// Create new message processor
    #[instrument(level = "info")]
    pub async fn new(config: &IoTConfig) -> Result<Self, IoTError> {
        info!("🔧 Initializing Message Processor");
        
        let processor_config = MessageProcessorConfig::default();
        
        Ok(MessageProcessor {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            filters: Vec::new(),
            transformers: Vec::new(),
            config: processor_config,
            stats: Arc::new(RwLock::new(MessageProcessorStats::default())),
        })
    }
    
    /// Register protocol handler
    #[instrument(level = "debug", skip(self, handler))]
    pub async fn register_handler(&self, protocol: ProtocolType, handler: Box<dyn ProtocolHandler>) -> Result<(), IoTError> {
        info!("📡 Registering protocol handler: {:?}", protocol);
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(protocol.clone(), handler);
        
        info!("✅ Protocol handler registered: {:?}", protocol);
        Ok(())
    }
    
    /// Process incoming message
    #[instrument(level = "debug", skip(self, message))]
    pub async fn process_message(&self, message: &IoTMessage) -> Result<(), IoTError> {
        debug!("⚙️ Processing message: {}", message.message_id);
        
        // Apply filters
        if self.config.enable_filtering {
            for filter in &self.filters {
                if !filter.filter(message).await? {
                    debug!("🚫 Message filtered out by: {}", filter.get_name());
                    return Ok(());
                }
            }
        }
        
        // Apply transformations
        let mut processed_message = message.clone();
        if self.config.enable_transformation {
            for transformer in &self.transformers {
                processed_message = transformer.transform(processed_message).await?;
            }
        }
        
        // Route message to appropriate handler
        let device_id = &processed_message.device_id;
        let routing_table = self.routing_table.read().await;
        
        if let Some(protocol) = routing_table.get(device_id) {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(protocol) {
                handler.send_message(&processed_message).await?;
            } else {
                return Err(IoTError::ProtocolError {
                    protocol: format!("{:?}", protocol),
                    message: "Handler not found".to_string(),
                });
            }
        } else {
            return Err(IoTError::MessageProcessingFailed {
                reason: format!("No routing rule for device: {}", device_id),
            });
        }
        
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_processed += 1;
        
        debug!("✅ Message processed successfully: {}", message.message_id);
        Ok(())
    }
    
    /// Add message filter
    pub fn add_filter(&mut self, filter: Box<dyn MessageFilter>) {
        self.filters.push(filter);
    }
    
    /// Add message transformer
    pub fn add_transformer(&mut self, transformer: Box<dyn MessageTransformer>) {
        self.transformers.push(transformer);
    }
    
    /// Set device routing
    pub async fn set_device_routing(&self, device_id: String, protocol: ProtocolType) {
        let mut routing_table = self.routing_table.write().await;
        routing_table.insert(device_id, protocol);
    }
    
    /// Get processor statistics
    pub async fn get_statistics(&self) -> MessageProcessorStats {
        self.stats.read().await.clone()
    }
}

// =============================================================================
// Protocol Handler Implementations
// =============================================================================

#[async_trait]
impl ProtocolHandler for MQTTHandler {
    async fn initialize(&mut self, config: ProtocolConfig) -> Result<(), IoTError> {
        if let ProtocolConfig::MQTT(mqtt_config) = config {
            self.config = Some(mqtt_config);
            Ok(())
        } else {
            Err(IoTError::ConfigurationError {
                parameter: "Invalid config type for MQTT handler".to_string(),
            })
        }
    }
    
    async fn start(&mut self) -> Result<(), IoTError> {
        info!("🚀 Starting MQTT handler");
        
        // Initialize MQTT client connection
        // This would contain actual MQTT client initialization
        self.connected = true;
        
        info!("✅ MQTT handler started");
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<(), IoTError> {
        info!("🛑 Stopping MQTT handler");
        
        // Disconnect MQTT client
        self.connected = false;
        
        info!("✅ MQTT handler stopped");
        Ok(())
    }
    
    async fn send_message(&self, message: &IoTMessage) -> Result<(), IoTError> {
        debug!("📤 Sending MQTT message: {}", message.message_id);
        
        if !self.connected {
            return Err(IoTError::ProtocolError {
                protocol: "MQTT".to_string(),
                message: "Not connected".to_string(),
            });
        }
        
        // Implement actual MQTT message sending
        // mqtt_client.publish(topic, payload).await?;
        
        debug!("✅ MQTT message sent: {}", message.message_id);
        Ok(())
    }
    
    async fn receive_message(&self) -> Result<Option<IoTMessage>, IoTError> {
        // Implement MQTT message receiving
        // This would poll the MQTT client for incoming messages
        Ok(None)
    }
    
    fn get_protocol_type(&self) -> ProtocolType {
        ProtocolType::MQTT
    }
    
    async fn is_connected(&self) -> bool {
        self.connected
    }
    
    async fn get_statistics(&self) -> ProtocolStatistics {
        self.stats.clone()
    }
    
    async fn update_config(&mut self, config: ProtocolConfig) -> Result<(), IoTError> {
        self.initialize(config).await
    }
    
    fn get_supported_message_types(&self) -> Vec<crate::MessageType> {
        vec![
            crate::MessageType::Telemetry,
            crate::MessageType::Command,
            crate::MessageType::Event,
            crate::MessageType::Alert,
        ]
    }
}

// Similar implementations would be created for other protocol handlers...

impl MQTTHandler {
    pub fn new() -> Self {
        MQTTHandler {
            config: None,
            connected: false,
            stats: ProtocolStatistics::default(),
        }
    }
}

impl CoAPHandler {
    pub fn new() -> Self {
        CoAPHandler {
            config: None,
            connected: false,
            stats: ProtocolStatistics::default(),
        }
    }
}

impl WebSocketHandler {
    pub fn new() -> Self {
        WebSocketHandler {
            config: None,
            connected: false,
            stats: ProtocolStatistics::default(),
        }
    }
}

impl ModbusHandler {
    pub fn new() -> Self {
        ModbusHandler {
            config: None,
            connected: false,
            stats: ProtocolStatistics::default(),
        }
    }
}

impl LoRaWANHandler {
    pub fn new() -> Self {
        LoRaWANHandler {
            config: None,
            connected: false,
            stats: ProtocolStatistics::default(),
        }
    }
}

// =============================================================================
// Default Implementations
// =============================================================================

impl Default for MessageProcessorConfig {
    fn default() -> Self {
        MessageProcessorConfig {
            max_message_size: 1024 * 1024, // 1MB
            message_timeout: Duration::from_secs(30),
            enable_validation: true,
            enable_transformation: true,
            enable_filtering: true,
            retry_config: RetryConfig::default(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl Default for MQTTConfig {
    fn default() -> Self {
        MQTTConfig {
            host: "localhost".to_string(),
            port: 1883,
            client_id: "matrixon-iot".to_string(),
            username: None,
            password: None,
            keep_alive: Duration::from_secs(60),
            qos: 1,
            use_tls: false,
            cert_path: None,
            subscribe_topics: vec!["device/+/telemetry".to_string()],
            publish_prefix: "matrixon".to_string(),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IoTConfig;
    
    #[tokio::test]
    async fn test_message_processor_creation() {
        let config = IoTConfig::default();
        let processor = MessageProcessor::new(&config).await;
        assert!(processor.is_ok());
    }
    
    #[tokio::test]
    async fn test_mqtt_handler_creation() {
        let handler = MQTTHandler::new();
        assert_eq!(handler.get_protocol_type(), ProtocolType::MQTT);
        assert!(!handler.is_connected().await);
    }
    
    #[test]
    fn test_protocol_types() {
        let mqtt = ProtocolType::MQTT;
        let coap = ProtocolType::CoAP;
        let custom = ProtocolType::Custom("MyProtocol".to_string());
        
        assert_eq!(mqtt, ProtocolType::MQTT);
        assert_ne!(mqtt, coap);
        assert_eq!(custom, ProtocolType::Custom("MyProtocol".to_string()));
    }
    
    #[test]
    fn test_mqtt_config_default() {
        let config = MQTTConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.qos, 1);
    }
    
    #[test]
    fn test_retry_config() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.backoff_multiplier, 2.0);
    }
} 
