//! Server metrics collection job
//!
//! This job periodically collects system metrics from servers
//! including CPU, memory, disk, and network statistics.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Server metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub server_id: Uuid,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub disk_percent: f32,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub load_average: [f32; 3],
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub docker_containers_running: u32,
    pub docker_containers_total: u32,
    pub recorded_at: DateTime<Utc>,
}

/// Context for server metrics job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetricsContext {
    pub server_id: Uuid,
    /// Whether to store metrics in the database
    pub persist: bool,
}

/// Server metrics collection job
pub struct ServerMetricsJob;

impl ServerMetricsJob {
    pub fn new() -> Self {
        Self
    }

    /// Run the metrics collection job
    pub async fn run(&self, ctx: ServerMetricsContext) -> Result<ServerMetrics> {
        tracing::info!(
            server_id = %ctx.server_id,
            "Collecting server metrics"
        );

        // Command to collect all metrics in one SSH call
        let cmd = r#"
echo "CPU:$(top -bn1 | grep 'Cpu(s)' | awk '{print $2}' | cut -d'%' -f1)"
echo "MEM:$(free -m | awk '/Mem:/ {print $3","$2}')"
echo "DISK:$(df -BG / | awk 'NR==2 {gsub("G",""); print $3","$2}')"
echo "LOAD:$(cat /proc/loadavg | awk '{print $1","$2","$3}')"
echo "NET:$(cat /proc/net/dev | awk '/eth0|ens|enp/ {gsub(":", ""); print $2","$10}' | head -1)"
echo "DOCKER:$(docker info --format '{{.ContainersRunning}},{{.Containers}}' 2>/dev/null || echo '0,0')"
"#;

        // TODO: Execute command on server via SSH
        // let output = ssh_client.execute(cmd).await?;
        // let metrics = self.parse_metrics_output(ctx.server_id, &output)?;

        // For now, return placeholder metrics
        let metrics = ServerMetrics {
            server_id: ctx.server_id,
            cpu_percent: 0.0,
            memory_percent: 0.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            disk_percent: 0.0,
            disk_used_gb: 0,
            disk_total_gb: 0,
            load_average: [0.0, 0.0, 0.0],
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            docker_containers_running: 0,
            docker_containers_total: 0,
            recorded_at: Utc::now(),
        };

        Ok(metrics)
    }

    /// Parse metrics output from the shell command
    pub fn parse_metrics_output(&self, server_id: Uuid, output: &str) -> Result<ServerMetrics> {
        let mut cpu_percent = 0.0f32;
        let mut memory_used_mb = 0u64;
        let mut memory_total_mb = 0u64;
        let mut disk_used_gb = 0u64;
        let mut disk_total_gb = 0u64;
        let mut load_average = [0.0f32; 3];
        let mut network_rx_bytes = 0u64;
        let mut network_tx_bytes = 0u64;
        let mut docker_running = 0u32;
        let mut docker_total = 0u32;

        for line in output.lines() {
            if let Some(value) = line.strip_prefix("CPU:") {
                cpu_percent = value.trim().parse().unwrap_or(0.0);
            } else if let Some(value) = line.strip_prefix("MEM:") {
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() == 2 {
                    memory_used_mb = parts[0].trim().parse().unwrap_or(0);
                    memory_total_mb = parts[1].trim().parse().unwrap_or(0);
                }
            } else if let Some(value) = line.strip_prefix("DISK:") {
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() == 2 {
                    disk_used_gb = parts[0].trim().parse().unwrap_or(0);
                    disk_total_gb = parts[1].trim().parse().unwrap_or(0);
                }
            } else if let Some(value) = line.strip_prefix("LOAD:") {
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() >= 3 {
                    load_average = [
                        parts[0].trim().parse().unwrap_or(0.0),
                        parts[1].trim().parse().unwrap_or(0.0),
                        parts[2].trim().parse().unwrap_or(0.0),
                    ];
                }
            } else if let Some(value) = line.strip_prefix("NET:") {
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() == 2 {
                    network_rx_bytes = parts[0].trim().parse().unwrap_or(0);
                    network_tx_bytes = parts[1].trim().parse().unwrap_or(0);
                }
            } else if let Some(value) = line.strip_prefix("DOCKER:") {
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() == 2 {
                    docker_running = parts[0].trim().parse().unwrap_or(0);
                    docker_total = parts[1].trim().parse().unwrap_or(0);
                }
            }
        }

        let memory_percent = if memory_total_mb > 0 {
            (memory_used_mb as f32 / memory_total_mb as f32) * 100.0
        } else {
            0.0
        };

        let disk_percent = if disk_total_gb > 0 {
            (disk_used_gb as f32 / disk_total_gb as f32) * 100.0
        } else {
            0.0
        };

        Ok(ServerMetrics {
            server_id,
            cpu_percent,
            memory_percent,
            memory_used_mb,
            memory_total_mb,
            disk_percent,
            disk_used_gb,
            disk_total_gb,
            load_average,
            network_rx_bytes,
            network_tx_bytes,
            docker_containers_running: docker_running,
            docker_containers_total: docker_total,
            recorded_at: Utc::now(),
        })
    }

    /// Check if metrics indicate any alerts should be triggered
    pub fn check_alerts(&self, metrics: &ServerMetrics) -> Vec<MetricAlert> {
        let mut alerts = Vec::new();

        // High CPU usage
        if metrics.cpu_percent > 90.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Critical,
                metric: "cpu".to_string(),
                message: format!("CPU usage is critically high: {:.1}%", metrics.cpu_percent),
                value: metrics.cpu_percent,
                threshold: 90.0,
            });
        } else if metrics.cpu_percent > 80.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Warning,
                metric: "cpu".to_string(),
                message: format!("CPU usage is high: {:.1}%", metrics.cpu_percent),
                value: metrics.cpu_percent,
                threshold: 80.0,
            });
        }

        // High memory usage
        if metrics.memory_percent > 95.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Critical,
                metric: "memory".to_string(),
                message: format!("Memory usage is critically high: {:.1}%", metrics.memory_percent),
                value: metrics.memory_percent,
                threshold: 95.0,
            });
        } else if metrics.memory_percent > 85.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Warning,
                metric: "memory".to_string(),
                message: format!("Memory usage is high: {:.1}%", metrics.memory_percent),
                value: metrics.memory_percent,
                threshold: 85.0,
            });
        }

        // High disk usage
        if metrics.disk_percent > 95.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Critical,
                metric: "disk".to_string(),
                message: format!("Disk usage is critically high: {:.1}%", metrics.disk_percent),
                value: metrics.disk_percent,
                threshold: 95.0,
            });
        } else if metrics.disk_percent > 85.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Warning,
                metric: "disk".to_string(),
                message: format!("Disk usage is high: {:.1}%", metrics.disk_percent),
                value: metrics.disk_percent,
                threshold: 85.0,
            });
        }

        // High load average (compared to typical 1.0 per CPU core threshold)
        if metrics.load_average[0] > 10.0 {
            alerts.push(MetricAlert {
                severity: AlertSeverity::Warning,
                metric: "load".to_string(),
                message: format!("System load is high: {:.2}", metrics.load_average[0]),
                value: metrics.load_average[0],
                threshold: 10.0,
            });
        }

        alerts
    }
}

impl Default for ServerMetricsJob {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAlert {
    pub severity: AlertSeverity,
    pub metric: String,
    pub message: String,
    pub value: f32,
    pub threshold: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metrics_output() {
        let output = r#"CPU:25.5
MEM:4096,8192
DISK:50,100
LOAD:1.5,1.2,0.8
NET:1000000,500000
DOCKER:5,10"#;

        let job = ServerMetricsJob::new();
        let metrics = job.parse_metrics_output(Uuid::new_v4(), output).unwrap();

        assert_eq!(metrics.cpu_percent, 25.5);
        assert_eq!(metrics.memory_used_mb, 4096);
        assert_eq!(metrics.memory_total_mb, 8192);
        assert_eq!(metrics.memory_percent, 50.0);
        assert_eq!(metrics.disk_used_gb, 50);
        assert_eq!(metrics.disk_total_gb, 100);
        assert_eq!(metrics.docker_containers_running, 5);
    }

    #[test]
    fn test_check_alerts() {
        let job = ServerMetricsJob::new();

        let high_cpu_metrics = ServerMetrics {
            server_id: Uuid::new_v4(),
            cpu_percent: 95.0,
            memory_percent: 50.0,
            memory_used_mb: 4096,
            memory_total_mb: 8192,
            disk_percent: 50.0,
            disk_used_gb: 50,
            disk_total_gb: 100,
            load_average: [1.0, 1.0, 1.0],
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            docker_containers_running: 0,
            docker_containers_total: 0,
            recorded_at: Utc::now(),
        };

        let alerts = job.check_alerts(&high_cpu_metrics);
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].metric, "cpu");
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
    }
}
