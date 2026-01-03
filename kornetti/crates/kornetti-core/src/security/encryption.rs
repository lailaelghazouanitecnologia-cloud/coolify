//! Encryption utilities for sensitive data
//!
//! This module provides encryption/decryption for sensitive data like:
//! - Private SSH keys
//! - Database passwords
//! - API tokens
//! - OAuth secrets
//!
//! Uses AES-256-GCM for authenticated encryption, similar to Laravel's encryption.

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::SecurityError;

/// Length of the encryption key (256 bits = 32 bytes)
const KEY_LENGTH: usize = 32;

/// Length of the nonce (96 bits = 12 bytes for AES-GCM)
const NONCE_LENGTH: usize = 12;

/// Encrypted payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Initialization vector / nonce (base64 encoded)
    pub iv: String,
    /// Encrypted value (base64 encoded)
    pub value: String,
    /// HMAC for integrity verification (base64 encoded)
    pub mac: String,
}

/// Encryption key wrapper with automatic zeroing
pub struct EncryptionKey {
    key: Zeroizing<[u8; KEY_LENGTH]>,
}

impl EncryptionKey {
    /// Create a new encryption key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SecurityError> {
        if bytes.len() != KEY_LENGTH {
            return Err(SecurityError::InvalidKeyFormat(
                format!("Key must be {} bytes, got {}", KEY_LENGTH, bytes.len())
            ));
        }
        let mut key = [0u8; KEY_LENGTH];
        key.copy_from_slice(bytes);
        Ok(Self {
            key: Zeroizing::new(key),
        })
    }

    /// Create a key from a base64-encoded string
    pub fn from_base64(encoded: &str) -> Result<Self, SecurityError> {
        let bytes = BASE64.decode(encoded)
            .map_err(|e| SecurityError::InvalidKeyFormat(e.to_string()))?;
        Self::from_bytes(&bytes)
    }

    /// Derive a key from a password/passphrase using PBKDF2-like derivation
    pub fn derive_from_password(password: &str, salt: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);

        // Multiple rounds for key stretching
        let mut result = hasher.finalize();
        for _ in 0..10000 {
            let mut hasher = Sha256::new();
            hasher.update(&result);
            hasher.update(salt);
            result = hasher.finalize();
        }

        let mut key = [0u8; KEY_LENGTH];
        key.copy_from_slice(&result);
        Self {
            key: Zeroizing::new(key),
        }
    }

    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_LENGTH];
        OsRng.fill_bytes(&mut key);
        Self {
            key: Zeroizing::new(key),
        }
    }

    /// Export the key as base64
    pub fn to_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(BASE64.encode(self.key.as_slice()))
    }
}

/// Encryptor for encrypting and decrypting sensitive data
pub struct Encryptor {
    key: EncryptionKey,
}

impl Encryptor {
    /// Create a new encryptor with the given key
    pub fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    /// Create from environment variable APP_KEY (like Laravel)
    pub fn from_app_key(app_key: &str) -> Result<Self, SecurityError> {
        // Laravel APP_KEY format: "base64:xxxx..."
        let key_bytes = if let Some(encoded) = app_key.strip_prefix("base64:") {
            BASE64.decode(encoded)
                .map_err(|e| SecurityError::InvalidKeyFormat(e.to_string()))?
        } else {
            // Try to decode directly
            BASE64.decode(app_key)
                .map_err(|e| SecurityError::InvalidKeyFormat(e.to_string()))?
        };

        let key = EncryptionKey::from_bytes(&key_bytes)?;
        Ok(Self::new(key))
    }

    /// Encrypt a string value
    ///
    /// Returns a base64-encoded JSON payload containing the IV, ciphertext, and MAC
    pub fn encrypt(&self, plaintext: &str) -> Result<String, SecurityError> {
        self.encrypt_bytes(plaintext.as_bytes())
    }

    /// Encrypt raw bytes
    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Result<String, SecurityError> {
        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LENGTH];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(self.key.key.as_slice())
            .map_err(|e| SecurityError::Encryption(e.to_string()))?;

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| SecurityError::Encryption(e.to_string()))?;

        // Calculate MAC for additional integrity check
        let mac = self.calculate_mac(&nonce_bytes, &ciphertext);

        // Create payload
        let payload = EncryptedPayload {
            iv: BASE64.encode(nonce_bytes),
            value: BASE64.encode(&ciphertext),
            mac: BASE64.encode(mac),
        };

        // Serialize and encode
        let json = serde_json::to_string(&payload)
            .map_err(|e| SecurityError::Encryption(e.to_string()))?;
        Ok(BASE64.encode(json))
    }

    /// Decrypt a string value
    pub fn decrypt(&self, encrypted: &str) -> Result<String, SecurityError> {
        let bytes = self.decrypt_bytes(encrypted)?;
        String::from_utf8(bytes)
            .map_err(|e| SecurityError::Decryption(e.to_string()))
    }

    /// Decrypt to raw bytes
    pub fn decrypt_bytes(&self, encrypted: &str) -> Result<Vec<u8>, SecurityError> {
        // Decode base64 outer layer
        let json = BASE64.decode(encrypted)
            .map_err(|e| SecurityError::Decryption(format!("Invalid base64: {}", e)))?;

        // Parse JSON payload
        let payload: EncryptedPayload = serde_json::from_slice(&json)
            .map_err(|e| SecurityError::Decryption(format!("Invalid payload: {}", e)))?;

        // Decode components
        let nonce_bytes = BASE64.decode(&payload.iv)
            .map_err(|e| SecurityError::Decryption(format!("Invalid IV: {}", e)))?;
        let ciphertext = BASE64.decode(&payload.value)
            .map_err(|e| SecurityError::Decryption(format!("Invalid ciphertext: {}", e)))?;
        let mac = BASE64.decode(&payload.mac)
            .map_err(|e| SecurityError::Decryption(format!("Invalid MAC: {}", e)))?;

        // Verify MAC
        let expected_mac = self.calculate_mac(&nonce_bytes, &ciphertext);
        if !constant_time_compare(&mac, &expected_mac) {
            return Err(SecurityError::Decryption("MAC verification failed".to_string()));
        }

        // Verify nonce length
        if nonce_bytes.len() != NONCE_LENGTH {
            return Err(SecurityError::Decryption("Invalid nonce length".to_string()));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        // Create cipher and decrypt
        let cipher = Aes256Gcm::new_from_slice(self.key.key.as_slice())
            .map_err(|e| SecurityError::Decryption(e.to_string()))?;

        cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| SecurityError::Decryption(e.to_string()))
    }

    /// Calculate HMAC for integrity verification
    fn calculate_mac(&self, iv: &[u8], ciphertext: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(iv);
        hasher.update(ciphertext);
        hasher.update(self.key.key.as_slice());
        hasher.finalize().to_vec()
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Encrypt a value using the global app key
///
/// This is a convenience function that mirrors Coolify's Crypt::encryptString()
pub fn encrypt_string(value: &str, app_key: &str) -> Result<String, SecurityError> {
    let encryptor = Encryptor::from_app_key(app_key)?;
    encryptor.encrypt(value)
}

/// Decrypt a value using the global app key
///
/// This is a convenience function that mirrors Coolify's Crypt::decryptString()
pub fn decrypt_string(encrypted: &str, app_key: &str) -> Result<String, SecurityError> {
    let encryptor = Encryptor::from_app_key(app_key)?;
    encryptor.decrypt(encrypted)
}

/// Hash a password using Argon2id (matching Laravel's bcrypt but more secure)
pub fn hash_password(password: &str) -> Result<String, SecurityError> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| SecurityError::Encryption(e.to_string()))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, SecurityError> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| SecurityError::Decryption(e.to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        let plaintext = "my secret password";
        let encrypted = encryptor.encrypt(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
        assert_ne!(plaintext, encrypted);
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        let plaintext = "same message";
        let encrypted1 = encryptor.encrypt(plaintext).unwrap();
        let encrypted2 = encryptor.encrypt(plaintext).unwrap();

        // Should produce different ciphertext due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same plaintext
        assert_eq!(encryptor.decrypt(&encrypted1).unwrap(), plaintext);
        assert_eq!(encryptor.decrypt(&encrypted2).unwrap(), plaintext);
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        let plaintext = "my secret";
        let mut encrypted = encryptor.encrypt(plaintext).unwrap();

        // Tamper with the encrypted value
        if encrypted.len() > 10 {
            let bytes: Vec<u8> = encrypted.bytes().collect();
            let mut tampered = bytes;
            tampered[10] ^= 0xFF;
            encrypted = String::from_utf8_lossy(&tampered).to_string();
        }

        // Decryption should fail
        assert!(encryptor.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_password_hashing() {
        let password = "my_secure_password123!";
        let hash = hash_password(password).unwrap();

        // Verify correct password
        assert!(verify_password(password, &hash).unwrap());

        // Verify incorrect password
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_derive_key_from_password() {
        let password = "my passphrase";
        let salt = b"random_salt_value";

        let key1 = EncryptionKey::derive_from_password(password, salt);
        let key2 = EncryptionKey::derive_from_password(password, salt);

        // Same password and salt should produce same key
        assert_eq!(key1.to_base64().as_str(), key2.to_base64().as_str());

        // Different salt should produce different key
        let key3 = EncryptionKey::derive_from_password(password, b"different_salt");
        assert_ne!(key1.to_base64().as_str(), key3.to_base64().as_str());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare(b"hello", b"hello"));
        assert!(!constant_time_compare(b"hello", b"world"));
        assert!(!constant_time_compare(b"hello", b"hell"));
    }
}
