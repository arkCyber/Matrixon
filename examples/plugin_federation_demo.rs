/**
 * @file examples/plugin_federation_demo.rs
 * @brief Plugin System and Federation Demo for matrixon Matrix Server
 * @author matrixon Contributors
 * @date 2025-01-27
 * @version 1.0.0
 * 
 * Comprehensive demonstration of the dynamic plugin system and
 * advanced federation capabilities of matrixon Matrix server.
 * 
 * This demo showcases:
 * - Plugin loading and management
 * - Multi-protocol bridge creation
 * - Federation server management
 * - Real-time event monitoring
 * - Performance statistics
 */

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use tokio::time::sleep;
use tracing::{info, warn, error};

// Import our plugin and federation systems
use matrixon::service::{
    plugins::{
        PluginMetadata, PluginCapability, PluginResourceLimits, PluginHook,
        PluginDependency, manager::{PluginManager, PluginManagerConfig}
    },
    federation::{
        FederationManager, FederationConfig, BridgeType, BridgeConfig
    }
};

/**
 * Demo plugin metadata for testing
 */
fn create_demo_plugin() -> PluginMetadata {
    PluginMetadata {
        id: "demo_discord_bridge".to_string(),
        name: "Discord Bridge Plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "Bridges Matrix rooms with Discord channels".to_string(),
        author: "matrixon Demo Team".to_string(),
        min_api_version: "1.0.0".to_string(),
        max_api_version: "1.0.0".to_string(),
        dependencies: vec![
            PluginDependency {
                plugin_id: "core_federation".to_string(),
                version_range: "1.0.0".to_string(),
                optional: false,
            }
        ],
        capabilities: vec![
            PluginCapability::RoomEvents,
            PluginCapability::FederationAccess,
            PluginCapability::NetworkAccess,
        ],
        resource_limits: PluginResourceLimits {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_cpu_percent: 25.0,
            max_threads: 4,
            max_bandwidth: 5 * 1024 * 1024, // 5MB/s
            max_file_descriptors: 32,
        },
        config_schema: Some(serde_json::to_string(&serde_json::json!({
            "type": "object",
            "properties": {
                "discord_token": {
                    "type": "string",
                    "description": "Discord bot token"
                },
                "enabled_guilds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of Discord guild IDs to bridge"
                }
            },
            "required": ["discord_token"]
        })).unwrap()),
        hooks: vec![
            PluginHook {
                name: "on_message".to_string(),
                event_type: "room.message".to_string(),
                priority: 10,
                can_modify: true,
            },
            PluginHook {
                name: "on_member_join".to_string(),
                event_type: "room.member".to_string(),
                priority: 5,
                can_modify: false,
            }
        ],
    }
}

/**
 * Demo bridge configuration
 */
fn create_discord_bridge_config() -> BridgeConfig {
    let mut credentials = HashMap::new();
    credentials.insert("token".to_string(), "demo_discord_bot_token".to_string());
    
    let mut settings = HashMap::new();
    settings.insert("auto_bridge_rooms".to_string(), serde_json::Value::Bool(true));
    settings.insert("sync_avatars".to_string(), serde_json::Value::Bool(true));
    settings.insert("sync_typing".to_string(), serde_json::Value::Bool(false));
    
    BridgeConfig {
        name: "Discord Bridge".to_string(),
        endpoint: "https://discord.com/api/v10".to_string(),
        credentials,
        settings,
        features: vec![
            "message_sync".to_string(),
            "member_sync".to_string(),
            "media_sync".to_string(),
        ],
    }
}

/**
 * Main demo function
 */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting matrixon Plugin & Federation Demo");
    
    // 1. Plugin System Demo
    info!("\n=== Plugin System Demo ===");
    
    // Create plugin manager
    let plugin_config = PluginManagerConfig::default();
    let plugin_manager = PluginManager::new(plugin_config).await?;
    
    info!("✅ Plugin Manager initialized");
    
    // Load demo plugin
    let demo_plugin = create_demo_plugin();
    info!("🔌 Loading demo plugin: {}", demo_plugin.name);
    
    plugin_manager.load_plugin(demo_plugin).await?;
    info!("✅ Demo plugin loaded successfully");
    
    // List loaded plugins
    let plugins = plugin_manager.list_plugins().await?;
    info!("📋 Loaded plugins ({}):", plugins.len());
    for plugin in &plugins {
        info!("   - {} v{} by {}", plugin.name, plugin.version, plugin.author);
        info!("     Capabilities: {:?}", plugin.capabilities);
        info!("     Memory limit: {} MB", plugin.resource_limits.max_memory / 1024 / 1024);
    }
    
    // Test plugin hook execution
    info!("🎯 Testing plugin hook execution...");
    let hook_context = matrixon::service::plugins::manager::PluginContext {
        request_id: "demo-request-001".to_string(),
        user_id: Some("@demo_user:matrix.org".to_string()),
        room_id: Some("!demo_room:matrix.org".to_string()),
        event_data: serde_json::json!({
            "type": "m.room.message",
            "content": {
                "msgtype": "m.text",
                "body": "Hello from demo!"
            }
        }),
        metadata: HashMap::new(),
        timestamp: std::time::Instant::now(),
    };
    
    let hook_results = plugin_manager.execute_hook("on_message", hook_context).await?;
    info!("✅ Hook executed, {} results returned", hook_results.len());
    
    // Show plugin statistics
    sleep(Duration::from_millis(100)).await; // Let stats update
    let plugin_stats = plugin_manager.get_plugin_stats(&plugins[0].id).await?;
    info!("📊 Plugin statistics:");
    info!("   - Hook executions: {}", plugin_stats.hook_executions);
    info!("   - Average execution time: {:?}", plugin_stats.avg_execution_time);
    info!("   - Memory usage: {} bytes", plugin_stats.memory_usage);
    
    // 2. Federation System Demo
    info!("\n=== Federation System Demo ===");
    
    // Create federation manager
    let mut federation_config = FederationConfig::default();
    federation_config.server_name = "demo.matrix.org".to_string();
    federation_config.enable_bridges = true;
    federation_config.enable_advanced_features = true;
    
    let federation_manager = FederationManager::new(federation_config).await?;
    info!("✅ Federation Manager initialized");
    
    // Start federation services
    federation_manager.start().await?;
    info!("✅ Federation services started");
    
    // Add federated servers
    let demo_servers = vec![
        "matrix.org".to_string(),
        "element.io".to_string(),
        "t2bot.io".to_string(),
    ];
    
    info!("🔗 Adding federated servers...");
    for server in &demo_servers {
        federation_manager.add_server(server.clone()).await?;
        info!("   + Added server: {}", server);
    }
    
    // Create bridges
    info!("🌉 Creating demo bridges...");
    
    // Discord bridge
    let discord_config = create_discord_bridge_config();
    let discord_bridge_id = federation_manager
        .create_bridge(BridgeType::Discord, discord_config)
        .await?;
    info!("   + Discord bridge created: {}", discord_bridge_id);
    
    // Telegram bridge
    let mut telegram_credentials = HashMap::new();
    telegram_credentials.insert("bot_token".to_string(), "demo_telegram_token".to_string());
    
    let telegram_config = BridgeConfig {
        name: "Telegram Bridge".to_string(),
        endpoint: "https://api.telegram.org".to_string(),
        credentials: telegram_credentials,
        settings: HashMap::new(),
        features: vec!["message_sync".to_string()],
    };
    
    let telegram_bridge_id = federation_manager
        .create_bridge(BridgeType::Telegram, telegram_config)
        .await?;
    info!("   + Telegram bridge created: {}", telegram_bridge_id);
    
    // List federated servers
    let servers = federation_manager.list_servers().await;
    info!("📋 Federated servers ({}):", servers.len());
    for server in &servers {
        info!("   - {} ({})", server.server_name, 
              match server.status {
                  matrixon::service::federation::ServerStatus::Online => "🟢 Online",
                  matrixon::service::federation::ServerStatus::Offline => "🔴 Offline",
                  matrixon::service::federation::ServerStatus::Unknown => "⚪ Unknown",
                  _ => "⚫ Other"
              });
        info!("     Trust level: {:?}", server.trust_level);
        info!("     Federation version: {}", server.federation_version);
    }
    
    // List active bridges
    let bridges = federation_manager.list_bridges().await;
    info!("📋 Active bridges ({}):", bridges.len());
    for bridge in &bridges {
        info!("   - {} ({:?})", bridge.config.name, bridge.bridge_type);
        info!("     Status: {:?}", bridge.status);
        info!("     Rooms: {}", bridge.rooms.len());
        info!("     Active rooms: {}", bridge.stats.active_rooms);
    }
    
    // 3. Event Monitoring Demo
    info!("\n=== Event Monitoring Demo ===");
    
    // Subscribe to plugin events
    let mut plugin_events = plugin_manager.subscribe_events();
    
    // Subscribe to federation events
    let mut federation_events = federation_manager.subscribe_events();
    
    info!("📡 Monitoring events for 5 seconds...");
    
    // Monitor events for a short time
    let monitor_duration = Duration::from_secs(5);
    let monitor_start = std::time::Instant::now();
    
    while monitor_start.elapsed() < monitor_duration {
        tokio::select! {
            Ok(plugin_event) = plugin_events.recv() => {
                match plugin_event {
                    matrixon::service::plugins::manager::PluginEvent::PluginLoaded(id) => {
                        info!("🔌 Plugin loaded: {}", id);
                    },
                    matrixon::service::plugins::manager::PluginEvent::HookExecuted(plugin_id, hook, duration) => {
                        info!("🎯 Hook executed: {}::{} in {:?}", plugin_id, hook, duration);
                    },
                    _ => {}
                }
            },
            Ok(federation_event) = federation_events.recv() => {
                match federation_event {
                    matrixon::service::federation::FederationEvent::ServerDiscovered(server) => {
                        info!("🔍 Server discovered: {}", server);
                    },
                    matrixon::service::federation::FederationEvent::BridgeConnected(bridge_id, bridge_type) => {
                        info!("🌉 Bridge connected: {} ({:?})", bridge_id, bridge_type);
                    },
                    _ => {}
                }
            },
            _ = sleep(Duration::from_millis(100)) => {
                // Continue monitoring
            }
        }
    }
    
    // 4. Performance Statistics
    info!("\n=== Performance Statistics ===");
    
    // Plugin manager stats
    let plugin_manager_stats = plugin_manager.get_manager_stats().await;
    info!("🔌 Plugin Manager Statistics:");
    info!("   - Total plugins loaded: {}", plugin_manager_stats.total_loaded);
    info!("   - Active plugins: {}", plugin_manager_stats.active_plugins);
    info!("   - Total executions: {}", plugin_manager_stats.total_executions);
    info!("   - Total execution time: {:?}", plugin_manager_stats.total_execution_time);
    info!("   - Error count: {}", plugin_manager_stats.error_count);
    
    // Federation stats
    let federation_stats = federation_manager.get_stats().await;
    info!("🌐 Federation Statistics:");
    info!("   - Total servers: {}", federation_stats.total_servers);
    info!("   - Online servers: {}", federation_stats.online_servers);
    info!("   - Total bridges: {}", federation_stats.total_bridges);
    info!("   - Active bridges: {}", federation_stats.active_bridges);
    info!("   - Events federated: {}", federation_stats.total_events);
    info!("   - Success rate: {:.2}%", federation_stats.success_rate);
    
    // 5. Cleanup Demo
    info!("\n=== Cleanup Demo ===");
    
    // Unload plugin
    if let Some(plugin) = plugins.first() {
        info!("🗑️ Unloading plugin: {}", plugin.id);
        plugin_manager.unload_plugin(&plugin.id).await?;
        info!("✅ Plugin unloaded successfully");
    }
    
    info!("\n🎉 Demo completed successfully!");
    info!("📝 Summary:");
    info!("   ✅ Plugin system: Dynamic loading, hook execution, resource management");
    info!("   ✅ Federation system: Multi-protocol bridges, server management");
    info!("   ✅ Event monitoring: Real-time event broadcasting and handling");
    info!("   ✅ Performance tracking: Comprehensive statistics and metrics");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_demo_plugin_creation() {
        let plugin = create_demo_plugin();
        assert_eq!(plugin.id, "demo_discord_bridge");
        assert_eq!(plugin.name, "Discord Bridge Plugin");
        assert!(!plugin.dependencies.is_empty());
        assert!(!plugin.capabilities.is_empty());
        assert!(!plugin.hooks.is_empty());
    }
    
    #[test]
    fn test_discord_bridge_config() {
        let config = create_discord_bridge_config();
        assert_eq!(config.name, "Discord Bridge");
        assert!(config.credentials.contains_key("token"));
        assert!(!config.features.is_empty());
    }
} 
