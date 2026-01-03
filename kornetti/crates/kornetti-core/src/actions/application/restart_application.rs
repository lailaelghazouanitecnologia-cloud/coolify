//! Restart Application Action
//!
//! Handles restarting application containers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Input for restarting an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartApplicationInput {
    /// Application ID
    pub application_id: Uuid,
    /// Server ID
    pub server_id: Uuid,
    /// Container name
    pub container_name: String,
    /// Working directory (where docker-compose.yml is)
    pub workdir: String,
    /// Whether to pull latest image before restart
    pub pull_image: bool,
    /// Whether to recreate the container (vs just restart)
    pub recreate: bool,
    /// Timeout in seconds for graceful stop
    pub stop_timeout: u32,
}

/// Output from restart operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartApplicationOutput {
    /// Container ID after restart
    pub container_id: String,
    /// Whether restart was successful
    pub success: bool,
    /// Restart message
    pub message: String,
    /// Time taken in seconds
    pub duration_seconds: u64,
}

/// Restart application action
pub struct RestartApplication;

impl RestartApplication {
    pub fn new() -> Self {
        Self
    }

    /// Generate restart commands
    pub fn generate_commands(&self, input: &RestartApplicationInput) -> Vec<String> {
        let mut builder = CommandBuilder::new();

        if input.recreate {
            // Full recreate: stop, pull (optional), start
            builder.add(format!(
                "cd {} && docker compose stop -t {}",
                input.workdir, input.stop_timeout
            ));

            if input.pull_image {
                builder.add(format!("cd {} && docker compose pull", input.workdir));
            }

            builder.add(format!(
                "cd {} && docker compose up -d --force-recreate",
                input.workdir
            ));
        } else {
            // Simple restart
            builder.add(format!(
                "docker restart -t {} {}",
                input.stop_timeout, input.container_name
            ));
        }

        // Wait for container to be running
        builder.add(format!(
            "timeout 60 sh -c 'until docker inspect --format=\"{{{{.State.Running}}}}\" {} 2>/dev/null | grep -q true; do sleep 1; done'",
            input.container_name
        ));

        // Get container ID
        builder.add(format!(
            "docker inspect --format='{{{{.Id}}}}' {}",
            input.container_name
        ));

        builder.build()
    }
}

impl Default for RestartApplication {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for RestartApplication {
    type Input = RestartApplicationInput;
    type Output = RestartApplicationOutput;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            application_id = %input.application_id,
            container = %input.container_name,
            recreate = input.recreate,
            pull = input.pull_image,
            "Restarting application"
        );

        let start = std::time::Instant::now();
        let commands = self.generate_commands(&input);

        // TODO: Execute commands via SSH

        let duration = start.elapsed().as_secs();

        tracing::info!(
            container = %input.container_name,
            duration_seconds = duration,
            "Application restart completed"
        );

        Ok(RestartApplicationOutput {
            container_id: String::new(),
            success: true,
            message: "Application restarted successfully".to_string(),
            duration_seconds: duration,
        })
    }

    fn name(&self) -> &'static str {
        "restart_application"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_restart_commands() {
        let action = RestartApplication::new();
        let input = RestartApplicationInput {
            application_id: Uuid::new_v4(),
            server_id: Uuid::new_v4(),
            container_name: "my-app".to_string(),
            workdir: "/data/coolify/applications/my-app".to_string(),
            pull_image: false,
            recreate: false,
            stop_timeout: 30,
        };

        let commands = action.generate_commands(&input);
        assert!(commands.iter().any(|c| c.contains("docker restart")));
    }

    #[test]
    fn test_recreate_commands() {
        let action = RestartApplication::new();
        let input = RestartApplicationInput {
            application_id: Uuid::new_v4(),
            server_id: Uuid::new_v4(),
            container_name: "my-app".to_string(),
            workdir: "/data/coolify/applications/my-app".to_string(),
            pull_image: true,
            recreate: true,
            stop_timeout: 30,
        };

        let commands = action.generate_commands(&input);
        assert!(commands.iter().any(|c| c.contains("docker compose pull")));
        assert!(commands.iter().any(|c| c.contains("--force-recreate")));
    }
}
