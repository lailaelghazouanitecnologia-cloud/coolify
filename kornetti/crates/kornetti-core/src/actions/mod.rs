//! Actions Pattern Implementation
//!
//! Actions encapsulate discrete business operations that can be composed
//! to perform complex workflows. Each action is a single unit of work
//! with a clear input/output contract.
//!
//! # Architecture
//!
//! Actions follow these principles:
//! - Single responsibility: each action does one thing well
//! - Composable: actions can call other actions
//! - Async-first: all actions are async for SSH/Docker operations
//! - Error handling: actions return Result types with rich error context
//! - Logging: actions emit structured logs via tracing
//!
//! # Example
//!
//! ```rust,ignore
//! use kornetti_core::actions::server::InstallDocker;
//!
//! let action = InstallDocker::new(ssh_client);
//! let result = action.handle(&server).await?;
//! ```

pub mod server;
pub mod database;
pub mod proxy;
pub mod application;
pub mod service;
pub mod swarm;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, instrument, warn};

/// Action result with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Duration of the action
    pub duration_ms: u64,
    /// Optional output data
    pub output: Option<serde_json::Value>,
    /// Commands that were executed (for debugging)
    pub commands_executed: Vec<String>,
}

impl ActionResult {
    /// Create a successful result
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            duration_ms: 0,
            output: None,
            commands_executed: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            duration_ms: 0,
            output: None,
            commands_executed: Vec::new(),
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }

    /// Set output data
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Add executed commands
    pub fn with_commands(mut self, commands: Vec<String>) -> Self {
        self.commands_executed = commands;
        self
    }
}

/// Action error with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionError {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Whether the action can be retried
    pub retryable: bool,
    /// Underlying cause if any
    pub cause: Option<String>,
    /// Commands that were executed before failure
    pub commands_executed: Vec<String>,
}

impl ActionError {
    /// Create a new action error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable: false,
            cause: None,
            commands_executed: Vec::new(),
        }
    }

    /// Mark as retryable
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }

    /// Add cause
    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause = Some(cause.into());
        self
    }

    /// Common errors
    pub fn ssh_connection_failed(cause: impl Into<String>) -> Self {
        Self::new("SSH_CONNECTION_FAILED", "Failed to connect to server via SSH")
            .retryable()
            .with_cause(cause)
    }

    pub fn docker_not_installed() -> Self {
        Self::new("DOCKER_NOT_INSTALLED", "Docker is not installed on the server")
    }

    pub fn docker_not_running() -> Self {
        Self::new("DOCKER_NOT_RUNNING", "Docker daemon is not running")
            .retryable()
    }

    pub fn unsupported_os(os: impl Into<String>) -> Self {
        Self::new("UNSUPPORTED_OS", format!("Unsupported operating system: {}", os.into()))
    }

    pub fn command_failed(cmd: impl Into<String>, error: impl Into<String>) -> Self {
        Self::new("COMMAND_FAILED", format!("Command failed: {}", cmd.into()))
            .with_cause(error)
    }

    pub fn container_not_found(name: impl Into<String>) -> Self {
        Self::new("CONTAINER_NOT_FOUND", format!("Container not found: {}", name.into()))
    }

    pub fn configuration_error(msg: impl Into<String>) -> Self {
        Self::new("CONFIGURATION_ERROR", msg)
    }
}

impl std::fmt::Display for ActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ActionError {}

/// Trait for all actions
#[async_trait]
pub trait Action: Send + Sync {
    /// Input type for the action
    type Input: Send + Sync;
    /// Output type for the action
    type Output: Send + Sync;

    /// Execute the action
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError>;

    /// Action name for logging
    fn name(&self) -> &'static str;
}

/// Action context for shared resources
#[derive(Clone)]
pub struct ActionContext {
    /// Request ID for tracing
    pub request_id: String,
    /// Team ID for multi-tenancy
    pub team_id: Option<i64>,
    /// User ID for audit
    pub user_id: Option<i64>,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            team_id: None,
            user_id: None,
        }
    }
}

/// Command builder for remote execution
#[derive(Debug, Clone, Default)]
pub struct CommandBuilder {
    commands: Vec<String>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a command
    pub fn add(&mut self, cmd: impl Into<String>) -> &mut Self {
        self.commands.push(cmd.into());
        self
    }

    /// Add an echo command for logging
    pub fn echo(&mut self, msg: impl Into<String>) -> &mut Self {
        self.commands.push(format!("echo '{}'", msg.into()));
        self
    }

    /// Add mkdir -p command
    pub fn mkdir(&mut self, path: impl Into<String>) -> &mut Self {
        self.commands.push(format!("mkdir -p {}", path.into()));
        self
    }

    /// Add a conditional command
    pub fn add_if(&mut self, condition: bool, cmd: impl Into<String>) -> &mut Self {
        if condition {
            self.commands.push(cmd.into());
        }
        self
    }

    /// Add multiple commands
    pub fn extend(&mut self, cmds: impl IntoIterator<Item = impl Into<String>>) -> &mut Self {
        for cmd in cmds {
            self.commands.push(cmd.into());
        }
        self
    }

    /// Build the command list
    pub fn build(self) -> Vec<String> {
        self.commands
    }

    /// Get current commands
    pub fn commands(&self) -> &[String] {
        &self.commands
    }
}

// Re-exports for convenience
pub use server::{
    InstallDocker, ValidateServer, CleanupDocker, ValidatePrerequisites,
    InstallPrerequisites, RunCommand, CheckUpdates,
};
pub use database::{
    StartPostgresql, StartMysql, StartRedis, StartMongodb,
    StartMariadb, StartClickhouse, StopDatabase, RestartDatabase,
};
pub use proxy::{
    StartProxy, StopProxy, SaveProxyConfiguration, GetProxyConfiguration, CheckProxy,
};
pub use application::{
    StopApplication, GenerateConfig, LoadComposeFile,
    NixpacksBuild, NixpacksBuildInput, NixpacksBuildOutput, NixpacksAppType,
    DockerfileGenerate, DockerfileGenerateInput, DockerfileGenerateOutput, BuildPackType,
};
pub use service::{
    StartService, StopService, RestartService, DeleteService,
};
pub use swarm::{
    InitSwarm, InitSwarmInput, InitSwarmOutput,
    JoinSwarm, JoinSwarmInput, JoinSwarmOutput,
    DeploySwarmService, DeploySwarmServiceInput, DeploySwarmServiceOutput,
    ListSwarmNodes, SwarmNode, SwarmNodeStatus, SwarmNodeRole,
    PromoteNode, DemoteNode, DrainNode, RemoveNode,
    PlacementConstraint, UpdateConfig, RollbackConfig, SwarmNetwork,
};
