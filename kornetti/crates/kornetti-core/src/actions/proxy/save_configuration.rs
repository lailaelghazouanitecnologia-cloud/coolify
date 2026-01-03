//! Save Proxy Configuration Action
//!
//! Saves dynamic proxy configuration for an application or service.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument};

use crate::models::{Server, Application};
use super::{ProxyType, traefik_path, caddy_path};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use crate::ssh_stub::SshClient;

/// Route configuration for proxy
#[derive(Debug, Clone)]
pub struct RouteConfig {
    /// Application/service UUID
    pub uuid: String,
    /// Domain names
    pub domains: Vec<String>,
    /// Container name to route to
    pub container: String,
    /// Container port
    pub port: u16,
    /// Enable HTTPS
    pub https: bool,
    /// Force HTTPS redirect
    pub force_https: bool,
    /// Enable www redirect
    pub www_redirect: bool,
    /// Custom headers
    pub headers: Vec<(String, String)>,
    /// Path prefix (optional)
    pub path_prefix: Option<String>,
    /// Strip path prefix
    pub strip_prefix: bool,
}

pub struct SaveProxyConfigurationInput<'a> {
    pub server: &'a Server,
    pub proxy_type: ProxyType,
    pub route: RouteConfig,
}

pub struct SaveProxyConfiguration {
    ssh: Arc<dyn SshClient>,
}

impl SaveProxyConfiguration {
    pub fn new(ssh: Arc<dyn SshClient>) -> Self {
        Self { ssh }
    }

    /// Generate Traefik dynamic configuration
    fn generate_traefik_config(route: &RouteConfig) -> String {
        let mut yaml = String::new();

        yaml.push_str("http:\n");

        // Routers
        yaml.push_str("  routers:\n");

        for (i, domain) in route.domains.iter().enumerate() {
            let router_name = format!("{}-{}", route.uuid, i);

            // HTTP router
            yaml.push_str(&format!("    {}:\n", router_name));
            yaml.push_str(&format!("      rule: \"Host(`{}`)\"\n", domain));
            yaml.push_str("      entryPoints:\n        - web\n");

            if route.force_https {
                yaml.push_str("      middlewares:\n        - redirect-to-https\n");
            } else {
                yaml.push_str(&format!("      service: {}\n", route.uuid));
            }

            // HTTPS router
            if route.https {
                yaml.push_str(&format!("    {}-secure:\n", router_name));
                yaml.push_str(&format!("      rule: \"Host(`{}`)\"\n", domain));
                yaml.push_str("      entryPoints:\n        - websecure\n");
                yaml.push_str(&format!("      service: {}\n", route.uuid));
                yaml.push_str("      tls:\n        certResolver: letsencrypt\n");
            }
        }

        // Middlewares
        yaml.push_str("  middlewares:\n");

        if route.force_https {
            yaml.push_str("    redirect-to-https:\n");
            yaml.push_str("      redirectScheme:\n");
            yaml.push_str("        scheme: https\n");
            yaml.push_str("        permanent: true\n");
        }

        if route.strip_prefix {
            if let Some(ref prefix) = route.path_prefix {
                yaml.push_str(&format!("    {}-strip:\n", route.uuid));
                yaml.push_str("      stripPrefix:\n");
                yaml.push_str(&format!("        prefixes:\n          - \"{}\"\n", prefix));
            }
        }

        if !route.headers.is_empty() {
            yaml.push_str(&format!("    {}-headers:\n", route.uuid));
            yaml.push_str("      headers:\n");
            yaml.push_str("        customResponseHeaders:\n");
            for (key, value) in &route.headers {
                yaml.push_str(&format!("          {}: \"{}\"\n", key, value));
            }
        }

        // Services
        yaml.push_str("  services:\n");
        yaml.push_str(&format!("    {}:\n", route.uuid));
        yaml.push_str("      loadBalancer:\n");
        yaml.push_str("        servers:\n");
        yaml.push_str(&format!("          - url: \"http://{}:{}\"\n", route.container, route.port));

        yaml
    }

    /// Generate Caddy configuration
    fn generate_caddy_config(route: &RouteConfig) -> String {
        let mut caddy = String::new();

        for domain in &route.domains {
            let scheme = if route.https { "https" } else { "http" };

            caddy.push_str(&format!("{} {{\n", domain));

            // Reverse proxy
            if let Some(ref prefix) = route.path_prefix {
                caddy.push_str(&format!("    handle_path {}* {{\n", prefix));
                caddy.push_str(&format!("        reverse_proxy {}:{}\n", route.container, route.port));
                caddy.push_str("    }\n");
            } else {
                caddy.push_str(&format!("    reverse_proxy {}:{}\n", route.container, route.port));
            }

            // Custom headers
            for (key, value) in &route.headers {
                caddy.push_str(&format!("    header {} \"{}\"\n", key, value));
            }

            caddy.push_str("}\n\n");
        }

        caddy
    }
}

#[async_trait]
impl Action for SaveProxyConfiguration {
    type Input = SaveProxyConfigurationInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "save_proxy_configuration"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id, uuid = %input.route.uuid))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let proxy_type = input.proxy_type;
        let route = &input.route;
        let start = std::time::Instant::now();

        if proxy_type == ProxyType::None {
            return Ok(ActionResult::success("Proxy disabled, skipping configuration")
                .with_duration(start.elapsed()));
        }

        info!("Saving proxy configuration for {} on server {}", route.uuid, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        let (config_content, config_path) = match proxy_type {
            ProxyType::Traefik => {
                let content = Self::generate_traefik_config(route);
                let path = format!("{}/dynamic/{}.yml", traefik_path(), route.uuid);
                (content, path)
            }
            ProxyType::Caddy => {
                let content = Self::generate_caddy_config(route);
                let path = format!("{}/dynamic/{}.caddy", caddy_path(), route.uuid);
                (content, path)
            }
            ProxyType::None => unreachable!(),
        };

        let config_base64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &config_content
        );

        builder.add(format!(
            "echo '{}' | base64 -d > {}",
            config_base64, config_path
        ));

        // Reload Caddy if using Caddy (Traefik watches files automatically)
        if proxy_type == ProxyType::Caddy {
            builder.add("docker exec coolify-proxy caddy reload --config /etc/caddy/Caddyfile 2>/dev/null || true");
        }

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0 && !cmd.contains("|| true") {
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        Ok(ActionResult::success("Proxy configuration saved")
            .with_duration(start.elapsed())
            .with_commands(commands)
            .with_output(serde_json::json!({
                "config_path": config_path,
                "domains": route.domains,
            })))
    }
}
