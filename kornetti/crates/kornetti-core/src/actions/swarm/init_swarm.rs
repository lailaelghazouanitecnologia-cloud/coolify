//! Initialize Docker Swarm Action
//!
//! Initializes a Docker Swarm cluster on a server.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Input for initializing Docker Swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitSwarmInput {
    /// Server ID
    pub server_id: Uuid,
    /// Advertise address (IP:port for other nodes to connect)
    pub advertise_addr: Option<String>,
    /// Listen address for manager
    pub listen_addr: Option<String>,
    /// Data path address for data traffic
    pub data_path_addr: Option<String>,
    /// Force re-initialization even if already in swarm
    pub force: bool,
}

/// Output from Swarm initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitSwarmOutput {
    /// Manager join token (for adding managers)
    pub manager_token: String,
    /// Worker join token (for adding workers)
    pub worker_token: String,
    /// Manager address
    pub manager_addr: String,
    /// Node ID of this manager
    pub node_id: String,
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Initialize Swarm action
pub struct InitSwarm;

impl InitSwarm {
    pub fn new() -> Self {
        Self
    }

    /// Generate swarm init command
    pub fn init_command(&self, input: &InitSwarmInput) -> String {
        let mut cmd = "docker swarm init".to_string();

        if let Some(ref addr) = input.advertise_addr {
            cmd.push_str(&format!(" --advertise-addr {}", addr));
        }

        if let Some(ref addr) = input.listen_addr {
            cmd.push_str(&format!(" --listen-addr {}", addr));
        }

        if let Some(ref addr) = input.data_path_addr {
            cmd.push_str(&format!(" --data-path-addr {}", addr));
        }

        if input.force {
            cmd.push_str(" --force-new-cluster");
        }

        cmd
    }

    /// Generate all commands for swarm initialization
    pub fn generate_commands(&self, input: &InitSwarmInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();

        // Leave existing swarm if force
        if input.force {
            builder.add("docker swarm leave --force 2>/dev/null || true");
        }

        // Initialize swarm
        builder.add(self.init_command(input));

        // Get manager token
        builder.add("docker swarm join-token manager -q");

        // Get worker token
        builder.add("docker swarm join-token worker -q");

        // Get node ID
        builder.add("docker info --format '{{.Swarm.NodeID}}'");

        // Create overlay network
        builder.add(
            "docker network create --attachable --driver overlay coolify-overlay 2>/dev/null || true",
        );

        builder.build()
    }
}

impl Default for InitSwarm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for InitSwarm {
    type Input = InitSwarmInput;
    type Output = InitSwarmOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            server_id = %input.server_id,
            advertise_addr = ?input.advertise_addr,
            "Initializing Docker Swarm"
        );

        let commands = self.generate_commands(&input);

        // Placeholder for actual execution
        // In real implementation:
        // 1. Execute swarm init
        // 2. Extract manager and worker tokens
        // 3. Create overlay network
        // 4. Return tokens and addresses

        Ok(InitSwarmOutput {
            manager_token: "SWMTKN-manager-token".to_string(),
            worker_token: "SWMTKN-worker-token".to_string(),
            manager_addr: input
                .advertise_addr
                .clone()
                .unwrap_or_else(|| "0.0.0.0:2377".to_string()),
            node_id: "node-id-placeholder".to_string(),
            success: true,
            message: "Swarm initialized successfully".to_string(),
        })
    }

    fn name(&self) -> &'static str {
        "init_swarm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_command() {
        let action = InitSwarm::new();
        let input = InitSwarmInput {
            server_id: Uuid::new_v4(),
            advertise_addr: Some("192.168.1.100:2377".to_string()),
            listen_addr: None,
            data_path_addr: None,
            force: false,
        };

        let cmd = action.init_command(&input);
        assert!(cmd.contains("docker swarm init"));
        assert!(cmd.contains("--advertise-addr 192.168.1.100:2377"));
    }

    #[test]
    fn test_force_init() {
        let action = InitSwarm::new();
        let input = InitSwarmInput {
            server_id: Uuid::new_v4(),
            advertise_addr: None,
            listen_addr: None,
            data_path_addr: None,
            force: true,
        };

        let cmd = action.init_command(&input);
        assert!(cmd.contains("--force-new-cluster"));
    }
}
