//! IPFS node
//! 
//! This module implements the IPFS node management functionality.
//! It provides methods for starting, stopping, and managing IPFS nodes.

use crate::{config::IpfsConfig, error::{Error, Result}};
use std::{process::Command, sync::Arc};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument};

/// IPFS node manager
#[derive(Debug)]
pub struct IpfsNode {
    /// Node configuration
    config: Arc<IpfsConfig>,
    /// Node process
    process: Arc<RwLock<Option<std::process::Child>>>,
    /// Node status
    status: Arc<RwLock<NodeStatus>>,
}

/// Node status
#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    /// Node is stopped
    Stopped,
    /// Node is starting
    Starting,
    /// Node is running
    Running,
    /// Node is stopping
    Stopping,
    /// Node has failed
    Failed(String),
}

impl IpfsNode {
    /// Create a new IPFS node manager
    #[instrument(level = "debug")]
    pub fn new(config: IpfsConfig) -> Self {
        debug!("🔧 Creating new IPFS node manager");
        let start = std::time::Instant::now();

        let node = Self {
            config: Arc::new(config),
            process: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(NodeStatus::Stopped)),
        };

        info!("✅ IPFS node manager created in {:?}", start.elapsed());
        node
    }

    /// Start the IPFS node
    #[instrument(level = "debug", skip(self))]
    pub async fn start(&self) -> Result<()> {
        debug!("🔧 Starting IPFS node");
        let start = std::time::Instant::now();

        let mut status = self.status.write().await;
        if *status != NodeStatus::Stopped {
            return Err(Error::InvalidState("Node is not stopped".to_string()));
        }
        *status = NodeStatus::Starting;

        let mut process = self.process.write().await;
        let child = Command::new("ipfs")
            .arg("daemon")
            .arg("--init")
            .arg("--routing=dht")
            .arg(format!("--api-address=/ip4/{}/tcp/{}", self.config.node_address, self.config.api_port))
            .arg(format!("--gateway-address=/ip4/{}/tcp/{}", self.config.node_address, self.config.gateway_port))
            .arg(format!("--swarm-address=/ip4/{}/tcp/{}", self.config.node_address, self.config.swarm_port))
            .spawn()?;

        *process = Some(child);
        *status = NodeStatus::Running;

        info!("✅ IPFS node started in {:?}", start.elapsed());
        Ok(())
    }

    /// Stop the IPFS node
    #[instrument(level = "debug", skip(self))]
    pub async fn stop(&self) -> Result<()> {
        debug!("🔧 Stopping IPFS node");
        let start = std::time::Instant::now();

        let mut status = self.status.write().await;
        if *status != NodeStatus::Running {
            return Err(Error::InvalidState("Node is not running".to_string()));
        }
        *status = NodeStatus::Stopping;

        let mut process = self.process.write().await;
        if let Some(mut child) = process.take() {
            child.kill()?;
            child.wait()?;
        }

        *status = NodeStatus::Stopped;

        info!("✅ IPFS node stopped in {:?}", start.elapsed());
        Ok(())
    }

    /// Get node status
    #[instrument(level = "debug", skip(self))]
    pub async fn status(&self) -> NodeStatus {
        self.status.read().await.clone()
    }

    /// Check if node is running
    #[instrument(level = "debug", skip(self))]
    pub async fn is_running(&self) -> bool {
        *self.status.read().await == NodeStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_creation() {
        let config = IpfsConfig::default();
        let node = IpfsNode::new(config);
        assert_eq!(node.status().await, NodeStatus::Stopped);
    }

    #[tokio::test]
    async fn test_node_status() {
        let config = IpfsConfig::default();
        let node = IpfsNode::new(config);
        assert!(!node.is_running().await);
    }
} 
