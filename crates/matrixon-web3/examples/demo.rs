//! Matrixon Web3 Demo
//!
//! Author: arkSong (arksong2018@gmail.com)
//! Date: 2025-06-15
//! Version: 0.1.0
//!
//! Demonstrates basic Web3 functionality integration with Matrixon.

use matrixon_web3::{client::Web3Client, wallet::Wallet};
use std::env;
use tracing::info;
use tracing_subscriber;
use web3::transports::Http;
use web3::Web3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    info!("🔧 Starting Matrixon Web3 demo");
    
    // Initialize Web3 client
    let rpc_url = env::var("WEB3_RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    let transport = Http::new(&rpc_url)?;
    let web3 = Web3::new(transport);
    let client = Web3Client::new(web3);
    
    // Create a new wallet account
    let mut wallet = Wallet::new();
    let address = wallet.create_account().map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    info!("✅ Created new wallet: {}", address);
    
    // Get latest block number
    let block_number = client.block_number().await?;
    info!("✅ Current block number: {}", block_number);
    
    info!("🎉 Demo completed successfully");
    Ok(())
}
