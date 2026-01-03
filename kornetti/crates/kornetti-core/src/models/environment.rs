//! Environment model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Environment {
    pub fn new(project_id: Uuid, name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            name,
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a default production environment
    pub fn production(project_id: Uuid) -> Self {
        Self::new(project_id, "production".to_string())
    }

    /// Create a staging environment
    pub fn staging(project_id: Uuid) -> Self {
        Self::new(project_id, "staging".to_string())
    }

    /// Create a development environment
    pub fn development(project_id: Uuid) -> Self {
        Self::new(project_id, "development".to_string())
    }
}
