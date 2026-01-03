//! Container operations

use kornetti_core::Result;

/// Container run configuration
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    pub name: String,
    pub image: String,
    pub ports: Vec<PortBinding>,
    pub volumes: Vec<VolumeMount>,
    pub environment: Vec<(String, String)>,
    pub labels: Vec<(String, String)>,
    pub network: Option<String>,
    pub restart_policy: RestartPolicy,
    pub resources: ResourceConfig,
}

#[derive(Debug, Clone)]
pub struct PortBinding {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub source: String,
    pub target: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum RestartPolicy {
    #[default]
    No,
    Always,
    OnFailure,
    UnlessStopped,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceConfig {
    pub memory_limit: Option<u64>,
    pub memory_reservation: Option<u64>,
    pub cpu_limit: Option<f64>,
    pub cpu_reservation: Option<f64>,
}

impl ContainerConfig {
    /// Generate docker run command from config
    pub fn to_docker_run_command(&self) -> String {
        let mut cmd = vec!["docker", "run", "-d"];

        cmd.push("--name");
        cmd.push(&self.name);

        for PortBinding { container_port, host_port, protocol } in &self.ports {
            let proto = match protocol {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
            };
            if let Some(hp) = host_port {
                cmd.push("-p");
                cmd.push(&format!("{}:{}/{}", hp, container_port, proto));
            }
        }

        for VolumeMount { source, target, read_only } in &self.volumes {
            cmd.push("-v");
            if *read_only {
                cmd.push(&format!("{}:{}:ro", source, target));
            } else {
                cmd.push(&format!("{}:{}", source, target));
            }
        }

        for (key, value) in &self.environment {
            cmd.push("-e");
            cmd.push(&format!("{}={}", key, value));
        }

        for (key, value) in &self.labels {
            cmd.push("--label");
            cmd.push(&format!("{}={}", key, value));
        }

        if let Some(network) = &self.network {
            cmd.push("--network");
            cmd.push(network);
        }

        let restart = match self.restart_policy {
            RestartPolicy::No => "no",
            RestartPolicy::Always => "always",
            RestartPolicy::OnFailure => "on-failure",
            RestartPolicy::UnlessStopped => "unless-stopped",
        };
        cmd.push("--restart");
        cmd.push(restart);

        if let Some(mem) = self.resources.memory_limit {
            cmd.push("--memory");
            cmd.push(&format!("{}m", mem / 1024 / 1024));
        }

        if let Some(cpu) = self.resources.cpu_limit {
            cmd.push("--cpus");
            cmd.push(&cpu.to_string());
        }

        cmd.push(&self.image);

        cmd.join(" ")
    }
}
