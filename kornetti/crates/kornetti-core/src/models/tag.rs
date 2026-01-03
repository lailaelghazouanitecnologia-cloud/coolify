//! Tag model
//!
//! Resource tagging for organization and filtering.
//! Mirrors Coolify's Tag model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tag for organizing resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub team_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: String, team_id: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            team_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Normalize tag name (lowercase, trim)
    pub fn normalize_name(name: &str) -> String {
        name.trim().to_lowercase()
    }
}

/// Resource-tag association (many-to-many pivot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taggable {
    pub tag_id: Uuid,
    pub taggable_id: Uuid,
    pub taggable_type: TaggableType,
}

/// Types of resources that can be tagged
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaggableType {
    Application,
    Service,
    Database,
    Server,
}

impl TaggableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaggableType::Application => "App\\Models\\Application",
            TaggableType::Service => "App\\Models\\Service",
            TaggableType::Database => "App\\Models\\StandaloneDatabase",
            TaggableType::Server => "App\\Models\\Server",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            s if s.contains("Application") => Some(TaggableType::Application),
            s if s.contains("Service") => Some(TaggableType::Service),
            s if s.contains("Database") || s.contains("Standalone") => Some(TaggableType::Database),
            s if s.contains("Server") => Some(TaggableType::Server),
            _ => None,
        }
    }
}

/// Tag with resource counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagWithCounts {
    pub tag: Tag,
    pub application_count: i32,
    pub service_count: i32,
    pub database_count: i32,
    pub server_count: i32,
}

impl TagWithCounts {
    pub fn from_tag(tag: Tag) -> Self {
        Self {
            tag,
            application_count: 0,
            service_count: 0,
            database_count: 0,
            server_count: 0,
        }
    }

    pub fn total_count(&self) -> i32 {
        self.application_count + self.service_count + self.database_count + self.server_count
    }
}
