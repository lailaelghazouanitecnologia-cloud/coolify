//! Proxy Actions
//!
//! Actions for managing reverse proxy servers (Traefik, Caddy, Nginx).

mod start_proxy;
mod stop_proxy;
mod save_configuration;
mod get_configuration;
mod check_proxy;
pub mod nginx_config;

pub use start_proxy::StartProxy;
pub use stop_proxy::StopProxy;
pub use save_configuration::SaveProxyConfiguration;
pub use get_configuration::GetProxyConfiguration;
pub use check_proxy::CheckProxy;
pub use nginx_config::{
    NginxServerBlock, NginxUpstream, NginxLocation, UpstreamServer, LoadBalancing,
    generate_nginx_config, generate_nginx_upstream, generate_nginx_main_config, generate_nginx_compose,
};

use serde::{Deserialize, Serialize};

/// Supported proxy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    Traefik,
    Caddy,
    None,
}

impl Default for ProxyType {
    fn default() -> Self {
        ProxyType::Traefik
    }
}

impl ProxyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProxyType::Traefik => "traefik",
            ProxyType::Caddy => "caddy",
            ProxyType::None => "none",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "traefik" => ProxyType::Traefik,
            "caddy" => ProxyType::Caddy,
            _ => ProxyType::None,
        }
    }

    pub fn default_image(&self) -> &'static str {
        match self {
            ProxyType::Traefik => "traefik:v3.0",
            ProxyType::Caddy => "caddy:2-alpine",
            ProxyType::None => "",
        }
    }
}

/// Proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    /// HTTP port (default: 80)
    pub http_port: u16,
    /// HTTPS port (default: 443)
    pub https_port: u16,
    /// Dashboard enabled
    pub dashboard_enabled: bool,
    /// Dashboard port (Traefik: 8080)
    pub dashboard_port: u16,
    /// ACME email for Let's Encrypt
    pub acme_email: Option<String>,
    /// Use staging ACME server
    pub acme_staging: bool,
    /// Additional networks to connect to
    pub networks: Vec<String>,
    /// Custom labels
    pub custom_labels: Vec<(String, String)>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            proxy_type: ProxyType::Traefik,
            http_port: 80,
            https_port: 443,
            dashboard_enabled: false,
            dashboard_port: 8080,
            acme_email: None,
            acme_staging: false,
            networks: vec!["coolify".to_string()],
            custom_labels: Vec::new(),
        }
    }
}

/// Get proxy configuration directory
pub fn proxy_path() -> &'static str {
    "/data/coolify/proxy"
}

/// Get Traefik configuration directory
pub fn traefik_path() -> &'static str {
    "/data/coolify/proxy"
}

/// Get Caddy configuration directory
pub fn caddy_path() -> &'static str {
    "/data/coolify/proxy/caddy"
}

/// Generate Traefik static configuration
pub fn generate_traefik_static_config(config: &ProxyConfig) -> String {
    let mut yaml = String::new();

    yaml.push_str("api:\n");
    if config.dashboard_enabled {
        yaml.push_str("  dashboard: true\n");
        yaml.push_str("  insecure: true\n");
    }

    yaml.push_str("\nentryPoints:\n");
    yaml.push_str(&format!("  web:\n    address: \":{}\"\n", config.http_port));
    yaml.push_str(&format!("  websecure:\n    address: \":{}\"\n", config.https_port));

    yaml.push_str("\nproviders:\n");
    yaml.push_str("  docker:\n");
    yaml.push_str("    exposedByDefault: false\n");
    yaml.push_str("    network: coolify\n");
    yaml.push_str("  file:\n");
    yaml.push_str("    directory: /dynamic\n");
    yaml.push_str("    watch: true\n");

    if let Some(ref email) = config.acme_email {
        yaml.push_str("\ncertificatesResolvers:\n");
        yaml.push_str("  letsencrypt:\n");
        yaml.push_str("    acme:\n");
        yaml.push_str(&format!("      email: {}\n", email));
        yaml.push_str("      storage: /letsencrypt/acme.json\n");
        if config.acme_staging {
            yaml.push_str("      caServer: https://acme-staging-v02.api.letsencrypt.org/directory\n");
        }
        yaml.push_str("      httpChallenge:\n");
        yaml.push_str("        entryPoint: web\n");
    }

    yaml
}

/// Generate Traefik docker-compose.yml
pub fn generate_traefik_compose(config: &ProxyConfig) -> String {
    let networks: Vec<String> = config.networks.iter()
        .map(|n| format!("      - {}", n))
        .collect();

    format!(r#"services:
  coolify-proxy:
    image: {}
    container_name: coolify-proxy
    restart: unless-stopped
    ports:
      - "{}:80"
      - "{}:443"
      - "{}:8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
      - {}:/traefik.yml:ro
      - {}/dynamic:/dynamic
      - {}/letsencrypt:/letsencrypt
    command:
      - --configFile=/traefik.yml
    networks:
{}
    labels:
      - coolify.managed=true
      - coolify.proxy=true

networks:
{}
"#,
        config.proxy_type.default_image(),
        config.http_port,
        config.https_port,
        config.dashboard_port,
        format!("{}/traefik.yml", traefik_path()),
        traefik_path(),
        traefik_path(),
        networks.join("\n"),
        config.networks.iter()
            .map(|n| format!("  {}:\n    external: true", n))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

/// Generate Caddy docker-compose.yml
pub fn generate_caddy_compose(config: &ProxyConfig) -> String {
    format!(r#"services:
  coolify-proxy:
    image: {}
    container_name: coolify-proxy
    restart: unless-stopped
    ports:
      - "{}:80"
      - "{}:443"
    volumes:
      - {}/Caddyfile:/etc/caddy/Caddyfile:ro
      - {}/dynamic:/dynamic
      - {}/data:/data
      - {}/config:/config
    networks:
      - coolify
    labels:
      - coolify.managed=true
      - coolify.proxy=true

networks:
  coolify:
    external: true
"#,
        config.proxy_type.default_image(),
        config.http_port,
        config.https_port,
        caddy_path(),
        caddy_path(),
        caddy_path(),
        caddy_path()
    )
}

/// Generate Caddyfile for Caddy proxy
pub fn generate_caddyfile(config: &ProxyConfig) -> String {
    let mut caddyfile = String::new();

    // Global options
    if let Some(ref email) = config.acme_email {
        caddyfile.push_str("{\n");
        caddyfile.push_str(&format!("    email {}\n", email));
        if config.acme_staging {
            caddyfile.push_str("    acme_ca https://acme-staging-v02.api.letsencrypt.org/directory\n");
        }
        caddyfile.push_str("}\n\n");
    }

    // Import dynamic configs
    caddyfile.push_str("import /dynamic/*.caddy\n");

    caddyfile
}
