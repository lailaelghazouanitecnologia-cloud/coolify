//! Standalone Docker Destination Model
//!
//! Represents a standalone Docker bridge network configuration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Standalone Docker destination (bridge network)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandaloneDocker {
    pub id: Uuid,
    /// Name of the destination
    pub name: String,
    /// Docker network name
    pub network: String,
    /// Associated server ID
    pub server_id: Uuid,
    /// UUID for external references
    pub uuid: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl StandaloneDocker {
    pub fn new(name: String, network: String, server_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            network,
            server_id,
            uuid: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create the default coolify destination
    pub fn coolify(server_id: Uuid) -> Self {
        Self::new(
            "coolify".to_string(),
            "coolify".to_string(),
            server_id,
        )
    }

    /// Get the docker network create command
    pub fn network_create_command(&self) -> String {
        format!("docker network create {} --attachable", self.network)
    }

    /// Get the docker network inspect command
    pub fn network_inspect_command(&self) -> String {
        format!("docker network inspect {}", self.network)
    }

    /// Get the docker network remove command
    pub fn network_remove_command(&self) -> String {
        format!("docker network rm {}", self.network)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standalone_docker_new() {
        let server_id = Uuid::new_v4();
        let docker = StandaloneDocker::new(
            "test".to_string(),
            "test-network".to_string(),
            server_id,
        );

        assert_eq!(docker.name, "test");
        assert_eq!(docker.network, "test-network");
        assert_eq!(docker.server_id, server_id);
    }

    #[test]
    fn test_coolify_default() {
        let server_id = Uuid::new_v4();
        let docker = StandaloneDocker::coolify(server_id);

        assert_eq!(docker.name, "coolify");
        assert_eq!(docker.network, "coolify");
    }

    #[test]
    fn test_network_commands() {
        let server_id = Uuid::new_v4();
        let docker = StandaloneDocker::new(
            "test".to_string(),
            "my-network".to_string(),
            server_id,
        );

        assert!(docker.network_create_command().contains("docker network create my-network"));
        assert!(docker.network_inspect_command().contains("docker network inspect my-network"));
        assert!(docker.network_remove_command().contains("docker network rm my-network"));
    }
}
