//! Wallet Management Module
//!
//! Handles cryptographic wallet operations for blockchain interactions.
//! Author: arkSong (arksong2018@gmail.com)
//! Version: 0.1.0
//! Date: 2025-06-15

use web3::types::H160;
use secp256k1::{SecretKey, PublicKey};
use rand::rngs::OsRng;
use thiserror::Error;
use tracing::{info, instrument};
use tiny_keccak::{Hasher, Keccak};

/// Keccak-256 hash function
pub(crate) fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(data);
    hasher.finalize(&mut output);
    output
}

/// Main wallet type for blockchain operations
#[derive(Debug, Clone)]
pub struct Wallet {
    accounts: Vec<WalletAccount>,
}

/// Individual wallet account
#[derive(Debug, Clone)]
pub struct WalletAccount {
    address: H160,
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl Default for Wallet {
    fn default() -> Self {
        Self::new()
    }
}

impl Wallet {
    /// Create new wallet instance
    #[instrument(level = "debug")]
    pub fn new() -> Self {
        info!("🔧 Initializing Wallet");
        Self {
            accounts: Vec::new(),
        }
    }

    /// Generate new wallet account
    #[instrument(level = "debug")]
    pub fn create_account(&mut self) -> Result<H160, WalletError> {
        let secp = secp256k1::Secp256k1::new();
        let mut rng = OsRng;
        let secret_key = SecretKey::new(&mut rng);
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        
        let address = public_key_to_address(&public_key);
        self.accounts.push(WalletAccount {
            address,
            secret_key,
            public_key,
        });
        
        Ok(address)
    }

    /// Get list of managed addresses
    pub fn addresses(&self) -> Vec<H160> {
        self.accounts.iter().map(|a| a.address).collect()
    }
}

/// Convert public key to Ethereum address
fn public_key_to_address(pub_key: &PublicKey) -> H160 {
    let pub_key = pub_key.serialize_uncompressed();
    let hash = keccak256(&pub_key[1..]);
    H160::from_slice(&hash[12..])
}


/// Wallet-specific errors
#[derive(Error, Debug)]
pub enum WalletError {
    /// Key generation error
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    
    /// Invalid key format
    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_wallet_creation() {
        let mut wallet = Wallet::new();
        assert!(wallet.create_account().is_ok());
        assert_eq!(wallet.addresses().len(), 1);
    }

    #[test]
    fn test_new_random_wallet() {
        let mut wallet = Wallet::new();
        let addr1 = wallet.create_account().unwrap();
        let addr2 = wallet.create_account().unwrap();
        assert_ne!(addr1, addr2);
    }
}
