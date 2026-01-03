//! Configuration management for Kornetti

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub providers: ProvidersConfig,
    pub security: SecurityConfig,
    /// JWT secret for API authentication
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    /// Application key for encryption (like Laravel APP_KEY)
    #[serde(default = "default_app_key")]
    pub app_key: String,
}

fn default_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_string())
}

fn default_app_key() -> String {
    std::env::var("APP_KEY").unwrap_or_else(|_| "base64:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string())
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable API rate limiting
    #[serde(default)]
    pub rate_limiting_enabled: bool,
    /// Requests per minute for API rate limiting
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    /// CORS allowed origins
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Require HTTPS in production
    #[serde(default = "default_true")]
    pub require_https: bool,
    /// Token expiration in hours
    #[serde(default = "default_token_expiration")]
    pub token_expiration_hours: u32,
}

fn default_rate_limit() -> u32 { 60 }
fn default_true() -> bool { true }
fn default_token_expiration() -> u32 { 24 }

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limiting_enabled: true,
            rate_limit_per_minute: 60,
            cors_origins: Vec::new(),
            require_https: true,
            token_expiration_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub vultr: Option<VultrConfig>,
    pub hetzner: Option<HetznerConfig>,
    pub digitalocean: Option<DigitalOceanConfig>,
    pub aws: Option<AwsConfig>,
    pub linode: Option<LinodeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VultrConfig {
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HetznerConfig {
    pub api_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOceanConfig {
    pub api_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinodeConfig {
    pub api_token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: None,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/kornetti".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            redis: RedisConfig {
                url: "redis://localhost:6379".to_string(),
                pool_size: 10,
            },
            providers: ProvidersConfig {
                vultr: None,
                hetzner: None,
                digitalocean: None,
                aws: None,
                linode: None,
            },
            security: SecurityConfig::default(),
            jwt_secret: default_jwt_secret(),
            app_key: default_app_key(),
        }
    }
}
