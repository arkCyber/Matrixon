// =============================================================================
// Matrixon Matrix NextServer - Bot Example
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Date: 2024-03-19
// Version: 0.11.0-alpha
// License: Apache 2.0 / MIT
//
// Description:
//   Example usage of Matrixon bot with configuration file
//
// =============================================================================

use matrixon_bot::Service;
use matrixon_core::error::Result;
use std::path::PathBuf;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Get configuration file path
    let config_path = PathBuf::from("config/default.json");
    
    info!("Starting Matrixon bot with config: {:?}", config_path);
    
    // Create and start bot service
    let bot = Service::from_config_file(&config_path).await?;
    
    // Start the bot
    if let Err(e) = bot.start().await {
        error!("Failed to start bot: {}", e);
        return Err(e);
    }
    
    info!("Bot started successfully");
    
    // Keep the main thread alive
    tokio::signal::ctrl_c().await?;
    
    info!("Shutting down bot...");
    Ok(())
} 
