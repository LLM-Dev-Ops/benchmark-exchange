//! Cryptography utilities.
//!
//! This module provides utilities for password hashing, token generation,
//! and checksum verification.

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use blake3::Hasher as Blake3Hasher;
use rand::Rng;
use sha2::{Digest, Sha256};

/// Hash a password using Argon2.
///
/// # Arguments
///
/// * `password` - The plaintext password to hash
///
/// # Examples
///
/// ```
/// use common::crypto::hash_password;
///
/// let hash = hash_password("my_secure_password").expect("Failed to hash password");
/// println!("Hash: {}", hash);
/// ```
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .context("Failed to hash password")?
        .to_string();

    Ok(password_hash)
}

/// Verify a password against a hash.
///
/// # Arguments
///
/// * `password` - The plaintext password to verify
/// * `hash` - The password hash to verify against
///
/// # Examples
///
/// ```
/// use common::crypto::{hash_password, verify_password};
///
/// let hash = hash_password("my_password").expect("Failed to hash");
/// assert!(verify_password("my_password", &hash).expect("Failed to verify"));
/// assert!(!verify_password("wrong_password", &hash).expect("Failed to verify"));
/// ```
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash).context("Invalid password hash format")?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate a cryptographically secure random token.
///
/// # Arguments
///
/// * `length` - The length of the token in bytes
///
/// # Examples
///
/// ```
/// use common::crypto::generate_token;
///
/// let token = generate_token(32);
/// assert_eq!(token.len(), 64); // 32 bytes = 64 hex characters
/// ```
pub fn generate_token(length: usize) -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Checksum verifier supporting multiple algorithms.
#[derive(Debug, Clone, Copy)]
pub enum ChecksumVerifier {
    /// SHA-256 checksums
    Sha256,
    /// BLAKE3 checksums
    Blake3,
}

impl ChecksumVerifier {
    /// Compute a checksum for the given data.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::crypto::ChecksumVerifier;
    ///
    /// let data = b"Hello, world!";
    /// let checksum = ChecksumVerifier::Sha256.compute(data);
    /// println!("SHA-256: {}", checksum);
    ///
    /// let checksum = ChecksumVerifier::Blake3.compute(data);
    /// println!("BLAKE3: {}", checksum);
    /// ```
    pub fn compute(&self, data: &[u8]) -> String {
        match self {
            Self::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            Self::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                hex::encode(hasher.finalize().as_bytes())
            }
        }
    }

    /// Verify data against a checksum.
    ///
    /// # Examples
    ///
    /// ```
    /// use common::crypto::ChecksumVerifier;
    ///
    /// let data = b"Hello, world!";
    /// let checksum = ChecksumVerifier::Sha256.compute(data);
    ///
    /// assert!(ChecksumVerifier::Sha256.verify(data, &checksum).expect("Failed to verify"));
    /// assert!(!ChecksumVerifier::Sha256.verify(b"Wrong data", &checksum).expect("Failed to verify"));
    /// ```
    pub fn verify(&self, data: &[u8], expected_checksum: &str) -> Result<bool> {
        let actual_checksum = self.compute(data);

        // Use constant-time comparison to prevent timing attacks
        if actual_checksum.len() != expected_checksum.len() {
            return Ok(false);
        }

        let mut result = 0u8;
        for (a, b) in actual_checksum.bytes().zip(expected_checksum.bytes()) {
            result |= a ^ b;
        }

        Ok(result == 0)
    }

    /// Compute a checksum for a file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use common::crypto::ChecksumVerifier;
    ///
    /// let checksum = ChecksumVerifier::Sha256
    ///     .compute_file("/path/to/file")
    ///     .expect("Failed to compute checksum");
    /// ```
    pub fn compute_file(&self, path: &str) -> Result<String> {
        let data = std::fs::read(path).context("Failed to read file")?;
        Ok(self.compute(&data))
    }

    /// Verify a file against a checksum.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use common::crypto::ChecksumVerifier;
    ///
    /// let checksum = "abc123...";
    /// let is_valid = ChecksumVerifier::Sha256
    ///     .verify_file("/path/to/file", checksum)
    ///     .expect("Failed to verify file");
    /// ```
    pub fn verify_file(&self, path: &str, expected_checksum: &str) -> Result<bool> {
        let data = std::fs::read(path).context("Failed to read file")?;
        self.verify(&data, expected_checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        // Hash should not be empty
        assert!(!hash.is_empty());

        // Hash should start with Argon2 identifier
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_verify_password_success() {
        let password = "test_password_123";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(password, &hash).expect("Failed to verify password");
        assert!(result);
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "test_password_123";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).expect("Failed to hash password");

        let result = verify_password(wrong_password, &hash).expect("Failed to verify password");
        assert!(!result);
    }

    #[test]
    fn test_generate_token() {
        let token1 = generate_token(32);
        let token2 = generate_token(32);

        // Token should be 64 characters (32 bytes in hex)
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Tokens should be different
        assert_ne!(token1, token2);

        // Token should only contain hex characters
        assert!(token1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_sha256_checksum() {
        let data = b"Hello, world!";
        let checksum = ChecksumVerifier::Sha256.compute(data);

        // SHA-256 produces 64 hex characters (32 bytes)
        assert_eq!(checksum.len(), 64);

        // Verify the checksum
        let result = ChecksumVerifier::Sha256
            .verify(data, &checksum)
            .expect("Failed to verify");
        assert!(result);

        // Verify with wrong data
        let result = ChecksumVerifier::Sha256
            .verify(b"Wrong data", &checksum)
            .expect("Failed to verify");
        assert!(!result);
    }

    #[test]
    fn test_blake3_checksum() {
        let data = b"Hello, world!";
        let checksum = ChecksumVerifier::Blake3.compute(data);

        // BLAKE3 produces 64 hex characters (32 bytes)
        assert_eq!(checksum.len(), 64);

        // Verify the checksum
        let result = ChecksumVerifier::Blake3
            .verify(data, &checksum)
            .expect("Failed to verify");
        assert!(result);

        // Verify with wrong data
        let result = ChecksumVerifier::Blake3
            .verify(b"Wrong data", &checksum)
            .expect("Failed to verify");
        assert!(!result);
    }

    #[test]
    fn test_checksum_verifiers_differ() {
        let data = b"Hello, world!";
        let sha256 = ChecksumVerifier::Sha256.compute(data);
        let blake3 = ChecksumVerifier::Blake3.compute(data);

        // Different algorithms should produce different checksums
        assert_ne!(sha256, blake3);
    }
}