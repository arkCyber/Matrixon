//! A2A Crypto Module
//! 
//! This module implements cryptographic operations for A2A protocol.
//! 
//! Author: arkSong <arksong2018@gmail.com>
//! Version: 0.1.0
//! License: MIT

use crate::error::Error;
use crate::message::Message;
use tracing::{info, instrument};
use std::time::Instant;
use std::sync::Arc;
use tokio::sync::RwLock;
use ed25519_dalek::{SigningKey, VerifyingKey, SecretKey, Signature, Verifier};
use ed25519_dalek::Signer;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::rngs::OsRng;
use rand::Rng;

/// Crypto configuration
#[derive(Debug, Clone)]
pub struct CryptoConfig {
    /// Signing key
    pub signing_key: Option<SigningKey>,
    /// Verifying key
    pub verifying_key: Option<VerifyingKey>,
    /// Secret key
    pub secret_key: Option<SecretKey>,
}

/// Crypto implementation
pub struct Crypto {
    /// Crypto configuration
    config: CryptoConfig,
    /// Signing key
    signing_key: Arc<RwLock<Option<SigningKey>>>,
    /// Verifying key
    verifying_key: Arc<RwLock<Option<VerifyingKey>>>,
    /// Secret key
    secret_key: Arc<RwLock<Option<SecretKey>>>,
    /// AES-GCM cipher
    cipher: Arc<RwLock<Option<Aes256Gcm>>>,
}

impl std::fmt::Debug for Crypto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crypto")
            .field("config", &self.config)
            .field("signing_key", &self.signing_key)
            .field("verifying_key", &self.verifying_key)
            .field("secret_key", &self.secret_key)
            .field("cipher", &"<Aes256Gcm>")
            .finish()
    }
}

impl Crypto {
    /// Create a new crypto instance
    #[instrument(level = "debug")]
    pub fn new(config: CryptoConfig) -> Self {
        Self {
            config,
            signing_key: Arc::new(RwLock::new(None)),
            verifying_key: Arc::new(RwLock::new(None)),
            secret_key: Arc::new(RwLock::new(None)),
            cipher: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize crypto
    #[instrument(level = "debug", skip(self))]
    pub async fn init(&self) -> Result<(), Error> {
        let start = Instant::now();
        info!("🔧 Initializing crypto");

        // Initialize signing key
        let signing_key = self.config.signing_key.clone().unwrap_or_else(|| {
            let mut rng = OsRng;
            let mut bytes = [0u8; 32];
            rng.fill(&mut bytes);
            SigningKey::from_bytes(&bytes)
        });

        let mut signing_key_guard = self.signing_key.write().await;
        *signing_key_guard = Some(signing_key.clone());

        // Initialize verifying key
        let mut verifying_key_guard = self.verifying_key.write().await;
        *verifying_key_guard = Some(signing_key.verifying_key());

        // Initialize secret key
        let mut secret_key_guard = self.secret_key.write().await;
        *secret_key_guard = Some(signing_key.to_bytes());

        // Initialize AES-GCM cipher
        let secret_key = secret_key_guard.as_ref().unwrap();
        let key = Key::<Aes256Gcm>::from_slice(secret_key.as_ref());
        let cipher = Aes256Gcm::new(key);
        let mut cipher_guard = self.cipher.write().await;
        *cipher_guard = Some(cipher);

        info!("✅ Crypto initialized in {:?}", start.elapsed());
        Ok(())
    }

    /// Encrypt a message
    #[instrument(level = "debug", skip(self, message))]
    pub async fn encrypt(&self, message: &Message) -> Result<Vec<u8>, Error> {
        let start = Instant::now();
        info!("🔧 Encrypting message");

        let cipher = self.cipher.read().await;
        let cipher = cipher.as_ref().ok_or_else(|| {
            Error::Crypto("Cipher not initialized".to_string())
        })?;

        let message_json = serde_json::to_vec(message)
            .map_err(|e| Error::Serialization(e))?;

        let nonce = Nonce::from_slice(&[0u8; 12]);
        let ciphertext = cipher
            .encrypt(nonce, message_json.as_ref())
            .map_err(|e| Error::Crypto(e.to_string()))?;

        info!("✅ Message encrypted in {:?}", start.elapsed());
        Ok(ciphertext)
    }

    /// Decrypt a message
    #[instrument(level = "debug", skip(self, ciphertext))]
    pub async fn decrypt(&self, ciphertext: &[u8]) -> Result<Message, Error> {
        let start = Instant::now();
        info!("🔧 Decrypting message");

        let cipher = self.cipher.read().await;
        let cipher = cipher.as_ref().ok_or_else(|| {
            Error::Crypto("Cipher not initialized".to_string())
        })?;

        let nonce = Nonce::from_slice(&[0u8; 12]);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Crypto(e.to_string()))?;

        let message = serde_json::from_slice(&plaintext)
            .map_err(|e| Error::Serialization(e))?;

        info!("✅ Message decrypted in {:?}", start.elapsed());
        Ok(message)
    }

    /// Sign data
    #[instrument(level = "debug", skip(self, data))]
    pub async fn sign(&self, data: &[u8]) -> Result<Signature, Error> {
        let start = Instant::now();
        info!("🔧 Signing data");

        let signing_key = self.signing_key.read().await;
        let signing_key = signing_key.as_ref().ok_or_else(|| {
            Error::Crypto("Signing key not initialized".to_string())
        })?;

        let signature = signing_key.sign(data);

        info!("✅ Data signed in {:?}", start.elapsed());
        Ok(signature)
    }

    /// Verify signature
    #[instrument(level = "debug", skip(self, data, signature))]
    pub async fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool, Error> {
        let start = Instant::now();
        info!("🔧 Verifying signature");

        let verifying_key = self.verifying_key.read().await;
        let verifying_key = verifying_key.as_ref().ok_or_else(|| {
            Error::Crypto("Verifying key not initialized".to_string())
        })?;

        let result = verifying_key.verify(data, signature).is_ok();

        info!("✅ Signature verified in {:?}", start.elapsed());
        Ok(result)
    }
}
