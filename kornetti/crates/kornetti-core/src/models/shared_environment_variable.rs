//! Shared Environment Variable Model
//!
//! Team-wide environment variables that can be used across resources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Shared environment variable across team resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedEnvironmentVariable {
    pub id: Uuid,
    /// UUID for external references
    pub uuid: String,
    /// Variable key/name
    pub key: String,
    /// Variable value (may be encrypted)
    pub value: String,
    /// Whether value is a secret (encrypted)
    pub is_secret: bool,
    /// Whether value should be shown in UI
    pub is_shown_once: bool,
    /// Team that owns this variable
    pub team_id: Uuid,
    /// Optional project scope
    pub project_id: Option<Uuid>,
    /// Optional environment scope
    pub environment_id: Option<Uuid>,
    /// Resource types this applies to
    pub resource_types: Vec<SharedVarResourceType>,
    /// Description
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SharedVarResourceType {
    Application,
    Service,
    Database,
    All,
}

impl SharedEnvironmentVariable {
    pub fn new(key: String, value: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            uuid: Uuid::new_v4().to_string(),
            key,
            value,
            is_secret: false,
            is_shown_once: false,
            team_id,
            project_id: None,
            environment_id: None,
            resource_types: vec![SharedVarResourceType::All],
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a secret variable
    pub fn secret(key: String, value: String, team_id: Uuid) -> Self {
        let mut var = Self::new(key, value, team_id);
        var.is_secret = true;
        var
    }

    /// Check if this variable applies to a given resource type
    pub fn applies_to(&self, resource_type: SharedVarResourceType) -> bool {
        self.resource_types.contains(&SharedVarResourceType::All)
            || self.resource_types.contains(&resource_type)
    }

    /// Check if this variable applies to a given project
    pub fn applies_to_project(&self, project_id: Uuid) -> bool {
        self.project_id.map_or(true, |p| p == project_id)
    }

    /// Check if this variable applies to a given environment
    pub fn applies_to_environment(&self, environment_id: Uuid) -> bool {
        self.environment_id.map_or(true, |e| e == environment_id)
    }

    /// Validate the key format
    pub fn validate_key(key: &str) -> bool {
        if key.is_empty() || key.len() > 255 {
            return false;
        }

        // Must start with letter or underscore
        let first = key.chars().next().unwrap();
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }

        // Rest must be alphanumeric or underscore
        key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    /// Format as shell export
    pub fn as_export(&self) -> String {
        let escaped = self.value.replace('\'', "'\\''");
        format!("export {}='{}'", self.key, escaped)
    }

    /// Format as docker env argument
    pub fn as_docker_env(&self) -> String {
        format!("{}={}", self.key, self.value)
    }
}

/// Collection of shared variables with filtering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharedVariableSet {
    pub variables: Vec<SharedEnvironmentVariable>,
}

impl SharedVariableSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, var: SharedEnvironmentVariable) {
        self.variables.push(var);
    }

    /// Filter by resource type
    pub fn for_resource_type(&self, rt: SharedVarResourceType) -> Vec<&SharedEnvironmentVariable> {
        self.variables.iter().filter(|v| v.applies_to(rt)).collect()
    }

    /// Filter by project
    pub fn for_project(&self, project_id: Uuid) -> Vec<&SharedEnvironmentVariable> {
        self.variables
            .iter()
            .filter(|v| v.applies_to_project(project_id))
            .collect()
    }

    /// Filter by environment
    pub fn for_environment(&self, env_id: Uuid) -> Vec<&SharedEnvironmentVariable> {
        self.variables
            .iter()
            .filter(|v| v.applies_to_environment(env_id))
            .collect()
    }

    /// Get all as docker env format
    pub fn as_docker_envs(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.as_docker_env()).collect()
    }

    /// Merge with another set (other takes precedence)
    pub fn merge(&mut self, other: &SharedVariableSet) {
        for var in &other.variables {
            // Remove existing with same key
            self.variables.retain(|v| v.key != var.key);
            self.variables.push(var.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_variable_new() {
        let team_id = Uuid::new_v4();
        let var = SharedEnvironmentVariable::new(
            "DATABASE_URL".to_string(),
            "postgres://...".to_string(),
            team_id,
        );

        assert_eq!(var.key, "DATABASE_URL");
        assert!(!var.is_secret);
    }

    #[test]
    fn test_validate_key() {
        assert!(SharedEnvironmentVariable::validate_key("DATABASE_URL"));
        assert!(SharedEnvironmentVariable::validate_key("_PRIVATE"));
        assert!(SharedEnvironmentVariable::validate_key("VAR123"));
        assert!(!SharedEnvironmentVariable::validate_key("123VAR"));
        assert!(!SharedEnvironmentVariable::validate_key(""));
        assert!(!SharedEnvironmentVariable::validate_key("VAR-NAME"));
    }

    #[test]
    fn test_applies_to() {
        let team_id = Uuid::new_v4();
        let var = SharedEnvironmentVariable::new("KEY".to_string(), "value".to_string(), team_id);

        assert!(var.applies_to(SharedVarResourceType::Application));
        assert!(var.applies_to(SharedVarResourceType::Database));
    }

    #[test]
    fn test_as_export() {
        let team_id = Uuid::new_v4();
        let var = SharedEnvironmentVariable::new(
            "MSG".to_string(),
            "Hello 'World'".to_string(),
            team_id,
        );

        let export = var.as_export();
        assert!(export.starts_with("export MSG="));
        assert!(export.contains("Hello"));
    }

    #[test]
    fn test_variable_set_merge() {
        let team_id = Uuid::new_v4();
        let mut set1 = SharedVariableSet::new();
        set1.add(SharedEnvironmentVariable::new("A".to_string(), "1".to_string(), team_id));
        set1.add(SharedEnvironmentVariable::new("B".to_string(), "2".to_string(), team_id));

        let mut set2 = SharedVariableSet::new();
        set2.add(SharedEnvironmentVariable::new("B".to_string(), "3".to_string(), team_id));
        set2.add(SharedEnvironmentVariable::new("C".to_string(), "4".to_string(), team_id));

        set1.merge(&set2);

        assert_eq!(set1.variables.len(), 3);
        let b = set1.variables.iter().find(|v| v.key == "B").unwrap();
        assert_eq!(b.value, "3"); // set2 value takes precedence
    }
}
