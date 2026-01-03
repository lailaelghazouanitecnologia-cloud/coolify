//! SSH executor utilities

use kornetti_core::{Result, models::Server, traits::CommandOutput};
use crate::SshClient;

/// Helper struct for executing common commands
pub struct SshExecutor {
    client: SshClient,
}

impl SshExecutor {
    pub async fn new(server: &Server) -> Result<Self> {
        let client = SshClient::connect(server).await?;
        Ok(Self { client })
    }

    /// Check if Docker is installed
    pub async fn check_docker(&self) -> Result<Option<String>> {
        let output = self.client.execute("docker --version 2>/dev/null || echo ''").await?;
        if output.stdout.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(output.stdout.trim().to_string()))
        }
    }

    /// Get system info
    pub async fn get_system_info(&self) -> Result<SystemInfo> {
        let hostname = self.client.execute("hostname").await?.stdout.trim().to_string();
        let os = self.client.execute("cat /etc/os-release | grep PRETTY_NAME | cut -d'=' -f2 | tr -d '\"'").await?.stdout.trim().to_string();
        let kernel = self.client.execute("uname -r").await?.stdout.trim().to_string();
        let arch = self.client.execute("uname -m").await?.stdout.trim().to_string();

        Ok(SystemInfo {
            hostname,
            os,
            kernel,
            arch,
        })
    }

    /// Get resource usage
    pub async fn get_resources(&self) -> Result<ResourceUsage> {
        // CPU usage
        let cpu_output = self.client.execute(
            "top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1"
        ).await?;
        let cpu_usage: f64 = cpu_output.stdout.trim().parse().unwrap_or(0.0);

        // Memory
        let mem_output = self.client.execute(
            "free -b | grep Mem | awk '{print $2,$3}'"
        ).await?;
        let mem_parts: Vec<u64> = mem_output.stdout
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (memory_total, memory_used) = if mem_parts.len() >= 2 {
            (mem_parts[0], mem_parts[1])
        } else {
            (0, 0)
        };

        // Disk
        let disk_output = self.client.execute(
            "df -B1 / | tail -1 | awk '{print $2,$3}'"
        ).await?;
        let disk_parts: Vec<u64> = disk_output.stdout
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (disk_total, disk_used) = if disk_parts.len() >= 2 {
            (disk_parts[0], disk_parts[1])
        } else {
            (0, 0)
        };

        Ok(ResourceUsage {
            cpu_usage,
            memory_total,
            memory_used,
            disk_total,
            disk_used,
        })
    }

    /// List running containers
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let output = self.client.execute(
            "docker ps --format '{{.ID}}|{{.Names}}|{{.Image}}|{{.Status}}' 2>/dev/null || echo ''"
        ).await?;

        let containers = output.stdout
            .lines()
            .filter(|line| !line.is_empty() && line.contains('|'))
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 4 {
                    Some(ContainerInfo {
                        id: parts[0].to_string(),
                        name: parts[1].to_string(),
                        image: parts[2].to_string(),
                        status: parts[3].to_string(),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(containers)
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub arch: String,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_total: u64,
    pub memory_used: u64,
    pub disk_total: u64,
    pub disk_used: u64,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
}
