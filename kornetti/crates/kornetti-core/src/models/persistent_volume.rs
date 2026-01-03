//! Persistent Volume models
//!
//! Docker volumes and file mounts for applications/databases/services.
//! Mirrors Coolify's LocalPersistentVolume and LocalFileVolume models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of resource the volume belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VolumeResourceType {
    Application,
    Database,
    Service,
}

/// Persistent Docker volume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPersistentVolume {
    pub id: Uuid,
    /// Volume name in Docker
    pub name: String,
    /// Mount path inside container
    pub mount_path: String,
    /// Host path (if using bind mount instead of volume)
    pub host_path: Option<String>,
    /// Resource type
    pub resource_type: VolumeResourceType,
    /// Resource ID (application, database, or service)
    pub resource_id: Uuid,
    /// Whether this is the main data volume
    pub is_primary: bool,
    /// Whether to include in backups
    pub include_in_backups: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LocalPersistentVolume {
    pub fn new(
        name: String,
        mount_path: String,
        resource_type: VolumeResourceType,
        resource_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            mount_path,
            host_path: None,
            resource_type,
            resource_id,
            is_primary: false,
            include_in_backups: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a bind mount volume
    pub fn bind_mount(
        host_path: String,
        mount_path: String,
        resource_type: VolumeResourceType,
        resource_id: Uuid,
    ) -> Self {
        let mut vol = Self::new(
            format!("bind-{}", Uuid::new_v4()),
            mount_path,
            resource_type,
            resource_id,
        );
        vol.host_path = Some(host_path);
        vol
    }

    /// Generate Docker volume string for docker-compose
    pub fn to_compose_volume(&self) -> String {
        if let Some(ref host_path) = self.host_path {
            format!("{}:{}", host_path, self.mount_path)
        } else {
            format!("{}:{}", self.name, self.mount_path)
        }
    }

    /// Check if this is a bind mount
    pub fn is_bind_mount(&self) -> bool {
        self.host_path.is_some()
    }
}

/// File mount (single file mounted into container)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileVolume {
    pub id: Uuid,
    /// File system path for the file
    pub fs_path: String,
    /// Mount path inside container
    pub mount_path: String,
    /// File content
    pub content: String,
    /// Whether file is executable
    pub is_executable: bool,
    /// Whether file is a directory
    pub is_directory: bool,
    /// Resource type
    pub resource_type: VolumeResourceType,
    /// Resource ID
    pub resource_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LocalFileVolume {
    pub fn new(
        fs_path: String,
        mount_path: String,
        content: String,
        resource_type: VolumeResourceType,
        resource_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            fs_path,
            mount_path,
            content,
            is_executable: false,
            is_directory: false,
            resource_type,
            resource_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a config file mount
    pub fn config_file(
        filename: &str,
        mount_path: String,
        content: String,
        resource_type: VolumeResourceType,
        resource_id: Uuid,
    ) -> Self {
        Self::new(
            format!("/data/coolify/configs/{}/{}", resource_id, filename),
            mount_path,
            content,
            resource_type,
            resource_id,
        )
    }

    /// Generate Docker volume string
    pub fn to_compose_volume(&self) -> String {
        let mut mount = format!("{}:{}", self.fs_path, self.mount_path);
        if !self.is_directory {
            mount.push_str(":ro"); // Single files are typically read-only
        }
        mount
    }
}

/// Summary of all volumes for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeSummary {
    pub persistent_volumes: Vec<LocalPersistentVolume>,
    pub file_volumes: Vec<LocalFileVolume>,
}

impl VolumeSummary {
    pub fn new() -> Self {
        Self {
            persistent_volumes: Vec::new(),
            file_volumes: Vec::new(),
        }
    }

    /// Get total number of volumes
    pub fn total_count(&self) -> usize {
        self.persistent_volumes.len() + self.file_volumes.len()
    }

    /// Generate all volume strings for docker-compose
    pub fn to_compose_volumes(&self) -> Vec<String> {
        let mut volumes = Vec::new();
        for pv in &self.persistent_volumes {
            volumes.push(pv.to_compose_volume());
        }
        for fv in &self.file_volumes {
            volumes.push(fv.to_compose_volume());
        }
        volumes
    }
}

impl Default for VolumeSummary {
    fn default() -> Self {
        Self::new()
    }
}
