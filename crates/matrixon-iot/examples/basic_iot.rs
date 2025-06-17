//! # Matrixon IoT Basic Example
//!
//! Demonstrates basic IoT functionality with device registration and communication.

use matrixon_iot::{
    IoTManager, DeviceConfig, ProtocolType, DeviceType, DeviceCapability,
    IoTMessage, MessageType, QualityOfService, MessagePriority
};
use uuid::Uuid;
use std::collections::HashMap;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::init();
    
    println!("🚀 Matrixon IoT Basic Example");
    println!("=============================");
    
    // Create IoT manager
    println!("📡 Initializing IoT Manager...");
    let mut iot_manager = IoTManager::new().await?;
    
    // Register a temperature sensor device
    println!("📱 Registering temperature sensor...");
    let temp_sensor_config = DeviceConfig::new("temp_sensor_001", ProtocolType::MQTT)
        .with_name("Office Temperature Sensor")
        .with_device_type(DeviceType::Sensor)
        .with_location(37.7749, -122.4194) // San Francisco coordinates
        .with_authentication("secure_token_123")
        .with_capability(DeviceCapability::TemperatureSensing)
        .with_capability(DeviceCapability::DataLogging)
        .with_metadata("location", "Office Room 101")
        .with_metadata("department", "Engineering")
        .with_firmware_version("1.2.3");
    
    let device_id = iot_manager.register_device(temp_sensor_config).await?;
    println!("✅ Temperature sensor registered: {}", device_id);
    
    // Register a humidity sensor device
    println!("📱 Registering humidity sensor...");
    let humidity_sensor_config = DeviceConfig::new("humidity_sensor_001", ProtocolType::CoAP)
        .with_name("Office Humidity Sensor")
        .with_device_type(DeviceType::Sensor)
        .with_location(37.7749, -122.4194)
        .with_authentication("secure_token_456")
        .with_capability(DeviceCapability::HumiditySensing)
        .with_capability(DeviceCapability::EdgeComputing)
        .with_metadata("location", "Office Room 101")
        .with_metadata("department", "Engineering")
        .with_firmware_version("2.1.0");
    
    let humidity_device_id = iot_manager.register_device(humidity_sensor_config).await?;
    println!("✅ Humidity sensor registered: {}", humidity_device_id);
    
    // Register an industrial actuator
    println!("📱 Registering industrial actuator...");
    let actuator_config = DeviceConfig::new("actuator_001", ProtocolType::Modbus)
        .with_name("Industrial Pump Controller")
        .with_device_type(DeviceType::Industrial)
        .with_location(37.7849, -122.4094)
        .with_authentication("industrial_token_789")
        .with_capability(DeviceCapability::RemoteControl)
        .with_capability(DeviceCapability::FirmwareUpdate)
        .with_metadata("machine_type", "Water Pump")
        .with_metadata("zone", "Manufacturing Floor A")
        .with_firmware_version("3.0.1");
    
    let actuator_device_id = iot_manager.register_device(actuator_config).await?;
    println!("✅ Industrial actuator registered: {}", actuator_device_id);
    
    // Start message processing
    println!("⚙️ Starting message processing...");
    iot_manager.start_processing().await?;
    
    // Simulate IoT messages
    println!("📤 Simulating IoT messages...");
    
    // Temperature sensor message
    let temp_message = IoTMessage {
        message_id: Uuid::new_v4(),
        device_id: device_id.clone(),
        timestamp: Utc::now(),
        message_type: MessageType::Telemetry,
        payload: serde_json::json!({
            "temperature": 22.5,
            "unit": "celsius",
            "sensor_id": "DHT22_001"
        }),
        qos: QualityOfService::AtLeastOnce,
        topic: "sensors/temperature/office".to_string(),
        priority: MessagePriority::Normal,
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("room".to_string(), "101".to_string());
            meta.insert("floor".to_string(), "2".to_string());
            meta
        },
        correlation_id: None,
    };
    
    // Humidity sensor message
    let humidity_message = IoTMessage {
        message_id: Uuid::new_v4(),
        device_id: humidity_device_id.clone(),
        timestamp: Utc::now(),
        message_type: MessageType::Telemetry,
        payload: serde_json::json!({
            "humidity": 45.2,
            "unit": "percent",
            "sensor_id": "SHT30_001"
        }),
        qos: QualityOfService::ExactlyOnce,
        topic: "sensors/humidity/office".to_string(),
        priority: MessagePriority::Normal,
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("room".to_string(), "101".to_string());
            meta.insert("calibrated".to_string(), "true".to_string());
            meta
        },
        correlation_id: None,
    };
    
    // Industrial actuator command
    let actuator_command = IoTMessage {
        message_id: Uuid::new_v4(),
        device_id: actuator_device_id.clone(),
        timestamp: Utc::now(),
        message_type: MessageType::Command,
        payload: serde_json::json!({
            "action": "set_pump_speed",
            "speed_percent": 75,
            "duration_seconds": 300
        }),
        qos: QualityOfService::ExactlyOnce,
        topic: "actuators/pump/control".to_string(),
        priority: MessagePriority::High,
        metadata: {
            let mut meta = HashMap::new();
            meta.insert("operator".to_string(), "system_auto".to_string());
            meta.insert("reason".to_string(), "scheduled_maintenance".to_string());
            meta
        },
        correlation_id: Some(Uuid::new_v4()),
    };
    
    // Send messages (this would typically be done by the protocol handlers)
    println!("📊 Processing telemetry messages...");
    println!("  🌡️  Temperature: {:.1}°C", temp_message.payload["temperature"]);
    println!("  💧 Humidity: {:.1}%", humidity_message.payload["humidity"]);
    println!("  ⚙️  Pump Command: Set speed to {}%", actuator_command.payload["speed_percent"]);
    
    // Get IoT statistics
    println!("\n📈 IoT System Statistics:");
    let stats = iot_manager.get_statistics().await;
    println!("  Connected Devices: {}", stats.connected_devices);
    println!("  Messages Processed: {}", stats.messages_processed);
    println!("  Average Latency: {:.2}ms", stats.avg_latency_ms);
    println!("  Error Count: {}", stats.error_count);
    
    // Demonstrate device capabilities
    println!("\n🛠️ Device Capabilities Summary:");
    println!("  Temperature Sensor: Temperature sensing, Data logging");
    println!("  Humidity Sensor: Humidity sensing, Edge computing");
    println!("  Industrial Actuator: Remote control, Firmware updates");
    
    // Simulate adding gateways and edge nodes
    println!("\n🌐 Adding IoT Gateway...");
    let gateway_config = matrixon_iot::GatewayConfig {
        gateway_id: "gateway_001".to_string(),
        name: "Main Building Gateway".to_string(),
        location: Some((37.7749, -122.4194)),
        max_devices: 1000,
    };
    
    let gateway_id = iot_manager.add_gateway(gateway_config).await?;
    println!("✅ Gateway added: {}", gateway_id);
    
    println!("\n⚡ Adding Edge Processing Node...");
    let edge_config = matrixon_iot::EdgeConfig {
        node_id: "edge_001".to_string(),
        name: "Manufacturing Edge Node".to_string(),
        compute_capacity: 100,
        memory_limit: 1024 * 1024 * 1024, // 1GB
    };
    
    let edge_id = iot_manager.add_edge_node(edge_config).await?;
    println!("✅ Edge node added: {}", edge_id);
    
    // Display final statistics
    println!("\n📊 Final System State:");
    let final_stats = iot_manager.get_statistics().await;
    println!("  Total Connected Devices: {}", final_stats.connected_devices);
    println!("  Active Gateways: {}", final_stats.gateway_stats.len());
    println!("  Edge Nodes: {}", final_stats.edge_stats.len());
    
    // Graceful shutdown
    println!("\n🛑 Shutting down IoT Manager...");
    iot_manager.shutdown().await?;
    println!("✅ Shutdown completed successfully!");
    
    println!("\n🎉 Matrixon IoT Basic Example completed!");
    Ok(())
} 
