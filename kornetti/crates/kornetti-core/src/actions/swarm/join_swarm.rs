//! Join Docker Swarm Action
//!
//! Joins a server to an existing Docker Swarm cluster.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Role to join swarm as
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmJoinRole {
    Manager,
    Worker,
}

/// Input for joining Docker Swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSwarmInput {
    /// Server ID of the server joining
    pub server_id: Uuid,
    /// Join token from manager
    pub token: String,
    /// Manager address(es) to connect to
    pub manager_addrs: Vec<String>,
    /// Role to join as
    pub role: SwarmJoinRole,
    /// Advertise address of this node
    pub advertise_addr: Option<String>,
    /// Listen address for management
    pub listen_addr: Option<String>,
    /// Data path address
    pub data_path_addr: Option<String>,
}

/// Output from joining Swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinSwarmOutput {
    /// Node ID assigned
    pub node_id: String,
    /// Role joined as
    pub role: SwarmJoinRole,
    /// Connected manager address
    pub connected_manager: String,
    /// Success status
    pub success: bool,
    /// Message
    pub message: String,
}

/// Join Swarm action
pub struct JoinSwarm;

impl JoinSwarm {
    pub fn new() -> Self {
        Self
    }

    /// Generate swarm join command
    pub fn join_command(&self, input: &JoinSwarmInput) -> String {
        let mut cmd = format!("docker swarm join --token {}", input.token);

        if let Some(ref addr) = input.advertise_addr {
            cmd.push_str(&format!(" --advertise-addr {}", addr));
        }

        if let Some(ref addr) = input.listen_addr {
            cmd.push_str(&format!(" --listen-addr {}", addr));
        }

        if let Some(ref addr) = input.data_path_addr {
            cmd.push_str(&format!(" --data-path-addr {}", addr));
        }

        // Add manager addresses
        for addr in &input.manager_addrs {
            cmd.push_str(&format!(" {}", addr));
            break; // Only first address is used in join command
        }

        cmd
    }

    /// Generate all commands for joining swarm
    pub fn generate_commands(&self, input: &JoinSwarmInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();

        // Leave existing swarm if any
        builder.add("docker swarm leave --force 2>/dev/null || true");

        // Join swarm
        builder.add(self.join_command(input));

        // Get node ID
        builder.add("docker info --format '{{.Swarm.NodeID}}'");

        // Verify swarm status
        builder.add("docker info --format '{{.Swarm.LocalNodeState}}'");

        builder.build()
    }
}

impl Default for JoinSwarm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for JoinSwarm {
    type Input = JoinSwarmInput;
    type Output = JoinSwarmOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            server_id = %input.server_id,
            role = ?input.role,
            managers = ?input.manager_addrs,
            "Joining Docker Swarm"
        );

        if input.manager_addrs.is_empty() {
            return Err(ActionError::configuration_error(
                "At least one manager address is required",
            ));
        }

        let commands = self.generate_commands(&input);

        // Placeholder for actual execution
        Ok(JoinSwarmOutput {
            node_id: "node-id-placeholder".to_string(),
            role: input.role,
            connected_manager: input.manager_addrs[0].clone(),
            success: true,
            message: format!("Successfully joined swarm as {:?}", input.role),
        })
    }

    fn name(&self) -> &'static str {
        "join_swarm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_command() {
        let action = JoinSwarm::new();
        let input = JoinSwarmInput {
            server_id: Uuid::new_v4(),
            token: "SWMTKN-test-token".to_string(),
            manager_addrs: vec!["192.168.1.100:2377".to_string()],
            role: SwarmJoinRole::Worker,
            advertise_addr: Some("192.168.1.101:2377".to_string()),
            listen_addr: None,
            data_path_addr: None,
        };

        let cmd = action.join_command(&input);
        assert!(cmd.contains("docker swarm join"));
        assert!(cmd.contains("SWMTKN-test-token"));
        assert!(cmd.contains("192.168.1.100:2377"));
        assert!(cmd.contains("--advertise-addr 192.168.1.101:2377"));
    }
}
