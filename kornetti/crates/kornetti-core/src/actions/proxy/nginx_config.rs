//! Nginx configuration generator
//!
//! Generates Nginx reverse proxy configurations for applications.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Nginx upstream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxUpstream {
    pub name: String,
    pub servers: Vec<UpstreamServer>,
    pub load_balancing: LoadBalancing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamServer {
    pub address: String,
    pub port: u16,
    pub weight: Option<u32>,
    pub max_fails: Option<u32>,
    pub fail_timeout: Option<String>,
    pub backup: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LoadBalancing {
    RoundRobin,
    LeastConn,
    IpHash,
    Random,
}

impl Default for LoadBalancing {
    fn default() -> Self {
        LoadBalancing::RoundRobin
    }
}

/// Nginx server block configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxServerBlock {
    /// Server names (domains)
    pub server_names: Vec<String>,
    /// Listen port (HTTP)
    pub listen_http: u16,
    /// Listen port (HTTPS)
    pub listen_https: Option<u16>,
    /// SSL certificate path
    pub ssl_certificate: Option<String>,
    /// SSL certificate key path
    pub ssl_certificate_key: Option<String>,
    /// Upstream name to proxy to
    pub upstream_name: String,
    /// Upstream port
    pub upstream_port: u16,
    /// Enable WebSocket support
    pub websocket: bool,
    /// Custom headers
    pub proxy_headers: Vec<(String, String)>,
    /// HTTPS redirect
    pub force_https: bool,
    /// Enable HTTP/2
    pub http2: bool,
    /// Proxy timeout
    pub proxy_timeout: u32,
    /// Max body size
    pub client_max_body_size: String,
    /// Custom locations
    pub custom_locations: Vec<NginxLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NginxLocation {
    pub path: String,
    pub config: String,
}

impl Default for NginxServerBlock {
    fn default() -> Self {
        Self {
            server_names: Vec::new(),
            listen_http: 80,
            listen_https: Some(443),
            ssl_certificate: None,
            ssl_certificate_key: None,
            upstream_name: String::new(),
            upstream_port: 3000,
            websocket: false,
            proxy_headers: vec![
                ("Host".to_string(), "$host".to_string()),
                ("X-Real-IP".to_string(), "$remote_addr".to_string()),
                ("X-Forwarded-For".to_string(), "$proxy_add_x_forwarded_for".to_string()),
                ("X-Forwarded-Proto".to_string(), "$scheme".to_string()),
            ],
            force_https: true,
            http2: true,
            proxy_timeout: 60,
            client_max_body_size: "100M".to_string(),
            custom_locations: Vec::new(),
        }
    }
}

/// Generate Nginx configuration for an application
pub fn generate_nginx_config(
    app_id: &Uuid,
    config: &NginxServerBlock,
) -> String {
    let mut nginx = String::new();
    let server_name = config.server_names.join(" ");

    // HTTP Server Block
    nginx.push_str(&format!(r#"# Application: {}
server {{
    listen {};
    server_name {};

"#, app_id, config.listen_http, server_name));

    // HTTPS redirect
    if config.force_https && config.ssl_certificate.is_some() {
        nginx.push_str(r#"    # Redirect HTTP to HTTPS
    return 301 https://$host$request_uri;
}

"#);
        // HTTPS Server Block
        nginx.push_str(&format!(r#"server {{
    listen {}{};
    server_name {};

    ssl_certificate {};
    ssl_certificate_key {};
    ssl_session_timeout 1d;
    ssl_session_cache shared:SSL:50m;
    ssl_session_tickets off;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;

"#,
            config.listen_https.unwrap_or(443),
            if config.http2 { " ssl http2" } else { " ssl" },
            server_name,
            config.ssl_certificate.as_ref().unwrap(),
            config.ssl_certificate_key.as_ref().unwrap()
        ));
    }

    // Client max body size
    nginx.push_str(&format!("    client_max_body_size {};\n\n", config.client_max_body_size));

    // Main location
    nginx.push_str("    location / {\n");
    nginx.push_str(&format!("        proxy_pass http://{}:{};\n", config.upstream_name, config.upstream_port));
    nginx.push_str(&format!("        proxy_connect_timeout {}s;\n", config.proxy_timeout));
    nginx.push_str(&format!("        proxy_send_timeout {}s;\n", config.proxy_timeout));
    nginx.push_str(&format!("        proxy_read_timeout {}s;\n", config.proxy_timeout));

    // Proxy headers
    for (header, value) in &config.proxy_headers {
        nginx.push_str(&format!("        proxy_set_header {} {};\n", header, value));
    }

    // WebSocket support
    if config.websocket {
        nginx.push_str(r#"
        # WebSocket support
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
"#);
    }

    nginx.push_str("    }\n");

    // Custom locations
    for location in &config.custom_locations {
        nginx.push_str(&format!("\n    location {} {{\n", location.path));
        for line in location.config.lines() {
            nginx.push_str(&format!("        {}\n", line.trim()));
        }
        nginx.push_str("    }\n");
    }

    nginx.push_str("}\n");
    nginx
}

/// Generate Nginx upstream block
pub fn generate_nginx_upstream(upstream: &NginxUpstream) -> String {
    let mut config = format!("upstream {} {{\n", upstream.name);

    // Load balancing method
    match upstream.load_balancing {
        LoadBalancing::RoundRobin => {} // default
        LoadBalancing::LeastConn => config.push_str("    least_conn;\n"),
        LoadBalancing::IpHash => config.push_str("    ip_hash;\n"),
        LoadBalancing::Random => config.push_str("    random;\n"),
    }

    // Servers
    for server in &upstream.servers {
        let mut line = format!("    server {}:{}", server.address, server.port);

        if let Some(weight) = server.weight {
            line.push_str(&format!(" weight={}", weight));
        }
        if let Some(max_fails) = server.max_fails {
            line.push_str(&format!(" max_fails={}", max_fails));
        }
        if let Some(ref timeout) = server.fail_timeout {
            line.push_str(&format!(" fail_timeout={}", timeout));
        }
        if server.backup {
            line.push_str(" backup");
        }

        line.push_str(";\n");
        config.push_str(&line);
    }

    config.push_str("}\n");
    config
}

/// Generate main Nginx configuration
pub fn generate_nginx_main_config() -> String {
    r#"user nginx;
worker_processes auto;
error_log /var/log/nginx/error.log warn;
pid /var/run/nginx.pid;

events {
    worker_connections 4096;
    multi_accept on;
    use epoll;
}

http {
    include /etc/nginx/mime.types;
    default_type application/octet-stream;

    log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                    '$status $body_bytes_sent "$http_referer" '
                    '"$http_user_agent" "$http_x_forwarded_for"';

    access_log /var/log/nginx/access.log main;

    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    types_hash_max_size 2048;

    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_proxied any;
    gzip_comp_level 6;
    gzip_types text/plain text/css text/xml application/json application/javascript application/rss+xml application/atom+xml image/svg+xml;

    # Include site configurations
    include /etc/nginx/conf.d/*.conf;
    include /etc/nginx/sites-enabled/*;
}
"#.to_string()
}

/// Generate Nginx Docker Compose file
pub fn generate_nginx_compose(http_port: u16, https_port: u16) -> String {
    format!(r#"services:
  coolify-proxy:
    image: nginx:alpine
    container_name: coolify-proxy
    restart: unless-stopped
    ports:
      - "{}:80"
      - "{}:443"
    volumes:
      - /data/coolify/proxy/nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - /data/coolify/proxy/nginx/conf.d:/etc/nginx/conf.d:ro
      - /data/coolify/proxy/nginx/sites-enabled:/etc/nginx/sites-enabled:ro
      - /data/coolify/proxy/letsencrypt:/etc/letsencrypt:ro
      - /var/log/nginx:/var/log/nginx
    networks:
      - coolify
    labels:
      - coolify.managed=true
      - coolify.proxy=true
    healthcheck:
      test: ["CMD", "nginx", "-t"]
      interval: 30s
      timeout: 10s
      retries: 3

networks:
  coolify:
    external: true
"#, http_port, https_port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_nginx_config() {
        let config = NginxServerBlock {
            server_names: vec!["example.com".to_string(), "www.example.com".to_string()],
            upstream_name: "my-app".to_string(),
            upstream_port: 3000,
            ssl_certificate: Some("/etc/letsencrypt/live/example.com/fullchain.pem".to_string()),
            ssl_certificate_key: Some("/etc/letsencrypt/live/example.com/privkey.pem".to_string()),
            websocket: true,
            ..Default::default()
        };

        let nginx = generate_nginx_config(&Uuid::new_v4(), &config);
        assert!(nginx.contains("server_name example.com www.example.com"));
        assert!(nginx.contains("proxy_pass http://my-app:3000"));
        assert!(nginx.contains("WebSocket support"));
        assert!(nginx.contains("ssl_certificate"));
    }

    #[test]
    fn test_generate_upstream() {
        let upstream = NginxUpstream {
            name: "my-app".to_string(),
            servers: vec![
                UpstreamServer {
                    address: "app1".to_string(),
                    port: 3000,
                    weight: Some(3),
                    max_fails: Some(3),
                    fail_timeout: Some("30s".to_string()),
                    backup: false,
                },
                UpstreamServer {
                    address: "app2".to_string(),
                    port: 3000,
                    weight: Some(2),
                    max_fails: None,
                    fail_timeout: None,
                    backup: true,
                },
            ],
            load_balancing: LoadBalancing::LeastConn,
        };

        let config = generate_nginx_upstream(&upstream);
        assert!(config.contains("least_conn"));
        assert!(config.contains("server app1:3000 weight=3"));
        assert!(config.contains("backup"));
    }
}
