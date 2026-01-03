//! Environment variable models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Environment variable for applications/services/databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentVariable {
    pub id: Uuid,
    /// The resource this variable belongs to
    pub resource_id: Uuid,
    pub resource_type: ResourceType,
    pub key: String,
    pub value: String,
    /// Whether to show in build logs
    pub is_build_time: bool,
    /// Whether the value is a secret (masked in UI)
    pub is_secret: bool,
    /// Whether this is a multiline value
    pub is_multiline: bool,
    /// Whether this variable is shown in UI
    pub is_shown_once: bool,
    /// Order for display
    pub order: i32,
    /// Real-time update (for Livewire in legacy)
    pub is_preview: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Type of resource the variable belongs to
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Application,
    Service,
    Database,
    SharedVariable,
}

impl EnvironmentVariable {
    pub fn new(resource_id: Uuid, resource_type: ResourceType, key: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            resource_id,
            resource_type,
            key,
            value,
            is_build_time: false,
            is_secret: false,
            is_multiline: false,
            is_shown_once: false,
            order: 0,
            is_preview: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a secret environment variable
    pub fn secret(resource_id: Uuid, resource_type: ResourceType, key: String, value: String) -> Self {
        let mut var = Self::new(resource_id, resource_type, key, value);
        var.is_secret = true;
        var
    }

    /// Create a build-time environment variable
    pub fn build_time(resource_id: Uuid, resource_type: ResourceType, key: String, value: String) -> Self {
        let mut var = Self::new(resource_id, resource_type, key, value);
        var.is_build_time = true;
        var
    }

    /// Format for docker-compose
    pub fn to_compose_format(&self) -> String {
        if self.is_multiline {
            format!("{}={}", self.key, self.value.replace('\n', "\\n"))
        } else {
            format!("{}={}", self.key, self.value)
        }
    }

    /// Format for .env file
    pub fn to_env_format(&self) -> String {
        if self.is_multiline {
            format!("{}=\"{}\"", self.key, self.value.replace('"', "\\\""))
        } else if self.value.contains(' ') || self.value.contains('$') {
            format!("{}=\"{}\"", self.key, self.value)
        } else {
            format!("{}={}", self.key, self.value)
        }
    }
}

/// Shared environment variable across multiple resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedEnvironmentVariable {
    pub id: Uuid,
    pub team_id: Uuid,
    /// Optional project scope (None = team-wide)
    pub project_id: Option<Uuid>,
    /// Optional environment scope (None = all environments)
    pub environment_id: Option<Uuid>,
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SharedEnvironmentVariable {
    pub fn team_wide(team_id: Uuid, key: String, value: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            team_id,
            project_id: None,
            environment_id: None,
            key,
            value,
            is_secret: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn for_project(team_id: Uuid, project_id: Uuid, key: String, value: String) -> Self {
        let mut var = Self::team_wide(team_id, key, value);
        var.project_id = Some(project_id);
        var
    }

    pub fn for_environment(team_id: Uuid, project_id: Uuid, environment_id: Uuid, key: String, value: String) -> Self {
        let mut var = Self::for_project(team_id, project_id, key, value);
        var.environment_id = Some(environment_id);
        var
    }
}
