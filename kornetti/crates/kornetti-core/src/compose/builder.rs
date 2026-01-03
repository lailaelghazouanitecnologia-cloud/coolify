//! Docker Compose builder
//!
//! Fluent API for building Docker Compose files.

use std::collections::HashMap;
use super::{
    ComposeFile, Service, Network, Volume, HealthCheck, Deploy,
    Resources, ResourceSpec, StringOrList, StringOrNull,
};

/// Builder for Docker Compose files
#[derive(Debug, Clone, Default)]
pub struct ComposeBuilder {
    compose: ComposeFile,
}

impl ComposeBuilder {
    /// Create a new compose builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the compose version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.compose.version = Some(version.into());
        self
    }

    /// Add a service using a builder
    pub fn service(mut self, name: impl Into<String>, service: ServiceBuilder) -> Self {
        self.compose.services.insert(name.into(), service.build());
        self
    }

    /// Add a network
    pub fn network(mut self, name: impl Into<String>, network: NetworkBuilder) -> Self {
        self.compose.networks.insert(name.into(), network.build());
        self
    }

    /// Add an external network
    pub fn external_network(mut self, name: impl Into<String>) -> Self {
        self.compose.networks.insert(name.into(), Network {
            external: Some(true),
            ..Default::default()
        });
        self
    }

    /// Add a volume
    pub fn volume(mut self, name: impl Into<String>, volume: VolumeBuilder) -> Self {
        self.compose.volumes.insert(name.into(), volume.build());
        self
    }

    /// Add an external volume
    pub fn external_volume(mut self, name: impl Into<String>) -> Self {
        self.compose.volumes.insert(name.into(), Volume {
            external: Some(true),
            ..Default::default()
        });
        self
    }

    /// Build the compose file
    pub fn build(self) -> ComposeFile {
        self.compose
    }

    /// Build and convert to YAML
    pub fn to_yaml(self) -> Result<String, serde_yaml::Error> {
        self.compose.to_yaml()
    }
}

/// Builder for service configuration
#[derive(Debug, Clone, Default)]
pub struct ServiceBuilder {
    service: Service,
}

impl ServiceBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the image
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.service.image = Some(image.into());
        self
    }

    /// Set container name
    pub fn container_name(mut self, name: impl Into<String>) -> Self {
        self.service.container_name = Some(name.into());
        self
    }

    /// Set command
    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.service.command = Some(StringOrList::String(command.into()));
        self
    }

    /// Set command as list
    pub fn command_list(mut self, commands: Vec<String>) -> Self {
        self.service.command = Some(StringOrList::List(commands));
        self
    }

    /// Add a port mapping
    pub fn port(mut self, host: u16, container: u16) -> Self {
        self.service.ports.push(format!("{}:{}", host, container));
        self
    }

    /// Add a port mapping with protocol
    pub fn port_with_protocol(mut self, host: u16, container: u16, protocol: &str) -> Self {
        self.service.ports.push(format!("{}:{}/{}", host, container, protocol));
        self
    }

    /// Add an exposed port
    pub fn expose(mut self, port: u16) -> Self {
        self.service.expose.push(port.to_string());
        self
    }

    /// Add a volume mount
    pub fn volume(mut self, host: impl Into<String>, container: impl Into<String>) -> Self {
        self.service.volumes.push(format!("{}:{}", host.into(), container.into()));
        self
    }

    /// Add a volume mount with options
    pub fn volume_with_opts(
        mut self,
        host: impl Into<String>,
        container: impl Into<String>,
        opts: impl Into<String>,
    ) -> Self {
        self.service.volumes.push(format!("{}:{}:{}", host.into(), container.into(), opts.into()));
        self
    }

    /// Add a named volume
    pub fn named_volume(mut self, name: impl Into<String>, container: impl Into<String>) -> Self {
        self.service.volumes.push(format!("{}:{}", name.into(), container.into()));
        self
    }

    /// Add an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.service.environment.insert(key.into(), StringOrNull::String(value.into()));
        self
    }

    /// Add multiple environment variables
    pub fn envs(mut self, vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        for (k, v) in vars {
            self.service.environment.insert(k.into(), StringOrNull::String(v.into()));
        }
        self
    }

    /// Set restart policy
    pub fn restart(mut self, policy: impl Into<String>) -> Self {
        self.service.restart = Some(policy.into());
        self
    }

    /// Restart always
    pub fn restart_always(self) -> Self {
        self.restart("always")
    }

    /// Restart unless stopped
    pub fn restart_unless_stopped(self) -> Self {
        self.restart("unless-stopped")
    }

    /// Restart on failure
    pub fn restart_on_failure(self) -> Self {
        self.restart("on-failure")
    }

    /// Add a label
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.service.labels.insert(key.into(), value.into());
        self
    }

    /// Add multiple labels
    pub fn labels(mut self, labels: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        for (k, v) in labels {
            self.service.labels.insert(k.into(), v.into());
        }
        self
    }

    /// Add a network
    pub fn network(mut self, network: impl Into<String>) -> Self {
        self.service.networks.push(network.into());
        self
    }

    /// Add dependency on another service
    pub fn depends_on(mut self, service: impl Into<String>) -> Self {
        self.service.depends_on.push(service.into());
        self
    }

    /// Set health check
    pub fn healthcheck(mut self, test: Vec<String>) -> Self {
        self.service.healthcheck = Some(HealthCheck {
            test: Some(StringOrList::List(test)),
            ..Default::default()
        });
        self
    }

    /// Set health check with full config
    pub fn healthcheck_full(
        mut self,
        test: Vec<String>,
        interval: impl Into<String>,
        timeout: impl Into<String>,
        retries: u32,
        start_period: impl Into<String>,
    ) -> Self {
        self.service.healthcheck = Some(HealthCheck {
            test: Some(StringOrList::List(test)),
            interval: Some(interval.into()),
            timeout: Some(timeout.into()),
            retries: Some(retries),
            start_period: Some(start_period.into()),
            disable: None,
        });
        self
    }

    /// Disable health check
    pub fn healthcheck_disabled(mut self) -> Self {
        self.service.healthcheck = Some(HealthCheck {
            disable: Some(true),
            ..Default::default()
        });
        self
    }

    /// Set memory limit
    pub fn mem_limit(mut self, limit: impl Into<String>) -> Self {
        self.service.mem_limit = Some(limit.into());
        self
    }

    /// Set CPU limit
    pub fn cpus(mut self, cpus: f64) -> Self {
        self.service.cpus = Some(cpus);
        self
    }

    /// Set resource limits via deploy
    pub fn resources(mut self, memory: Option<String>, cpus: Option<String>) -> Self {
        self.service.deploy = Some(Deploy {
            resources: Some(Resources {
                limits: Some(ResourceSpec { memory, cpus }),
                reservations: None,
            }),
            ..Default::default()
        });
        self
    }

    /// Set user
    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.service.user = Some(user.into());
        self
    }

    /// Set working directory
    pub fn working_dir(mut self, dir: impl Into<String>) -> Self {
        self.service.working_dir = Some(dir.into());
        self
    }

    /// Add capability
    pub fn cap_add(mut self, cap: impl Into<String>) -> Self {
        self.service.cap_add.push(cap.into());
        self
    }

    /// Drop capability
    pub fn cap_drop(mut self, cap: impl Into<String>) -> Self {
        self.service.cap_drop.push(cap.into());
        self
    }

    /// Enable privileged mode (use with caution!)
    pub fn privileged(mut self) -> Self {
        self.service.privileged = Some(true);
        self
    }

    /// Enable TTY
    pub fn tty(mut self) -> Self {
        self.service.tty = Some(true);
        self
    }

    /// Enable stdin
    pub fn stdin_open(mut self) -> Self {
        self.service.stdin_open = Some(true);
        self
    }

    /// Add DNS server
    pub fn dns(mut self, server: impl Into<String>) -> Self {
        self.service.dns.push(server.into());
        self
    }

    /// Add extra host
    pub fn extra_host(mut self, host: impl Into<String>, ip: impl Into<String>) -> Self {
        self.service.extra_hosts.push(format!("{}:{}", host.into(), ip.into()));
        self
    }

    /// Build the service
    pub fn build(self) -> Service {
        self.service
    }
}

/// Builder for network configuration
#[derive(Debug, Clone, Default)]
pub struct NetworkBuilder {
    network: Network,
}

impl NetworkBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the driver
    pub fn driver(mut self, driver: impl Into<String>) -> Self {
        self.network.driver = Some(driver.into());
        self
    }

    /// Mark as external
    pub fn external(mut self) -> Self {
        self.network.external = Some(true);
        self
    }

    /// Set network name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.network.name = Some(name.into());
        self
    }

    /// Add driver option
    pub fn driver_opt(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.network.driver_opts.insert(key.into(), value.into());
        self
    }

    /// Add label
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.network.labels.insert(key.into(), value.into());
        self
    }

    /// Build the network
    pub fn build(self) -> Network {
        self.network
    }
}

/// Builder for volume configuration
#[derive(Debug, Clone, Default)]
pub struct VolumeBuilder {
    volume: Volume,
}

impl VolumeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the driver
    pub fn driver(mut self, driver: impl Into<String>) -> Self {
        self.volume.driver = Some(driver.into());
        self
    }

    /// Mark as external
    pub fn external(mut self) -> Self {
        self.volume.external = Some(true);
        self
    }

    /// Set volume name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.volume.name = Some(name.into());
        self
    }

    /// Add driver option
    pub fn driver_opt(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.volume.driver_opts.insert(key.into(), value.into());
        self
    }

    /// Add label
    pub fn label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.volume.labels.insert(key.into(), value.into());
        self
    }

    /// Build the volume
    pub fn build(self) -> Volume {
        self.volume
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compose_builder() {
        let compose = ComposeBuilder::new()
            .version("3.8")
            .external_network("coolify")
            .service("web", ServiceBuilder::new()
                .image("nginx:alpine")
                .container_name("my-web")
                .port(8080, 80)
                .env("NGINX_HOST", "example.com")
                .restart_unless_stopped()
                .network("coolify")
                .healthcheck_full(
                    vec!["CMD".into(), "curl".into(), "-f".into(), "http://localhost/".into()],
                    "30s",
                    "10s",
                    3,
                    "40s",
                )
                .mem_limit("512m")
            )
            .build();

        assert_eq!(compose.services.len(), 1);
        assert!(compose.services.contains_key("web"));

        let web = compose.services.get("web").unwrap();
        assert_eq!(web.image, Some("nginx:alpine".to_string()));
        assert_eq!(web.container_name, Some("my-web".to_string()));
        assert!(web.healthcheck.is_some());

        let yaml = compose.to_yaml().unwrap();
        assert!(yaml.contains("nginx:alpine"));
        assert!(yaml.contains("8080:80"));
    }

    #[test]
    fn test_service_builder() {
        let service = ServiceBuilder::new()
            .image("postgres:15")
            .container_name("my-db")
            .env("POSTGRES_USER", "admin")
            .env("POSTGRES_PASSWORD", "secret")
            .volume("/data/postgres", "/var/lib/postgresql/data")
            .port(5432, 5432)
            .restart_always()
            .resources(Some("1g".to_string()), Some("0.5".to_string()))
            .build();

        assert_eq!(service.image, Some("postgres:15".to_string()));
        assert_eq!(service.restart, Some("always".to_string()));
        assert_eq!(service.environment.len(), 2);
        assert!(service.deploy.is_some());
    }
}
