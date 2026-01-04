//! Swarm Node Management Actions
//!
//! Actions for managing nodes in a Docker Swarm cluster.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actions::{Action, ActionError, ActionResult, CommandBuilder};

/// Swarm node status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeStatus {
    Ready,
    Down,
    Unknown,
    Disconnected,
}

impl From<&str> for SwarmNodeStatus {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ready" => SwarmNodeStatus::Ready,
            "down" => SwarmNodeStatus::Down,
            "disconnected" => SwarmNodeStatus::Disconnected,
            _ => SwarmNodeStatus::Unknown,
        }
    }
}

/// Swarm node role
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeRole {
    Manager,
    Worker,
}

impl From<&str> for SwarmNodeRole {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "manager" => SwarmNodeRole::Manager,
            _ => SwarmNodeRole::Worker,
        }
    }
}

/// Swarm node availability
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SwarmNodeAvailability {
    Active,
    Pause,
    Drain,
}

impl From<&str> for SwarmNodeAvailability {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => SwarmNodeAvailability::Active,
            "pause" => SwarmNodeAvailability::Pause,
            "drain" => SwarmNodeAvailability::Drain,
            _ => SwarmNodeAvailability::Active,
        }
    }
}

/// Swarm node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmNode {
    /// Node ID
    pub id: String,
    /// Hostname
    pub hostname: String,
    /// Status
    pub status: SwarmNodeStatus,
    /// Role
    pub role: SwarmNodeRole,
    /// Availability
    pub availability: SwarmNodeAvailability,
    /// Manager status (for manager nodes)
    pub manager_status: Option<ManagerStatus>,
    /// Engine version
    pub engine_version: String,
    /// IP address
    pub addr: String,
    /// Labels
    pub labels: std::collections::HashMap<String, String>,
}

/// Manager-specific status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerStatus {
    /// Is leader
    pub leader: bool,
    /// Reachability
    pub reachability: String,
    /// Manager address
    pub addr: String,
}

/// List Swarm Nodes action
pub struct ListSwarmNodes;

impl ListSwarmNodes {
    pub fn new() -> Self {
        Self
    }

    /// Generate command to list nodes
    pub fn list_command() -> String {
        "docker node ls --format '{{json .}}'".to_string()
    }

    /// Generate command to inspect a node
    pub fn inspect_command(node_id: &str) -> String {
        format!("docker node inspect {} --format '{{{{json .}}}}'", node_id)
    }
}

impl Default for ListSwarmNodes {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for ListSwarmNodes {
    type Input = ();
    type Output = Vec<SwarmNode>;

    async fn handle(&self, _input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!("Listing Swarm nodes");

        // Placeholder: would execute docker node ls and parse output
        Ok(vec![])
    }

    fn name(&self) -> &'static str {
        "list_swarm_nodes"
    }
}

/// Input for promoting a node to manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteNodeInput {
    /// Node ID to promote
    pub node_id: String,
}

/// Promote Node action
pub struct PromoteNode;

impl PromoteNode {
    pub fn new() -> Self {
        Self
    }

    /// Generate promote command
    pub fn promote_command(node_id: &str) -> String {
        format!("docker node promote {}", node_id)
    }
}

impl Default for PromoteNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for PromoteNode {
    type Input = PromoteNodeInput;
    type Output = ActionResult;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(node_id = %input.node_id, "Promoting node to manager");

        let cmd = Self::promote_command(&input.node_id);

        Ok(ActionResult::success(format!(
            "Node {} promoted to manager",
            input.node_id
        )).with_commands(vec![cmd]))
    }

    fn name(&self) -> &'static str {
        "promote_node"
    }
}

/// Input for demoting a manager to worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoteNodeInput {
    /// Node ID to demote
    pub node_id: String,
}

/// Demote Node action
pub struct DemoteNode;

impl DemoteNode {
    pub fn new() -> Self {
        Self
    }

    /// Generate demote command
    pub fn demote_command(node_id: &str) -> String {
        format!("docker node demote {}", node_id)
    }
}

impl Default for DemoteNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for DemoteNode {
    type Input = DemoteNodeInput;
    type Output = ActionResult;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(node_id = %input.node_id, "Demoting node to worker");

        let cmd = Self::demote_command(&input.node_id);

        Ok(ActionResult::success(format!(
            "Node {} demoted to worker",
            input.node_id
        )).with_commands(vec![cmd]))
    }

    fn name(&self) -> &'static str {
        "demote_node"
    }
}

/// Input for draining a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrainNodeInput {
    /// Node ID to drain
    pub node_id: String,
}

/// Drain Node action (stops scheduling new tasks)
pub struct DrainNode;

impl DrainNode {
    pub fn new() -> Self {
        Self
    }

    /// Generate drain command
    pub fn drain_command(node_id: &str) -> String {
        format!("docker node update --availability drain {}", node_id)
    }

    /// Generate activate command
    pub fn activate_command(node_id: &str) -> String {
        format!("docker node update --availability active {}", node_id)
    }

    /// Generate pause command
    pub fn pause_command(node_id: &str) -> String {
        format!("docker node update --availability pause {}", node_id)
    }
}

impl Default for DrainNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for DrainNode {
    type Input = DrainNodeInput;
    type Output = ActionResult;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(node_id = %input.node_id, "Draining node");

        let cmd = Self::drain_command(&input.node_id);

        Ok(ActionResult::success(format!(
            "Node {} set to drain",
            input.node_id
        )).with_commands(vec![cmd]))
    }

    fn name(&self) -> &'static str {
        "drain_node"
    }
}

/// Input for removing a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveNodeInput {
    /// Node ID to remove
    pub node_id: String,
    /// Force removal
    pub force: bool,
}

/// Remove Node action
pub struct RemoveNode;

impl RemoveNode {
    pub fn new() -> Self {
        Self
    }

    /// Generate remove command
    pub fn remove_command(node_id: &str, force: bool) -> String {
        if force {
            format!("docker node rm --force {}", node_id)
        } else {
            format!("docker node rm {}", node_id)
        }
    }
}

impl Default for RemoveNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for RemoveNode {
    type Input = RemoveNodeInput;
    type Output = ActionResult;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            node_id = %input.node_id,
            force = input.force,
            "Removing node from swarm"
        );

        let cmd = Self::remove_command(&input.node_id, input.force);

        Ok(ActionResult::success(format!(
            "Node {} removed from swarm",
            input.node_id
        )).with_commands(vec![cmd]))
    }

    fn name(&self) -> &'static str {
        "remove_node"
    }
}

/// Input for updating node labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNodeLabelsInput {
    /// Node ID
    pub node_id: String,
    /// Labels to add
    pub labels_add: std::collections::HashMap<String, String>,
    /// Labels to remove
    pub labels_rm: Vec<String>,
}

/// Update Node Labels action
pub struct UpdateNodeLabels;

impl UpdateNodeLabels {
    pub fn new() -> Self {
        Self
    }

    /// Generate label update command
    pub fn update_command(input: &UpdateNodeLabelsInput) -> String {
        let mut cmd = format!("docker node update");

        for (key, value) in &input.labels_add {
            cmd.push_str(&format!(" --label-add {}={}", key, value));
        }

        for key in &input.labels_rm {
            cmd.push_str(&format!(" --label-rm {}", key));
        }

        cmd.push_str(&format!(" {}", input.node_id));
        cmd
    }
}

impl Default for UpdateNodeLabels {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Action for UpdateNodeLabels {
    type Input = UpdateNodeLabelsInput;
    type Output = ActionResult;

    async fn handle(&self, input: Self::Input) -> Result<Self::Output, ActionError> {
        tracing::info!(
            node_id = %input.node_id,
            labels_add = ?input.labels_add,
            labels_rm = ?input.labels_rm,
            "Updating node labels"
        );

        let cmd = Self::update_command(&input);

        Ok(ActionResult::success(format!(
            "Node {} labels updated",
            input.node_id
        )).with_commands(vec![cmd]))
    }

    fn name(&self) -> &'static str {
        "update_node_labels"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promote_command() {
        let cmd = PromoteNode::promote_command("abc123");
        assert_eq!(cmd, "docker node promote abc123");
    }

    #[test]
    fn test_drain_command() {
        let cmd = DrainNode::drain_command("abc123");
        assert!(cmd.contains("--availability drain"));
    }

    #[test]
    fn test_remove_command_force() {
        let cmd = RemoveNode::remove_command("abc123", true);
        assert!(cmd.contains("--force"));
    }

    #[test]
    fn test_update_labels_command() {
        let input = UpdateNodeLabelsInput {
            node_id: "abc123".to_string(),
            labels_add: std::collections::HashMap::from([
                ("env".to_string(), "production".to_string()),
            ]),
            labels_rm: vec!["old-label".to_string()],
        };

        let cmd = UpdateNodeLabels::update_command(&input);
        assert!(cmd.contains("--label-add env=production"));
        assert!(cmd.contains("--label-rm old-label"));
    }
}
