//! Start Proxy Action
//!
//! Starts the reverse proxy (Traefik or Caddy) on a server.

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, instrument, warn};

use crate::models::Server;
use super::{
    ProxyConfig, ProxyType, proxy_path, traefik_path, caddy_path,
    generate_traefik_static_config, generate_traefik_compose,
    generate_caddy_compose, generate_caddyfile,
};
use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};
use kornetti_ssh::SshClient;

pub struct StartProxyInput<'a> {
    pub server: &'a Server,
    pub config: ProxyConfig,
    /// Force restart even if already running
    pub force: bool,
}

pub struct StartProxy {
    ssh: Arc<SshClient>,
}

impl StartProxy {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl Action for StartProxy {
    type Input = StartProxyInput<'static>;
    type Output = ActionResult;

    fn name(&self) -> &'static str {
        "start_proxy"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;
        let config = &input.config;
        let start = std::time::Instant::now();

        if config.proxy_type == ProxyType::None {
            return Ok(ActionResult::success("Proxy disabled, skipping start")
                .with_duration(start.elapsed()));
        }

        info!("Starting {:?} proxy on server {}", config.proxy_type, server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut builder = CommandBuilder::new();

        // Create directories
        builder.echo(&format!("Starting {} proxy...", config.proxy_type.as_str()));
        builder.mkdir(proxy_path());

        match config.proxy_type {
            ProxyType::Traefik => {
                builder.mkdir(&format!("{}/dynamic", traefik_path()));
                builder.mkdir(&format!("{}/letsencrypt", traefik_path()));

                // Generate and write static config
                let static_config = generate_traefik_static_config(config);
                let config_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &static_config
                );
                builder.add(format!(
                    "echo '{}' | base64 -d > {}/traefik.yml",
                    config_base64, traefik_path()
                ));

                // Generate and write docker-compose
                let compose = generate_traefik_compose(config);
                let compose_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &compose
                );
                builder.add(format!(
                    "echo '{}' | base64 -d > {}/docker-compose.yml",
                    compose_base64, traefik_path()
                ));
            }
            ProxyType::Caddy => {
                builder.mkdir(caddy_path());
                builder.mkdir(&format!("{}/dynamic", caddy_path()));
                builder.mkdir(&format!("{}/data", caddy_path()));
                builder.mkdir(&format!("{}/config", caddy_path()));

                // Generate and write Caddyfile
                let caddyfile = generate_caddyfile(config);
                let caddyfile_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &caddyfile
                );
                builder.add(format!(
                    "echo '{}' | base64 -d > {}/Caddyfile",
                    caddyfile_base64, caddy_path()
                ));

                // Generate and write docker-compose
                let compose = generate_caddy_compose(config);
                let compose_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &compose
                );
                builder.add(format!(
                    "echo '{}' | base64 -d > {}/docker-compose.yml",
                    compose_base64, caddy_path()
                ));

                // Write default dynamic import
                builder.add(format!(
                    "echo 'import /dynamic/*.caddy' > {}/dynamic/Caddyfile",
                    caddy_path()
                ));
            }
            ProxyType::None => unreachable!(),
        }

        // Ensure networks exist
        for network in &config.networks {
            builder.add(format!(
                "docker network create --attachable {} 2>/dev/null || true",
                network
            ));
        }

        let compose_dir = match config.proxy_type {
            ProxyType::Traefik => traefik_path(),
            ProxyType::Caddy => caddy_path(),
            ProxyType::None => unreachable!(),
        };

        // Pull image
        builder.echo("Pulling proxy image...");
        builder.add(format!("docker compose -f {}/docker-compose.yml pull", compose_dir));

        // Stop existing proxy
        builder.add("docker stop coolify-proxy 2>/dev/null || true");
        builder.add("docker rm -f coolify-proxy 2>/dev/null || true");

        // Wait for container to be removed
        builder.add(r#"for i in {1..10}; do
            if ! docker ps -a --format "{{.Names}}" | grep -q "^coolify-proxy$"; then
                break
            fi
            echo "Waiting for coolify-proxy to be removed... ($i/10)"
            sleep 1
        done"#);

        // Start proxy
        builder.echo("Starting proxy container...");
        builder.add(format!(
            "docker compose -f {}/docker-compose.yml up -d --wait --remove-orphans",
            compose_dir
        ));

        // Connect to additional networks
        for network in &config.networks {
            builder.add(format!(
                "docker network connect {} coolify-proxy 2>/dev/null || true",
                network
            ));
        }

        builder.echo("Proxy started successfully!");

        let commands = builder.build();

        for cmd in &commands {
            let result = session.execute(cmd)
                .await
                .map_err(|e| ActionError::command_failed(cmd, e.to_string()))?;

            if result.exit_code != 0
                && !cmd.contains("|| true")
                && !cmd.contains("2>/dev/null")
                && !cmd.starts_with("echo")
                && !cmd.starts_with("for ")
            {
                warn!("Proxy start command failed: {}", result.stderr);
                return Err(ActionError::command_failed(cmd, result.stderr));
            }
        }

        // Verify proxy is running
        let status = session.execute("docker inspect -f '{{.State.Running}}' coolify-proxy 2>/dev/null")
            .await
            .map_err(|e| ActionError::command_failed("check proxy", e.to_string()))?;

        if status.stdout.trim() != "true" {
            return Err(ActionError::new(
                "PROXY_NOT_RUNNING",
                "Proxy container failed to start"
            ));
        }

        Ok(ActionResult::success(format!("{} proxy started successfully", config.proxy_type.as_str()))
            .with_duration(start.elapsed())
            .with_commands(commands))
    }
}
