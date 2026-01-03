//! Kornetti Proxy
//!
//! Proxy configuration generation for Traefik and Caddy.

pub mod traefik;
pub mod caddy;

use kornetti_core::models::ProxyType;

/// Generate proxy configuration based on type
pub fn generate_config(proxy_type: ProxyType, services: &[ServiceConfig]) -> String {
    match proxy_type {
        ProxyType::Traefik => traefik::generate(services),
        ProxyType::Caddy => caddy::generate(services),
        ProxyType::None => String::new(),
    }
}

/// Service configuration for proxy
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: String,
    pub container_name: String,
    pub domain: String,
    pub port: u16,
    pub https: bool,
    pub www_redirect: bool,
    pub headers: Vec<(String, String)>,
}
