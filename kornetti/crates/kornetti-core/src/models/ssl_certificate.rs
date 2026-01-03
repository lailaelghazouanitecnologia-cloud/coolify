//! SSL Certificate model
//!
//! SSL/TLS certificates for domains.
//! Mirrors Coolify's SslCertificate model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// SSL certificate type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateType {
    /// Let's Encrypt automatic certificate
    LetsEncrypt,
    /// Custom uploaded certificate
    Custom,
    /// Self-signed certificate
    SelfSigned,
}

impl Default for CertificateType {
    fn default() -> Self {
        CertificateType::LetsEncrypt
    }
}

/// SSL certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslCertificate {
    pub id: Uuid,
    /// Domain(s) covered by this certificate
    pub domain: String,
    /// Additional domains (SANs)
    pub additional_domains: Vec<String>,
    /// Certificate type
    pub certificate_type: CertificateType,
    /// PEM-encoded certificate
    pub certificate: Option<String>,
    /// PEM-encoded private key (encrypted)
    pub private_key: Option<String>,
    /// PEM-encoded certificate chain
    pub certificate_chain: Option<String>,
    /// Certificate expiry date
    pub expires_at: Option<DateTime<Utc>>,
    /// When the certificate was last renewed
    pub renewed_at: Option<DateTime<Utc>>,
    /// Resource type this certificate belongs to
    pub resource_type: CertificateResourceType,
    /// Resource ID
    pub resource_id: Uuid,
    /// Whether certificate is active
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Types of resources that can have SSL certificates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateResourceType {
    Application,
    Service,
    Database,
}

impl SslCertificate {
    pub fn new(domain: String, resource_type: CertificateResourceType, resource_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            domain,
            additional_domains: Vec::new(),
            certificate_type: CertificateType::LetsEncrypt,
            certificate: None,
            private_key: None,
            certificate_chain: None,
            expires_at: None,
            renewed_at: None,
            resource_type,
            resource_id,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a custom certificate
    pub fn custom(
        domain: String,
        certificate: String,
        private_key: String,
        certificate_chain: Option<String>,
        resource_type: CertificateResourceType,
        resource_id: Uuid,
    ) -> Self {
        let mut cert = Self::new(domain, resource_type, resource_id);
        cert.certificate_type = CertificateType::Custom;
        cert.certificate = Some(certificate);
        cert.private_key = Some(private_key);
        cert.certificate_chain = certificate_chain;
        cert
    }

    /// Check if certificate is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    /// Check if certificate needs renewal (within 30 days of expiry)
    pub fn needs_renewal(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now() + chrono::Duration::days(30))
            .unwrap_or(true)
    }

    /// Get all domains covered by this certificate
    pub fn all_domains(&self) -> Vec<&str> {
        let mut domains = vec![self.domain.as_str()];
        domains.extend(self.additional_domains.iter().map(|s| s.as_str()));
        domains
    }

    /// Update certificate with new values
    pub fn renew(&mut self, certificate: String, private_key: String, expires_at: DateTime<Utc>) {
        self.certificate = Some(certificate);
        self.private_key = Some(private_key);
        self.expires_at = Some(expires_at);
        self.renewed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}
