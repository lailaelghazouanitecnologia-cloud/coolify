//! SSH private key management

use kornetti_core::{Result, Error};
use russh_keys::PrivateKey as RusshPrivateKey;

/// Wrapper around SSH private key
pub struct PrivateKey {
    inner: RusshPrivateKey,
}

impl PrivateKey {
    /// Parse a private key from PEM format
    pub fn from_pem(pem: &str, passphrase: Option<&str>) -> Result<Self> {
        let inner = if let Some(pass) = passphrase {
            russh_keys::decode_secret_key(pem, Some(pass))
                .map_err(|e| Error::Ssh(format!("Failed to decode private key: {}", e)))?
        } else {
            russh_keys::decode_secret_key(pem, None)
                .map_err(|e| Error::Ssh(format!("Failed to decode private key: {}", e)))?
        };

        Ok(Self { inner })
    }

    /// Generate a new Ed25519 key pair
    pub fn generate_ed25519() -> Result<Self> {
        let inner = RusshPrivateKey::random(&mut rand::thread_rng(), russh_keys::Algorithm::Ed25519)
            .map_err(|e| Error::Ssh(format!("Failed to generate key: {}", e)))?;
        Ok(Self { inner })
    }

    /// Generate a new RSA key pair
    pub fn generate_rsa(bits: usize) -> Result<Self> {
        let inner = RusshPrivateKey::random(&mut rand::thread_rng(), russh_keys::Algorithm::Rsa { bits })
            .map_err(|e| Error::Ssh(format!("Failed to generate key: {}", e)))?;
        Ok(Self { inner })
    }

    /// Get the inner russh key
    pub fn inner(&self) -> &RusshPrivateKey {
        &self.inner
    }

    /// Get the public key in OpenSSH format
    pub fn public_key_openssh(&self) -> String {
        self.inner.public_key().to_string()
    }
}
