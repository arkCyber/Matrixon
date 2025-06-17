//! Blockchain Event Subscription Module
//!
//! Provides functionality for subscribing to Ethereum blockchain events.
//! Author: arkSong (arksong2018@gmail.com)
//! Version: 0.1.0
//! Date: 2025-06-15

use web3::{
    types::{Address, Filter, FilterBuilder, H256},
    Transport,
};
use web3::Web3;
use thiserror::Error;
use tracing::{info, instrument};

/// Event subscriber
#[derive(Debug, Clone)]
pub struct EventSubscriber<T: Transport> {
    subscriptions: Vec<Filter>,
    web3: web3::Web3<T>,
}

impl<T: Transport> EventSubscriber<T> {
    /// Create new event subscriber
    #[instrument(level = "debug")]
    pub fn new(web3: Web3<T>) -> Self {
        info!("🔧 Initializing EventSubscriber");
        Self {
            subscriptions: Vec::new(),
            web3,
        }
    }

    /// Subscribe to contract events
    #[instrument(level = "debug")]
    pub async fn subscribe(
        &mut self,
        address: Address,
        topics: Vec<H256>,
    ) -> Result<(), EventError> {
        let filter = FilterBuilder::default()
            .address(vec![address])
            .topics(Some(topics), None, None, None)
            .build();

        self.subscriptions.push(filter);
        Ok(())
    }

    /// Poll for new events
    #[instrument(level = "debug")]
    pub async fn poll(&self) -> Result<Vec<web3::types::Log>, EventError> {
        let mut all_logs = Vec::new();
        for filter in &self.subscriptions {
            let logs = self.web3.eth().logs(filter.clone()).await?;
            all_logs.extend(logs);
        }
        Ok(all_logs)
    }
}

/// Event-specific errors
#[derive(Error, Debug)]
pub enum EventError {
    /// Web3 event error
    #[error("Web3 event error: {0}")]
    Web3Error(#[from] web3::Error),

    /// Invalid filter parameters
    #[error("Invalid filter parameters: {0}")]
    InvalidFilter(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;
    use test_log::test;

    #[test]
    fn test_event_subscriber_creation() {
        let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
        let web3 = web3::Web3::new(transport);
        let subscriber = EventSubscriber::new(web3);
        assert_eq!(subscriber.subscriptions.len(), 0);
    }
}
