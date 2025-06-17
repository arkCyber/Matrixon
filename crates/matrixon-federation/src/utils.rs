// =============================================================================
// Matrixon Federation - Utils Module
// =============================================================================
//
// Author: arkSong <arksong2018@gmail.com>
// Version: 0.11.0-alpha
// Date: 2024-03-21
//
// This module provides utility functions for the Matrixon federation
// implementation. It includes functions for server discovery, authentication,
// event exchange, and state resolution.
//
// =============================================================================

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use ruma::{
    api::federation::{
        discovery::ServerSigningKeys,
        authentication::ServerAuthentication,
    },
    events::AnyRoomEvent,
    RoomId, ServerName, UserId,
};
use serde_json::Value;
use tracing::{debug, info, instrument, warn};

use crate::error::{Result, FederationError};

/// Returns the current timestamp in milliseconds since Unix epoch
#[instrument(level = "debug")]
pub fn get_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

/// Generates a new Ed25519 keypair for Matrix federation
#[instrument(level = "debug")]
pub fn generate_keypair() -> Result<Ed25519KeyPair> {
    let rng = SystemRandom::new();
    Ed25519KeyPair::generate_pkcs8(&rng)
        .map_err(|e| FederationError::Internal(format!("Failed to generate keypair: {}", e)))
        .and_then(|pkcs8| {
            Ed25519KeyPair::from_pkcs8(pkcs8.as_ref())
                .map_err(|e| FederationError::Internal(format!("Failed to create keypair: {}", e)))
        })
}

/// Verifies a server's signature
#[instrument(level = "debug")]
pub fn verify_server_signature(
    server_name: &ServerName,
    message: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<bool> {
    let public_key = ring::signature::UnparsedPublicKey::new(
        &ring::signature::ED25519,
        public_key,
    );
    
    public_key
        .verify(message, signature)
        .map(|_| true)
        .map_err(|e| FederationError::Auth(format!("Invalid signature: {}", e)))
}

/// Calculates a hash of multiple byte arrays
#[instrument(level = "debug")]
pub fn calculate_hash(parts: &[&[u8]]) -> Result<String> {
    use ring::digest::{Context, SHA256};
    
    let mut context = Context::new(&SHA256);
    for part in parts {
        context.update(part);
    }
    
    let digest = context.finish();
    Ok(BASE64.encode(digest.as_ref()))
}

/// Finds common elements across multiple iterators
#[instrument(level = "debug")]
pub fn common_elements<T: Eq + Clone + 'static>(
    iterators: Vec<Box<dyn Iterator<Item = T> + '_>>,
) -> Vec<T> {
    if iterators.is_empty() {
        return vec![];
    }
    
    let mut result: Vec<T> = iterators[0].clone().collect();
    
    for iterator in iterators.iter().skip(1) {
        let current: Vec<T> = iterator.clone().collect();
        result.retain(|item| current.contains(item));
    }
    
    result
}

/// Converts a value to canonical JSON
#[instrument(level = "debug")]
pub fn to_canonical_object(value: &Value) -> Result<Vec<u8>> {
    serde_json::to_vec(value)
        .map_err(|e| FederationError::Json(format!("Failed to serialize JSON: {}", e)))
}

/// Deserializes a value from a string
#[instrument(level = "debug")]
pub fn deserialize_from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s)
        .map_err(|e| FederationError::Json(format!("Failed to deserialize JSON: {}", e)))
}

/// Wrapper for HTML escaping strings
#[derive(Debug, Clone)]
pub struct HtmlEscape<'a>(pub &'a str);

impl<'a> std::fmt::Display for HtmlEscape<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for c in self.0.chars() {
            match c {
                '&' => write!(f, "&amp;")?,
                '<' => write!(f, "&lt;")?,
                '>' => write!(f, "&gt;")?,
                '"' => write!(f, "&quot;")?,
                '\'' => write!(f, "&#39;")?,
                c => write!(f, "{}", c)?,
            }
        }
        Ok(())
    }
}

/// Generates a random alphanumeric string
#[instrument(level = "debug")]
pub fn random_string(length: usize) -> Result<String> {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    
    let mut rng = rand::thread_rng();
    let result: String = (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    
    Ok(result)
}

/// Calculates a password hash using Argon2
#[instrument(level = "debug")]
pub fn calculate_password_hash(password: &str) -> Result<String> {
    use argon2::{
        password_hash::{
            rand_core::OsRng,
            PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        },
        Argon2,
    };
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| FederationError::Internal(format!("Failed to hash password: {}", e)))?;
    
    Ok(password_hash.to_string())
}

/// Verifies a password hash
#[instrument(level = "debug")]
pub fn verify_password_hash(password: &str, hash: &str) -> Result<bool> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| FederationError::Internal(format!("Invalid password hash: {}", e)))?;
    
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    
    #[test]
    fn test_get_timestamp() {
        let timestamp = get_timestamp();
        assert!(timestamp > 0);
    }
    
    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair().unwrap();
        assert_eq!(keypair.public_key().as_ref().len(), 32);
    }
    
    #[test]
    fn test_verify_server_signature() {
        let keypair = generate_keypair().unwrap();
        let message = b"test message";
        let signature = keypair.sign(message);
        
        let result = verify_server_signature(
            &server_name!("example.com"),
            message,
            signature.as_ref(),
            keypair.public_key().as_ref(),
        );
        
        assert!(result.unwrap());
    }
    
    #[test]
    fn test_calculate_hash() {
        let parts = vec![b"part1", b"part2", b"part3"];
        let hash = calculate_hash(&parts.iter().map(|p| *p).collect::<Vec<_>>()).unwrap();
        assert!(!hash.is_empty());
    }
    
    #[test]
    fn test_common_elements() {
        let v1 = vec![1, 2, 3, 4, 5];
        let v2 = vec![2, 4, 6, 8, 10];
        let v3 = vec![2, 3, 5, 7, 11];
        
        let iterators = vec![
            Box::new(v1.into_iter()) as Box<dyn Iterator<Item = i32>>,
            Box::new(v2.into_iter()),
            Box::new(v3.into_iter()),
        ];
        
        let common = common_elements(iterators);
        assert_eq!(common, vec![2]);
    }
    
    #[test]
    fn test_to_canonical_object() {
        let value = serde_json::json!({
            "key": "value",
            "number": 42,
            "array": [1, 2, 3]
        });
        
        let result = to_canonical_object(&value).unwrap();
        assert!(!result.is_empty());
    }
    
    #[test]
    fn test_deserialize_from_str() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Test {
            key: String,
            value: i32,
        }
        
        let json = r#"{"key": "test", "value": 42}"#;
        let result: Test = deserialize_from_str(json).unwrap();
        
        assert_eq!(
            result,
            Test {
                key: "test".to_string(),
                value: 42
            }
        );
    }
    
    #[test]
    fn test_html_escape() {
        let input = "<script>alert('test')</script>";
        let escaped = HtmlEscape(input).to_string();
        
        assert_eq!(
            escaped,
            "&lt;script&gt;alert(&#39;test&#39;)&lt;/script&gt;"
        );
    }
    
    #[test]
    fn test_random_string() {
        let length = 16;
        let result = random_string(length).unwrap();
        
        assert_eq!(result.len(), length);
        assert!(result.chars().all(|c| c.is_ascii_alphanumeric()));
    }
    
    #[test]
    fn test_password_hash() {
        let password = "test_password";
        let hash = calculate_password_hash(password).unwrap();
        
        assert!(verify_password_hash(password, &hash).unwrap());
        assert!(!verify_password_hash("wrong_password", &hash).unwrap());
    }
} 
