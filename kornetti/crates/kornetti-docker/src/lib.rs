//! Kornetti Docker
//!
//! Docker client and container orchestration.

pub mod client;
pub mod compose;
pub mod container;
pub mod image;

pub use client::DockerClient;

use async_trait::async_trait;
use kornetti_core::{
    Result, Error,
    models::Server,
    traits::{ContainerOrchestrator, Container, CommandOutput},
};
use kornetti_ssh::SshClient;

/// Remote Docker executor (executes Docker commands over SSH)
pub struct RemoteDockerExecutor;

#[async_trait]
impl ContainerOrchestrator for RemoteDockerExecutor {
    async fn list_containers(&self, server: &Server) -> Result<Vec<Container>> {
        let client = SshClient::connect(server).await?;
        let output = client.execute(
            "docker ps -a --format '{{json .}}'"
        ).await?;

        let containers = output.stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<DockerPsOutput>(line).ok())
            .map(|c| Container {
                id: c.id,
                name: c.names,
                image: c.image,
                status: c.status,
                created_at: c.created_at,
                labels: std::collections::HashMap::new(),
            })
            .collect();

        Ok(containers)
    }

    async fn start_container(&self, server: &Server, container_id: &str) -> Result<()> {
        let client = SshClient::connect(server).await?;
        let output = client.execute(&format!("docker start {}", container_id)).await?;
        if output.exit_code != 0 {
            return Err(Error::Docker(output.stderr));
        }
        Ok(())
    }

    async fn stop_container(&self, server: &Server, container_id: &str) -> Result<()> {
        let client = SshClient::connect(server).await?;
        let output = client.execute(&format!("docker stop {}", container_id)).await?;
        if output.exit_code != 0 {
            return Err(Error::Docker(output.stderr));
        }
        Ok(())
    }

    async fn remove_container(&self, server: &Server, container_id: &str) -> Result<()> {
        let client = SshClient::connect(server).await?;
        let output = client.execute(&format!("docker rm -f {}", container_id)).await?;
        if output.exit_code != 0 {
            return Err(Error::Docker(output.stderr));
        }
        Ok(())
    }

    async fn container_logs(
        &self,
        server: &Server,
        container_id: &str,
        tail: Option<usize>,
    ) -> Result<String> {
        let client = SshClient::connect(server).await?;
        let tail_arg = tail.map(|n| format!("--tail {}", n)).unwrap_or_default();
        let output = client.execute(&format!("docker logs {} {}", tail_arg, container_id)).await?;
        Ok(format!("{}{}", output.stdout, output.stderr))
    }

    async fn exec_in_container(
        &self,
        server: &Server,
        container_id: &str,
        command: &str,
    ) -> Result<CommandOutput> {
        let client = SshClient::connect(server).await?;
        client.execute(&format!("docker exec {} {}", container_id, command)).await
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DockerPsOutput {
    #[serde(rename = "ID")]
    id: String,
    names: String,
    image: String,
    status: String,
    created_at: String,
}
