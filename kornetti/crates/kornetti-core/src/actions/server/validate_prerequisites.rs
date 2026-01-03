//! Validate Prerequisites Action
//!
//! Checks that all required tools are installed on the server.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::Server;
use crate::actions::{Action, ActionError};
use kornetti_ssh::SshClient;

/// Prerequisite check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrerequisitesResult {
    pub all_present: bool,
    pub curl: bool,
    pub wget: bool,
    pub git: bool,
    pub jq: bool,
    pub tar: bool,
    pub gzip: bool,
    pub ssh_client: bool,
    pub rsync: bool,
    pub missing: Vec<String>,
}

impl Default for PrerequisitesResult {
    fn default() -> Self {
        Self {
            all_present: false,
            curl: false,
            wget: false,
            git: false,
            jq: false,
            tar: false,
            gzip: false,
            ssh_client: false,
            rsync: false,
            missing: Vec::new(),
        }
    }
}

/// Input for prerequisites validation
pub struct ValidatePrerequisitesInput<'a> {
    pub server: &'a Server,
}

/// Action to validate server prerequisites
pub struct ValidatePrerequisites {
    ssh: Arc<SshClient>,
}

impl ValidatePrerequisites {
    pub fn new(ssh: Arc<SshClient>) -> Self {
        Self { ssh }
    }

    /// Required tools that must be present
    const REQUIRED: &'static [&'static str] = &["curl", "git", "tar"];

    /// Optional tools that are useful but not required
    const OPTIONAL: &'static [&'static str] = &["wget", "jq", "gzip", "rsync", "ssh"];
}

#[async_trait]
impl Action for ValidatePrerequisites {
    type Input = ValidatePrerequisitesInput<'static>;
    type Output = PrerequisitesResult;

    fn name(&self) -> &'static str {
        "validate_prerequisites"
    }

    #[instrument(skip(self, input), fields(server_id = %input.server.id))]
    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        let server = input.server;

        info!("Checking prerequisites on server {}", server.id);

        let session = self.ssh.connect(server)
            .await
            .map_err(|e| ActionError::ssh_connection_failed(e.to_string()))?;

        let mut result = PrerequisitesResult::default();

        // Check each tool
        let tools = [
            ("curl", &mut result.curl),
            ("wget", &mut result.wget),
            ("git", &mut result.git),
            ("jq", &mut result.jq),
            ("tar", &mut result.tar),
            ("gzip", &mut result.gzip),
            ("ssh", &mut result.ssh_client),
            ("rsync", &mut result.rsync),
        ];

        for (tool, present) in tools {
            let check = session.execute(&format!("command -v {} >/dev/null 2>&1 && echo 'yes'", tool))
                .await
                .ok();

            *present = check
                .map(|r| r.stdout.trim() == "yes")
                .unwrap_or(false);

            if !*present && Self::REQUIRED.contains(&tool) {
                result.missing.push(tool.to_string());
            }
        }

        result.all_present = result.missing.is_empty();

        info!(
            "Prerequisites check complete: all_present={}, missing={:?}",
            result.all_present,
            result.missing
        );

        Ok(result)
    }
}
