//! Server Health Check Job
//!
//! Periodically checks server health, connectivity, and Docker status.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use kornetti_core::{
    Result, Error,
    models::{Server, ServerStatus},
};
use kornetti_ssh::SshClient;

/// Server check context
#[derive(Debug, Clone)]
pub struct ServerCheckContext {
    pub server_id: Uuid,
    pub check_docker: bool,
    pub check_disk: bool,
    pub check_memory: bool,
}

/// Server health check job
pub struct ServerCheckJob {
    ssh: Arc<SshClient>,
    context: ServerCheckContext,
}

impl ServerCheckJob {
    pub fn new(ssh: Arc<SshClient>, context: ServerCheckContext) -> Self {
        Self { ssh, context }
    }

    /// Run the health check
    #[instrument(skip(self, server))]
    pub async fn run(&self, server: &Server) -> Result<HealthCheckResult> {
        info!(server_id = %server.id, ip = %server.ip, "Running server health check");

        let start_time = Utc::now();
        let mut result = HealthCheckResult {
            server_id: server.id,
            checked_at: start_time,
            is_reachable: false,
            docker_running: false,
            disk_usage: None,
            memory_usage: None,
            cpu_load: None,
            errors: Vec::new(),
        };

        // Check SSH connectivity
        match self.ssh.check_connection(server).await {
            Ok(true) => {
                result.is_reachable = true;
                info!(server_id = %server.id, "Server is reachable");
            }
            Ok(false) => {
                result.errors.push("SSH connection failed".to_string());
                warn!(server_id = %server.id, "Server is not reachable");
                return Ok(result);
            }
            Err(e) => {
                result.errors.push(format!("SSH error: {}", e));
                error!(server_id = %server.id, error = %e, "SSH connection error");
                return Ok(result);
            }
        }

        // Check Docker if requested
        if self.context.check_docker {
            match self.check_docker(server).await {
                Ok(running) => {
                    result.docker_running = running;
                    if !running {
                        result.errors.push("Docker daemon not running".to_string());
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Docker check failed: {}", e));
                }
            }
        }

        // Check disk usage if requested
        if self.context.check_disk {
            match self.check_disk_usage(server).await {
                Ok(usage) => {
                    result.disk_usage = Some(usage);
                    if usage.percent_used > 90.0 {
                        result.errors.push(format!("Disk usage critical: {:.1}%", usage.percent_used));
                    } else if usage.percent_used > 80.0 {
                        warn!(server_id = %server.id, usage = usage.percent_used, "Disk usage high");
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Disk check failed: {}", e));
                }
            }
        }

        // Check memory if requested
        if self.context.check_memory {
            match self.check_memory_usage(server).await {
                Ok(usage) => {
                    result.memory_usage = Some(usage);
                    if usage.percent_used > 95.0 {
                        result.errors.push(format!("Memory critical: {:.1}%", usage.percent_used));
                    }
                }
                Err(e) => {
                    result.errors.push(format!("Memory check failed: {}", e));
                }
            }
        }

        // Check CPU load
        match self.check_cpu_load(server).await {
            Ok(load) => {
                result.cpu_load = Some(load);
            }
            Err(e) => {
                debug!(error = %e, "CPU load check failed");
            }
        }

        Ok(result)
    }

    /// Check if Docker is running
    async fn check_docker(&self, server: &Server) -> Result<bool> {
        let result = self.ssh.execute(server, "docker info >/dev/null 2>&1 && echo 1 || echo 0").await?;
        Ok(result.output.stdout.trim() == "1")
    }

    /// Check disk usage
    async fn check_disk_usage(&self, server: &Server) -> Result<DiskUsage> {
        let result = self.ssh.execute(
            server,
            "df -h / | tail -1 | awk '{print $2,$3,$4,$5}'"
        ).await?;

        let parts: Vec<&str> = result.output.stdout.trim().split_whitespace().collect();
        if parts.len() < 4 {
            return Err(Error::Internal("Failed to parse disk usage".to_string()));
        }

        let percent_str = parts[3].trim_end_matches('%');
        let percent_used: f64 = percent_str.parse().unwrap_or(0.0);

        Ok(DiskUsage {
            total: parts[0].to_string(),
            used: parts[1].to_string(),
            available: parts[2].to_string(),
            percent_used,
        })
    }

    /// Check memory usage
    async fn check_memory_usage(&self, server: &Server) -> Result<MemoryUsage> {
        let result = self.ssh.execute(
            server,
            "free -m | grep Mem | awk '{print $2,$3,$4,$7}'"
        ).await?;

        let parts: Vec<&str> = result.output.stdout.trim().split_whitespace().collect();
        if parts.len() < 4 {
            return Err(Error::Internal("Failed to parse memory usage".to_string()));
        }

        let total: u64 = parts[0].parse().unwrap_or(0);
        let used: u64 = parts[1].parse().unwrap_or(0);
        let free: u64 = parts[2].parse().unwrap_or(0);
        let available: u64 = parts[3].parse().unwrap_or(0);

        let percent_used = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Ok(MemoryUsage {
            total_mb: total,
            used_mb: used,
            free_mb: free,
            available_mb: available,
            percent_used,
        })
    }

    /// Check CPU load
    async fn check_cpu_load(&self, server: &Server) -> Result<CpuLoad> {
        let result = self.ssh.execute(server, "cat /proc/loadavg").await?;

        let parts: Vec<&str> = result.output.stdout.trim().split_whitespace().collect();
        if parts.len() < 3 {
            return Err(Error::Internal("Failed to parse CPU load".to_string()));
        }

        Ok(CpuLoad {
            load_1: parts[0].parse().unwrap_or(0.0),
            load_5: parts[1].parse().unwrap_or(0.0),
            load_15: parts[2].parse().unwrap_or(0.0),
        })
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub server_id: Uuid,
    pub checked_at: DateTime<Utc>,
    pub is_reachable: bool,
    pub docker_running: bool,
    pub disk_usage: Option<DiskUsage>,
    pub memory_usage: Option<MemoryUsage>,
    pub cpu_load: Option<CpuLoad>,
    pub errors: Vec<String>,
}

impl HealthCheckResult {
    pub fn is_healthy(&self) -> bool {
        self.is_reachable && self.docker_running && self.errors.is_empty()
    }

    pub fn suggested_status(&self) -> ServerStatus {
        if !self.is_reachable {
            ServerStatus::Unreachable
        } else if !self.docker_running {
            ServerStatus::Error
        } else if !self.errors.is_empty() {
            ServerStatus::Warning
        } else {
            ServerStatus::Running
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub total: String,
    pub used: String,
    pub available: String,
    pub percent_used: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub available_mb: u64,
    pub percent_used: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLoad {
    pub load_1: f64,
    pub load_5: f64,
    pub load_15: f64,
}
