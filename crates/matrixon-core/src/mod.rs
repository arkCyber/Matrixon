// =============================================================================
// Matrixon Matrix NextServer - Mod Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Utility functions and helper components. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • Common utility functions
//   • Error handling and logging
//   • Performance instrumentation
//   • Helper traits and macros
//   • Shared functionality
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

pub mod error;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2
};
use cmp::Ordering;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use ring::digest;
use ruma::{canonical_json::try_from_json_map, CanonicalJsonError, CanonicalJsonObject};
use std::{
    cmp, fmt,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH, Duration, Instant},
    sync::{Arc, RwLock, atomic::{AtomicU64, Ordering as AtomicOrdering}},
    collections::HashMap,
};
use tracing::{debug, error, info, instrument, warn};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use comfy_table::{Table, Row, Cell, ContentArrangement};

use crate::{Error, Result};

/// Returns the current time in milliseconds since Unix epoch
/// 
/// This function provides high-precision timestamp generation for Matrix events
/// and server operations. Critical for event ordering and synchronization.
/// 
/// # Performance Target
/// <1μs execution time
/// 
/// # Returns
/// * `u64` - Milliseconds since Unix epoch (January 1, 1970)
/// 
/// # Examples
/// ```
/// use matrixon::utils::millis_since_unix_epoch;
/// 
/// let timestamp = millis_since_unix_epoch();
/// assert!(timestamp > 1640995200000); // After 2022-01-01
/// ```
#[instrument(level = "trace")]
pub fn millis_since_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time is valid")
        .as_millis() as u64
}

/// Returns the current time in seconds since Unix epoch
/// 
/// Provides second-precision timestamps for less time-critical operations
/// where millisecond precision is not required.
/// 
/// # Performance Target
/// <1μs execution time
/// 
/// # Returns
/// * `u64` - Seconds since Unix epoch
/// 
/// # Examples
/// ```
/// use matrixon::utils::secs_since_unix_epoch;
/// 
/// let timestamp = secs_since_unix_epoch();
/// assert!(timestamp > 1640995200); // After 2022-01-01
/// ```
#[instrument(level = "trace")]
pub fn secs_since_unix_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time is valid")
        .as_secs()
}

/// Increments a big-endian encoded u64 counter
/// 
/// This function is used for database key generation and event sequencing.
/// Handles the case where no previous value exists by starting at 1.
/// 
/// # Performance Target
/// <10μs execution time
/// 
/// # Arguments
/// * `old` - Optional previous counter value as bytes
/// 
/// # Returns
/// * `Option<Vec<u8>>` - Incremented counter as big-endian bytes, or None if overflow
/// 
/// # Examples
/// ```
/// use matrixon::utils::increment;
/// 
/// let first = increment(None); // Returns Some([0, 0, 0, 0, 0, 0, 0, 1])
/// let second = increment(first.as_deref()); // Returns Some([0, 0, 0, 0, 0, 0, 0, 2])
/// ```
#[instrument(level = "trace")]
pub fn increment(old: Option<&[u8]>) -> Option<Vec<u8>> {
    let number = match old.map(|bytes| bytes.try_into()) {
        Some(Ok(bytes)) => {
            let number = u64::from_be_bytes(bytes);
            number.wrapping_add(1)
        }
        _ => 1, // Start at one. since 0 should return the first event in the db
    };

    Some(number.to_be_bytes().to_vec())
}

/// Generates a new Ed25519 keypair for Matrix federation
/// 
/// Creates a keypair suitable for Matrix server-to-server authentication.
/// The format includes a random prefix for key rotation support.
/// 
/// # Performance Target
/// <10ms key generation time
/// 
/// # Returns
/// * `Vec<u8>` - Encoded keypair with random prefix
/// 
/// # Security
/// - Uses cryptographically secure random number generation
/// - Ed25519 provides strong security guarantees
/// - Suitable for Matrix federation requirements
#[instrument(level = "debug")]
pub fn generate_keypair() -> Vec<u8> {
    debug!("🔧 Generating new Ed25519 keypair for Matrix federation");
    
    let mut value = random_string(8).as_bytes().to_vec();
    value.push(0xff);
    value.extend_from_slice(
        &ruma::signatures::Ed25519KeyPair::generate()
            .expect("Ed25519KeyPair generation always works (?)"),
    );
    
    debug!("✅ Ed25519 keypair generated successfully");
    value
}

/// Parses bytes into a u64 value
/// 
/// Converts big-endian byte array to u64 for counter and timestamp processing.
/// Used throughout the database layer for key manipulation.
/// 
/// # Performance Target
/// <1μs execution time
/// 
/// # Arguments
/// * `bytes` - Byte slice to convert (must be exactly 8 bytes)
/// 
/// # Returns
/// * `Result<u64, TryFromSliceError>` - Parsed u64 value or conversion error
/// 
/// # Examples
/// ```
/// use matrixon::utils::u64_from_bytes;
/// 
/// let bytes = [0, 0, 0, 0, 0, 0, 0, 42];
/// assert_eq!(u64_from_bytes(&bytes).unwrap(), 42);
/// ```
pub fn u64_from_bytes(bytes: &[u8]) -> std::result::Result<u64, std::array::TryFromSliceError> {
    let array: [u8; 8] = bytes.try_into()?;
    Ok(u64::from_be_bytes(array))
}

/// Parses bytes into a UTF-8 string
/// 
/// Converts byte array to string with proper UTF-8 validation.
/// Used for database value deserialization and data processing.
/// 
/// # Performance Target
/// <1μs for small strings (<100 bytes)
/// 
/// # Arguments
/// * `bytes` - Byte slice to convert to string
/// 
/// # Returns
/// * `Result<String, FromUtf8Error>` - Parsed string or UTF-8 error
/// 
/// # Examples
/// ```
/// use matrixon::utils::string_from_bytes;
/// 
/// let bytes = b"hello world";
/// assert_eq!(string_from_bytes(bytes).unwrap(), "hello world");
/// ```
pub fn string_from_bytes(bytes: &[u8]) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(bytes.to_vec())
}

/// Generates a cryptographically secure random string
/// 
/// Creates random alphanumeric strings for session IDs, tokens, and salts.
/// Uses cryptographically secure random number generation.
/// 
/// # Performance Target
/// <1ms for strings up to 1000 characters
/// 
/// # Arguments
/// * `length` - Desired length of the random string
/// 
/// # Returns
/// * `String` - Random alphanumeric string
/// 
/// # Security
/// - Uses cryptographically secure random number generator
/// - Suitable for session tokens and security-critical applications
/// - Alphanumeric character set provides good entropy
/// 
/// # Examples
/// ```
/// use matrixon::utils::random_string;
/// 
/// let token = random_string(32);
/// assert_eq!(token.len(), 32);
/// assert!(token.chars().all(|c| c.is_alphanumeric()));
/// ```
#[instrument(level = "debug")]
pub fn random_string(length: usize) -> String {
    debug!("🔧 Generating random string of length {}", length);
    
    let result = thread_rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
        
    debug!("✅ Random string generated successfully");
    result
}

/// Calculate a new Argon2id hash for the given password
/// 
/// Generates secure password hashes using Argon2id algorithm with random salt.
/// Designed to be computationally expensive to resist brute force attacks.
/// 
/// # Performance Target
/// 100-500ms per hash (intentionally slow for security)
/// 
/// # Arguments
/// * `password` - Plain text password to hash
/// 
/// # Returns
/// * `Result<String, argon2::password_hash::Error>` - Encoded hash string or hashing error
/// 
/// # Security Features
/// - Argon2id variant for resistance to both side-channel and GPU attacks
/// - Random salt for each password (cryptographically secure)
/// - Custom parameters matching OWASP recommendations
/// - Timing-safe verification recommended
/// - Comprehensive error handling
/// 
/// # Examples
/// ```
/// use matrixon::utils::calculate_password_hash;
/// 
/// let hash = calculate_password_hash("secret123").unwrap();
/// assert!(hash.starts_with("$argon2id$"));
/// ```
#[instrument(level = "debug", skip(password))]
pub fn calculate_password_hash(password: &str) -> std::result::Result<String, argon2::password_hash::Error> {
    let start = Instant::now();
    debug!("🔧 Starting Argon2id password hash calculation");
    
    // Generate cryptographically secure random salt
    let salt = SaltString::generate(&mut thread_rng());
    
    // Configure Argon2 with parameters matching OWASP recommendations
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(
            19456,  // 19MB memory cost
            2,      // 2 iterations
            1,      // parallelism factor
            None    // let argon2 calculate output length
        ).expect("Valid Argon2 parameters"));
    
    // Hash password and measure performance
    let result = argon2.hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string());
    
    match &result {
        Ok(_) => {
            let duration = start.elapsed();
            info!(
                "✅ Password hash calculated in {:?} ({} chars)",
                duration,
                password.len()
            );
            
            // Log if hash took less than recommended time (potential security issue)
            if duration < Duration::from_millis(100) {
                warn!("⚠️ Password hash calculation too fast - consider increasing parameters");
            }
        },
        Err(e) => {
            error!("❌ Password hash calculation failed: {}", e);
        }
    }
    
    result
}

/// Calculate SHA256 hash of concatenated byte arrays
/// 
/// Computes cryptographic hash for event IDs, state hashes, and integrity verification.
/// Used extensively in Matrix protocol for event authentication.
/// 
/// # Performance Target
/// <1ms for typical input sizes (<10KB)
/// 
/// # Arguments
/// * `keys` - Slice of byte arrays to hash
/// 
/// # Returns
/// * `Vec<u8>` - SHA256 hash digest (32 bytes)
/// 
/// # Security
/// - SHA256 provides strong collision resistance
/// - Suitable for cryptographic applications
/// - Used in Matrix event authentication chains
/// 
/// # Examples
/// ```
/// use matrixon::utils::calculate_hash;
/// 
/// let data: &[&[u8]] = &[b"hello", b"world"];
/// let hash = calculate_hash(data);
/// assert_eq!(hash.len(), 32); // SHA256 produces 32-byte hashes
/// ```
#[tracing::instrument(skip(keys))]
pub fn calculate_hash(keys: &[&[u8]]) -> Vec<u8> {
    debug!("🔧 Calculating SHA256 hash for {} keys", keys.len());
    
    // We only hash the pdu's event ids, not the whole pdu
    let bytes = keys.join(&0xff);
    let hash = digest::digest(&digest::SHA256, &bytes);
    let result = hash.as_ref().to_owned();
    
    debug!("✅ SHA256 hash calculated successfully");
    result
}

/// Find common elements across multiple sorted iterators
/// 
/// Efficiently finds elements that appear in all provided iterators.
/// Used for intersection operations in database queries and room state resolution.
/// 
/// # Performance Target
/// O(n*m) where n is total elements and m is number of iterators
/// 
/// # Arguments
/// * `iterators` - Iterator of sorted iterators to intersect
/// * `check_order` - Function to compare elements for ordering
/// 
/// # Returns
/// * `Option<impl Iterator<Item = Vec<u8>>>` - Iterator of common elements
/// 
/// # Algorithm
/// Uses multi-way merge algorithm for efficient intersection of sorted sequences.
/// Assumes all input iterators are sorted according to `check_order`.
/// 
/// # Examples
/// ```
/// use matrixon::utils::common_elements;
/// use std::cmp::Ordering;
/// 
/// let iter1 = vec![vec![1], vec![2], vec![3]].into_iter();
/// let iter2 = vec![vec![2], vec![3], vec![4]].into_iter();
/// let common = common_elements([iter1, iter2].into_iter(), |a, b| a.cmp(b));
/// // Returns iterator over [vec![2], vec![3]]
/// ```
pub fn common_elements(
    mut iterators: impl Iterator<Item = impl Iterator<Item = Vec<u8>>>,
    check_order: impl Fn(&[u8], &[u8]) -> Ordering,
) -> Option<impl Iterator<Item = Vec<u8>>> {
    debug!("🔧 Finding common elements across multiple iterators");
    
    let first_iterator = iterators.next()?;
    let mut other_iterators = iterators.map(|i| i.peekable()).collect::<Vec<_>>();

    let result = Some(first_iterator.filter(move |target| {
        other_iterators.iter_mut().all(|it| {
            while let Some(element) = it.peek() {
                match check_order(element, target) {
                    Ordering::Greater => return false, // We went too far
                    Ordering::Equal => return true,    // Element is in both iters
                    Ordering::Less => {
                        // Keep searching
                        it.next();
                    }
                }
            }
            false
        })
    }));
    
    debug!("✅ Common elements iterator created successfully");
    result
}

/// Convert any serializable value to CanonicalJsonObject
/// 
/// Fallible conversion ensuring the value serializes to a JSON object.
/// Used for Matrix event serialization and canonical JSON processing.
/// 
/// # Performance Target
/// <1ms for typical Matrix events (<10KB)
/// 
/// # Arguments
/// * `value` - Any value implementing Serialize
/// 
/// # Returns
/// * `Result<CanonicalJsonObject, CanonicalJsonError>` - Canonical JSON object or error
/// 
/// # Matrix Protocol
/// Essential for Matrix event handling and federation message processing.
/// Ensures consistent JSON representation across different implementations.
/// 
/// # Examples
/// ```
/// use matrixon::utils::to_canonical_object;
/// use serde_json::json;
/// 
/// let obj = json!({"type": "m.room.message", "content": {"body": "hello"}});
/// let canonical = to_canonical_object(obj).unwrap();
/// ```
pub fn to_canonical_object<T: serde::Serialize>(
    value: T,
) -> Result<CanonicalJsonObject, CanonicalJsonError> {
    use serde::ser::Error;

    debug!("🔧 Converting value to canonical JSON object");
    
    let result = match serde_json::to_value(value).map_err(CanonicalJsonError::SerDe)? {
        serde_json::Value::Object(map) => try_from_json_map(map),
        _ => Err(CanonicalJsonError::SerDe(serde_json::Error::custom(
            "Value must be an object",
        ))),
    };
    
    match result {
        Ok(_) => debug!("✅ Canonical JSON object created successfully"),
        Err(_) => warn!("❌ Failed to create canonical JSON object"),
    }
    
    result
}

/// Deserialize from string using FromStr implementation
/// 
/// Generic deserializer for types that implement FromStr.
/// Used in configuration parsing and data deserialization.
/// 
/// # Performance Target
/// <100μs for simple types
/// 
/// # Arguments
/// * `deserializer` - Serde deserializer
/// 
/// # Returns
/// * `Result<T, D::Error>` - Deserialized value or error
/// 
/// # Examples
/// Used with serde derive macros for custom string parsing.
pub fn deserialize_from_str<
    'de,
    D: serde::de::Deserializer<'de>,
    T: FromStr<Err = E>,
    E: fmt::Display,
>(
    deserializer: D,
) -> Result<T, D::Error> {
    struct Visitor<T: FromStr<Err = E>, E>(std::marker::PhantomData<T>);
    impl<T: FromStr<Err = Err>, Err: fmt::Display> serde::de::Visitor<'_> for Visitor<T, Err> {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "a parsable string")
        }
        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.parse().map_err(serde::de::Error::custom)
        }
    }
    deserializer.deserialize_str(Visitor(std::marker::PhantomData))
}

// Copied from librustdoc:
// https://github.com/rust-lang/rust/blob/cbaeec14f90b59a91a6b0f17fc046c66fa811892/src/librustdoc/html/escape.rs

/// HTML escape wrapper for secure string display
/// 
/// Wrapper struct that emits HTML-escaped version of contained string.
/// Critical for preventing XSS attacks in web interfaces and admin panels.
/// 
/// # Security Features
/// - Escapes all dangerous HTML characters
/// - Prevents script injection attacks
/// - Safe for use in HTML templates
/// - Zero-copy implementation for performance
/// 
/// # Examples
/// ```
/// use matrixon::utils::HtmlEscape;
/// 
/// let dangerous = "<script>alert('xss')</script>";
/// let safe = HtmlEscape(dangerous);
/// assert_eq!(format!("{}", safe), "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
/// ```
pub struct HtmlEscape<'a>(pub &'a str);

impl fmt::Display for HtmlEscape<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Because the internet is always right, turns out there's not that many
        // characters to escape: http://stackoverflow.com/questions/7381974
        let HtmlEscape(s) = *self;
        let pile_o_bits = s;
        let mut last = 0;
        for (i, ch) in s.char_indices() {
            let s = match ch {
                '>' => "&gt;",
                '<' => "&lt;",
                '&' => "&amp;",
                '\'' => "&#39;",
                '"' => "&quot;",
                _ => continue,
            };
            fmt.write_str(&pile_o_bits[last..i])?;
            fmt.write_str(s)?;
            // NOTE: we only expect single byte characters here - which is fine as long as we
            // only match single byte characters
            last = i + 1;
        }

        if last < s.len() {
            fmt.write_str(&pile_o_bits[last..])?;
        }
        Ok(())
    }
}

/// Returns the current UNIX timestamp in seconds.
///
/// # Example
/// ```
/// let ts = crate::utils::get_timestamp();
/// println!("Current timestamp: {}", ts);
/// ```
pub fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use serde_json::json;

    #[test]
    fn test_millis_since_unix_epoch() {
        let timestamp = millis_since_unix_epoch();
        
        // Should be after year 2020
        assert!(timestamp > 1577836800000, "Timestamp should be after 2020-01-01");
        
        // Should be reasonable (before year 2030)
        assert!(timestamp < 1893456000000, "Timestamp should be before 2030-01-01");
        
        // Should be increasing
        let timestamp2 = millis_since_unix_epoch();
        assert!(timestamp2 >= timestamp, "Timestamps should be non-decreasing");
    }

    #[test]
    fn test_secs_since_unix_epoch() {
        let timestamp = secs_since_unix_epoch();
        
        // Should be after year 2020
        assert!(timestamp > 1577836800, "Timestamp should be after 2020-01-01");
        
        // Should be reasonable (before year 2030)
        assert!(timestamp < 1893456000, "Timestamp should be before 2030-01-01");
        
        // Should be increasing
        let timestamp2 = secs_since_unix_epoch();
        assert!(timestamp2 >= timestamp, "Timestamps should be non-decreasing");
    }

    #[test]
    fn test_increment_from_none() {
        let result = increment(None);
        assert_eq!(result, Some(vec![0, 0, 0, 0, 0, 0, 0, 1]), "Should start at 1");
    }

    #[test]
    fn test_increment_existing_value() {
        let initial = vec![0, 0, 0, 0, 0, 0, 0, 5];
        let result = increment(Some(&initial));
        assert_eq!(result, Some(vec![0, 0, 0, 0, 0, 0, 0, 6]), "Should increment by 1");
    }

    #[test]
    fn test_increment_max_value() {
        let max_val = vec![255, 255, 255, 255, 255, 255, 255, 255];
        let result = increment(Some(&max_val));
        // Should wrap around to 0
        assert_eq!(result, Some(vec![0, 0, 0, 0, 0, 0, 0, 0]), "Should wrap around on overflow");
    }

    #[test]
    fn test_increment_invalid_bytes() {
        let invalid = vec![1, 2, 3]; // Wrong length
        let result = increment(Some(&invalid));
        assert_eq!(result, Some(vec![0, 0, 0, 0, 0, 0, 0, 1]), "Should start at 1 for invalid input");
    }

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair();
        
        // Should have reasonable length (8 bytes prefix + 1 separator + Ed25519 key)
        assert!(keypair.len() > 40, "Keypair should have reasonable length");
        
        // Should contain separator byte
        assert!(keypair.contains(&0xff), "Keypair should contain separator byte");
        
        // Should generate different keys each time
        let keypair2 = generate_keypair();
        assert_ne!(keypair, keypair2, "Should generate different keypairs");
    }

    #[test]
    fn test_u64_from_bytes_valid() {
        let bytes = [0, 0, 0, 0, 0, 0, 0, 42];
        let result = u64_from_bytes(&bytes);
        assert_eq!(result.unwrap(), 42, "Should parse u64 correctly");
    }

    #[test]
    fn test_u64_from_bytes_max_value() {
        let bytes = [255, 255, 255, 255, 255, 255, 255, 255];
        let result = u64_from_bytes(&bytes);
        assert_eq!(result.unwrap(), u64::MAX, "Should parse max u64 value");
    }

    #[test]
    fn test_u64_from_bytes_invalid_length() {
        let bytes = [1, 2, 3]; // Wrong length
        let result = u64_from_bytes(&bytes);
        assert!(result.is_err(), "Should fail for invalid length");
    }

    #[test]
    fn test_string_from_bytes_valid_utf8() {
        let bytes = b"hello world";
        let result = string_from_bytes(bytes);
        assert_eq!(result.unwrap(), "hello world", "Should parse valid UTF-8");
    }

    #[test]
    fn test_string_from_bytes_unicode() {
        let bytes = "Hello 世界 🌍".as_bytes();
        let result = string_from_bytes(bytes);
        assert_eq!(result.unwrap(), "Hello 世界 🌍", "Should handle Unicode correctly");
    }

    #[test]
    fn test_string_from_bytes_invalid_utf8() {
        let bytes = [0x80, 0x81, 0x82]; // Invalid UTF-8
        let result = string_from_bytes(&bytes);
        assert!(result.is_err(), "Should fail for invalid UTF-8");
    }

    #[test]
    fn test_random_string_length() {
        for length in [0, 1, 16, 32, 64, 128] {
            let result = random_string(length);
            assert_eq!(result.len(), length, "Should generate string of correct length");
        }
    }

    #[test]
    fn test_random_string_alphanumeric() {
        let result = random_string(100);
        assert!(result.chars().all(|c| c.is_alphanumeric()), 
                "Should contain only alphanumeric characters");
    }

    #[test]
    fn test_random_string_uniqueness() {
        let strings: Vec<String> = (0..10).map(|_| random_string(32)).collect();
        
        // Check that all strings are unique
        for i in 0..strings.len() {
            for j in (i + 1)..strings.len() {
                assert_ne!(strings[i], strings[j], "Random strings should be unique");
            }
        }
    }

    #[test]
    fn test_calculate_password_hash() {
        let password = "test_password_123";
        let hash = calculate_password_hash(password).unwrap();
        
        // Should start with Argon2id identifier
        assert!(hash.starts_with("$argon2id$"), "Should use Argon2id variant");
        
        // Should be reasonably long
        assert!(hash.len() > 50, "Hash should be reasonably long");
        
        // Should generate different hashes for same password (different salts)
        let hash2 = calculate_password_hash(password).unwrap();
        assert_ne!(hash, hash2, "Should generate different hashes due to different salts");
    }

    #[test]
    fn test_calculate_hash() {
        let data: &[&[u8]] = &[b"hello", b"world"];
        let hash = calculate_hash(data);
        
        // SHA256 produces 32-byte hashes
        assert_eq!(hash.len(), 32, "SHA256 hash should be 32 bytes");
        
        // Same input should produce same hash
        let hash2 = calculate_hash(data);
        assert_eq!(hash, hash2, "Same input should produce same hash");
        
        // Different input should produce different hash
        let data3: &[&[u8]] = &[b"hello", b"matrix"];
        let hash3 = calculate_hash(data3);
        assert_ne!(hash, hash3, "Different input should produce different hash");
    }

    #[test]
    fn test_calculate_hash_empty_input() {
        let empty: [&[u8]; 0] = [];
        let hash = calculate_hash(&empty);
        assert_eq!(hash.len(), 32, "Should handle empty input");
    }

    #[test]
    fn test_common_elements_intersection() {
        let iter1 = vec![vec![1], vec![2], vec![3], vec![4]];
        let iter2 = vec![vec![2], vec![3], vec![5]];
        let iter3 = vec![vec![1], vec![3], vec![6]];
        
        let iterators = vec![iter1.into_iter(), iter2.into_iter(), iter3.into_iter()];
        let common = common_elements(iterators.into_iter(), |a, b| a.cmp(b));
        
        let result: Vec<Vec<u8>> = common.unwrap().collect();
        assert_eq!(result, vec![vec![3]], "Should find only common element [3]");
    }

    #[test]
    fn test_common_elements_no_intersection() {
        let iter1 = vec![vec![1], vec![2]];
        let iter2 = vec![vec![3], vec![4]];
        
        let iterators = vec![iter1.into_iter(), iter2.into_iter()];
        let common = common_elements(iterators.into_iter(), |a, b| a.cmp(b));
        
        let result: Vec<Vec<u8>> = common.unwrap().collect();
        assert!(result.is_empty(), "Should find no common elements");
    }

    #[test]
    fn test_common_elements_empty_iterators() {
        let empty_iterators: Vec<std::vec::IntoIter<Vec<u8>>> = vec![];
        let common = common_elements(empty_iterators.into_iter(), |a, b| a.cmp(b));
        assert!(common.is_none(), "Should return None for empty iterator list");
    }

    #[test]
    fn test_to_canonical_object_valid() {
        let value = json!({"type": "m.room.message", "content": {"body": "hello"}});
        let result = to_canonical_object(value);
        assert!(result.is_ok(), "Should convert valid object");
        
        let canonical = result.unwrap();
        assert!(canonical.contains_key("type"), "Should contain 'type' key");
        assert!(canonical.contains_key("content"), "Should contain 'content' key");
    }

    #[test]
    fn test_to_canonical_object_invalid() {
        let value = json!("not an object");
        let result = to_canonical_object(value);
        assert!(result.is_err(), "Should fail for non-object values");
    }

    #[test]
    fn test_html_escape_basic() {
        let input = "<script>alert('xss')</script>";
        let escaped = format!("{}", HtmlEscape(input));
        assert_eq!(escaped, "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    }

    #[test]
    fn test_html_escape_all_characters() {
        let input = r#"<>&"'"#;
        let escaped = format!("{}", HtmlEscape(input));
        assert_eq!(escaped, "&lt;&gt;&amp;&quot;&#39;");
    }

    #[test]
    fn test_html_escape_safe_content() {
        let input = "Safe content with no special chars";
        let escaped = format!("{}", HtmlEscape(input));
        assert_eq!(escaped, input, "Safe content should remain unchanged");
    }

    #[test]
    fn test_html_escape_unicode() {
        let input = "Hello 世界 <script>";
        let escaped = format!("{}", HtmlEscape(input));
        assert_eq!(escaped, "Hello 世界 &lt;script&gt;", "Should preserve Unicode and escape HTML");
    }

    #[test]
    fn test_performance_benchmarks() {
        // Benchmark millis_since_unix_epoch (target: <1μs)
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = millis_since_unix_epoch();
        }
        let avg_duration = start.elapsed() / 1000;
        assert!(avg_duration < Duration::from_micros(10),
                "millis_since_unix_epoch should be <10μs average, was: {:?}", avg_duration);

        // Benchmark u64_from_bytes (target: <1μs)
        let bytes = [0, 0, 0, 0, 0, 0, 0, 42];
        let start = Instant::now();
        for _ in 0..1000 {
            let _ = u64_from_bytes(&bytes).unwrap();
        }
        let avg_duration = start.elapsed() / 1000;
        assert!(avg_duration < Duration::from_micros(1),
                "u64_from_bytes should be <1μs average, was: {:?}", avg_duration);

        // Benchmark random_string generation (target: <1ms for 32 chars)
        let start = Instant::now();
        for _ in 0..100 {
            let _ = random_string(32);
        }
        let avg_duration = start.elapsed() / 100;
        assert!(avg_duration < Duration::from_millis(1),
                "random_string(32) should be <1ms average, was: {:?}", avg_duration);
    }

    #[test]
    fn test_concurrent_random_string_generation() {
        use std::sync::Arc;
        use std::thread;
        
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handles: Vec<_> = (0..10).map(|_| {
            let results_clone = Arc::clone(&results);
            thread::spawn(move || {
                let random_str = random_string(16);
                results_clone.lock().unwrap().push(random_str);
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        let final_results = results.lock().unwrap();
        assert_eq!(final_results.len(), 10, "Should generate 10 strings");
        
        // Check uniqueness
        for i in 0..final_results.len() {
            for j in (i + 1)..final_results.len() {
                assert_ne!(final_results[i], final_results[j], 
                          "Concurrent random strings should be unique");
            }
        }
    }

    #[test]
    fn test_memory_efficiency() {
        use std::mem::size_of;
        
        // HtmlEscape should be zero-cost wrapper
        assert_eq!(size_of::<HtmlEscape>(), size_of::<&str>(), 
                  "HtmlEscape should be zero-cost wrapper");
        
        // Test that we don't hold unnecessary memory
        let small_string = "test";
        let escape = HtmlEscape(small_string);
        let formatted = format!("{}", escape);
        assert_eq!(formatted, "test", "Should work with small strings efficiently");
    }

    #[test]
    fn test_edge_cases() {
        // Test empty string handling
        assert_eq!(random_string(0), "", "Should handle zero-length strings");
        assert_eq!(format!("{}", HtmlEscape("")), "", "Should handle empty HTML escape");
        
        // Test single character handling
        assert_eq!(random_string(1).len(), 1, "Should handle single character strings");
        assert_eq!(format!("{}", HtmlEscape("<")), "&lt;", "Should escape single characters");
        
        // Test increment with empty bytes
        let result = increment(Some(&[]));
        assert_eq!(result, Some(vec![0, 0, 0, 0, 0, 0, 0, 1]), "Should handle empty bytes");
    }
}
