//! Local File Volume Model
//!
//! Represents file mounts (single files) for containers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Resource type that can have file volumes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileVolumeResourceType {
    Application,
    Service,
    StandalonePostgresql,
    StandaloneMysql,
    StandaloneMariadb,
    StandaloneMongodb,
    StandaloneRedis,
    StandaloneKeydb,
    StandaloneDragonfly,
    StandaloneClickhouse,
}

impl FileVolumeResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileVolumeResourceType::Application => "application",
            FileVolumeResourceType::Service => "service",
            FileVolumeResourceType::StandalonePostgresql => "standalone_postgresql",
            FileVolumeResourceType::StandaloneMysql => "standalone_mysql",
            FileVolumeResourceType::StandaloneMariadb => "standalone_mariadb",
            FileVolumeResourceType::StandaloneMongodb => "standalone_mongodb",
            FileVolumeResourceType::StandaloneRedis => "standalone_redis",
            FileVolumeResourceType::StandaloneKeydb => "standalone_keydb",
            FileVolumeResourceType::StandaloneDragonfly => "standalone_dragonfly",
            FileVolumeResourceType::StandaloneClickhouse => "standalone_clickhouse",
        }
    }
}

/// Local file volume (single file mount)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileVolume {
    pub id: Uuid,
    /// File content
    pub content: String,
    /// Original filename
    pub filename: String,
    /// Mount path inside container
    pub mount_path: String,
    /// Whether this is a directory (false = file)
    pub is_directory: bool,
    /// Resource type
    pub resource_type: FileVolumeResourceType,
    /// Resource ID
    pub resource_id: Uuid,
    /// Chmod permissions (e.g., "644", "755")
    pub chmod: Option<String>,
    /// Chown user:group
    pub chown: Option<String>,
    /// Whether to base64 encode the content
    pub is_based64: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LocalFileVolume {
    pub fn new(
        content: String,
        filename: String,
        mount_path: String,
        resource_type: FileVolumeResourceType,
        resource_id: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content,
            filename,
            mount_path,
            is_directory: false,
            resource_type,
            resource_id,
            chmod: None,
            chown: None,
            is_based64: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the full mount path including filename
    pub fn full_mount_path(&self) -> String {
        if self.mount_path.ends_with('/') {
            format!("{}{}", self.mount_path, self.filename)
        } else {
            format!("{}/{}", self.mount_path, self.filename)
        }
    }

    /// Get the host path for this file
    pub fn host_path(&self, base_path: &str) -> String {
        format!("{}/files/{}", base_path, self.id)
    }

    /// Generate docker volume mount argument
    pub fn docker_mount_arg(&self, base_path: &str) -> String {
        format!("{}:{}:ro", self.host_path(base_path), self.full_mount_path())
    }

    /// Get content, decoding from base64 if needed
    pub fn get_content(&self) -> Result<String, String> {
        if self.is_based64 {
            use base64::Engine;
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(&self.content)
                .map_err(|e| format!("Base64 decode error: {}", e))?;
            String::from_utf8(decoded).map_err(|e| format!("UTF-8 decode error: {}", e))
        } else {
            Ok(self.content.clone())
        }
    }

    /// Set content, encoding to base64 if needed
    pub fn set_content(&mut self, content: String, encode_base64: bool) {
        if encode_base64 {
            use base64::Engine;
            self.content = base64::engine::general_purpose::STANDARD.encode(content.as_bytes());
            self.is_based64 = true;
        } else {
            self.content = content;
            self.is_based64 = false;
        }
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_file_volume_new() {
        let resource_id = Uuid::new_v4();
        let volume = LocalFileVolume::new(
            "file content".to_string(),
            "config.yaml".to_string(),
            "/app/config".to_string(),
            FileVolumeResourceType::Application,
            resource_id,
        );

        assert_eq!(volume.filename, "config.yaml");
        assert_eq!(volume.mount_path, "/app/config");
        assert!(!volume.is_directory);
    }

    #[test]
    fn test_full_mount_path() {
        let resource_id = Uuid::new_v4();
        let volume = LocalFileVolume::new(
            "content".to_string(),
            "file.txt".to_string(),
            "/data".to_string(),
            FileVolumeResourceType::Application,
            resource_id,
        );

        assert_eq!(volume.full_mount_path(), "/data/file.txt");
    }

    #[test]
    fn test_base64_content() {
        let resource_id = Uuid::new_v4();
        let mut volume = LocalFileVolume::new(
            "".to_string(),
            "file.txt".to_string(),
            "/data".to_string(),
            FileVolumeResourceType::Application,
            resource_id,
        );

        volume.set_content("Hello World".to_string(), true);
        assert!(volume.is_based64);

        let decoded = volume.get_content().unwrap();
        assert_eq!(decoded, "Hello World");
    }
}
