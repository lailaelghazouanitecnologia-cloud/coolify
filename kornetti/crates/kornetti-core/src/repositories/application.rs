//! Application repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{
    Application, ApplicationStatus, ApplicationSource,
    BuildConfig, DeployConfig, BuildPack, HealthCheck,
    ResourceLimits, PortMapping, VolumeMapping, EnvironmentVariable,
};

#[derive(FromRow)]
struct ApplicationRow {
    id: Uuid,
    environment_id: Uuid,
    server_id: Uuid,
    name: String,
    description: Option<String>,
    fqdn: Option<String>,
    // Git
    git_repository: Option<String>,
    git_branch: Option<String>,
    git_commit_sha: Option<String>,
    private_key_id: Option<Uuid>,
    // Docker
    docker_image: Option<String>,
    // Build
    build_pack: String,
    dockerfile_path: Option<String>,
    dockerfile_content: Option<String>,
    docker_compose_content: Option<String>,
    build_command: Option<String>,
    install_command: Option<String>,
    start_command: Option<String>,
    base_directory: Option<String>,
    publish_directory: Option<String>,
    // Deploy
    status: String,
    replicas: i32,
    health_check_enabled: bool,
    health_check_path: Option<String>,
    health_check_port: Option<i32>,
    health_check_interval: Option<i32>,
    health_check_timeout: Option<i32>,
    health_check_retries: Option<i32>,
    health_check_start_period: Option<i32>,
    // Limits
    limits_memory: Option<String>,
    limits_memory_reservation: Option<String>,
    limits_cpus: Option<String>,
    // Ports
    ports_mappings: serde_json::Value,
    // Settings
    settings: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ApplicationRow> for Application {
    fn from(row: ApplicationRow) -> Self {
        let source = if let Some(repo) = row.git_repository {
            ApplicationSource::Git {
                repository_url: repo,
                branch: row.git_branch.unwrap_or_else(|| "main".to_string()),
                commit_sha: row.git_commit_sha,
                private_key_id: row.private_key_id,
            }
        } else if let Some(image) = row.docker_image {
            ApplicationSource::DockerImage {
                image: image.clone(),
                tag: "latest".to_string(),
                registry_id: None,
            }
        } else if let Some(content) = row.docker_compose_content {
            ApplicationSource::DockerCompose { content }
        } else if let Some(content) = row.dockerfile_content {
            ApplicationSource::Dockerfile {
                content,
                context: row.base_directory.clone().unwrap_or_else(|| ".".to_string()),
            }
        } else {
            ApplicationSource::Git {
                repository_url: String::new(),
                branch: "main".to_string(),
                commit_sha: None,
                private_key_id: None,
            }
        };

        let build_config = BuildConfig {
            build_pack: parse_build_pack(&row.build_pack),
            dockerfile_path: row.dockerfile_path,
            build_command: row.build_command,
            install_command: row.install_command,
            start_command: row.start_command,
            base_directory: row.base_directory,
            publish_directory: row.publish_directory,
        };

        let health_check = if row.health_check_enabled {
            Some(HealthCheck {
                path: row.health_check_path.unwrap_or_else(|| "/".to_string()),
                port: row.health_check_port.unwrap_or(80) as u16,
                interval: row.health_check_interval.unwrap_or(30) as u32,
                timeout: row.health_check_timeout.unwrap_or(5) as u32,
                retries: row.health_check_retries.unwrap_or(3) as u32,
                start_period: row.health_check_start_period.unwrap_or(30) as u32,
            })
        } else {
            None
        };

        let ports: Vec<PortMapping> = serde_json::from_value(row.ports_mappings)
            .unwrap_or_default();

        let deploy_config = DeployConfig {
            replicas: row.replicas as u32,
            health_check,
            resources: ResourceLimits {
                memory_limit: row.limits_memory,
                memory_reservation: row.limits_memory_reservation,
                cpu_limit: row.limits_cpus.and_then(|s| s.parse().ok()),
                cpu_reservation: None,
            },
            ports,
            volumes: vec![],
            environment_variables: vec![],
            labels: vec![],
        };

        Application {
            id: row.id,
            environment_id: row.environment_id,
            server_id: row.server_id,
            name: row.name,
            description: row.description,
            fqdn: row.fqdn,
            source,
            build_config,
            deploy_config,
            status: parse_application_status(&row.status),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn parse_build_pack(s: &str) -> BuildPack {
    match s {
        "nixpacks" => BuildPack::Nixpacks,
        "dockerfile" => BuildPack::Dockerfile,
        "docker_image" | "dockerimage" => BuildPack::DockerImage,
        "docker_compose" | "dockercompose" => BuildPack::DockerCompose,
        "static" => BuildPack::Static,
        _ => BuildPack::Nixpacks,
    }
}

fn parse_application_status(s: &str) -> ApplicationStatus {
    match s {
        "stopped" => ApplicationStatus::Stopped,
        "starting" => ApplicationStatus::Starting,
        "running" => ApplicationStatus::Running,
        "stopping" => ApplicationStatus::Stopping,
        "restarting" => ApplicationStatus::Restarting,
        "degraded" => ApplicationStatus::Degraded,
        _ => ApplicationStatus::Error,
    }
}

pub struct ApplicationRepository {
    pool: PgPool,
}

impl ApplicationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all applications for a team (via projects/environments)
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            r#"
            SELECT a.*
            FROM applications a
            JOIN environments e ON a.environment_id = e.id
            JOIN projects p ON e.project_id = p.id
            WHERE p.team_id = $1
            ORDER BY a.created_at DESC
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Application::from).collect())
    }

    /// Find application by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Application>> {
        let row = sqlx::query_as::<_, ApplicationRow>(
            "SELECT * FROM applications WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Application::from))
    }

    /// Find applications by server
    pub async fn find_by_server(&self, server_id: Uuid) -> Result<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT * FROM applications WHERE server_id = $1 ORDER BY name"
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Application::from).collect())
    }

    /// Find applications by environment
    pub async fn find_by_environment(&self, environment_id: Uuid) -> Result<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT * FROM applications WHERE environment_id = $1 ORDER BY name"
        )
        .bind(environment_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Application::from).collect())
    }

    /// Create application
    pub async fn create(&self, app: &Application) -> Result<Application> {
        let (git_repo, git_branch, git_sha, docker_image, dockerfile_content, compose_content) =
            match &app.source {
                ApplicationSource::Git { repository_url, branch, commit_sha, .. } =>
                    (Some(repository_url.clone()), Some(branch.clone()), commit_sha.clone(), None, None, None),
                ApplicationSource::DockerImage { image, .. } =>
                    (None, None, None, Some(image.clone()), None, None),
                ApplicationSource::Dockerfile { content, .. } =>
                    (None, None, None, None, Some(content.clone()), None),
                ApplicationSource::DockerCompose { content } =>
                    (None, None, None, None, None, Some(content.clone())),
            };

        let build_pack = format!("{:?}", app.build_config.build_pack).to_lowercase();
        let status = format!("{:?}", app.status).to_lowercase();
        let ports = serde_json::to_value(&app.deploy_config.ports)
            .map_err(|e| Error::Internal(e.to_string()))?;

        let (hc_path, hc_port, hc_interval, hc_timeout, hc_retries, hc_start) =
            if let Some(hc) = &app.deploy_config.health_check {
                (Some(hc.path.clone()), Some(hc.port as i32),
                 Some(hc.interval as i32), Some(hc.timeout as i32),
                 Some(hc.retries as i32), Some(hc.start_period as i32))
            } else {
                (None, None, None, None, None, None)
            };

        let row = sqlx::query_as::<_, ApplicationRow>(
            r#"
            INSERT INTO applications (
                id, environment_id, server_id, name, description, fqdn,
                git_repository, git_branch, git_commit_sha, docker_image,
                dockerfile_content, docker_compose_content,
                build_pack, dockerfile_path, build_command, install_command, start_command,
                base_directory, publish_directory,
                status, replicas,
                health_check_enabled, health_check_path, health_check_port,
                health_check_interval, health_check_timeout, health_check_retries, health_check_start_period,
                limits_memory, limits_memory_reservation, limits_cpus,
                ports_mappings
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
                $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32
            )
            RETURNING *
            "#
        )
        .bind(app.id)
        .bind(app.environment_id)
        .bind(app.server_id)
        .bind(&app.name)
        .bind(&app.description)
        .bind(&app.fqdn)
        .bind(&git_repo)
        .bind(&git_branch)
        .bind(&git_sha)
        .bind(&docker_image)
        .bind(&dockerfile_content)
        .bind(&compose_content)
        .bind(&build_pack)
        .bind(&app.build_config.dockerfile_path)
        .bind(&app.build_config.build_command)
        .bind(&app.build_config.install_command)
        .bind(&app.build_config.start_command)
        .bind(&app.build_config.base_directory)
        .bind(&app.build_config.publish_directory)
        .bind(&status)
        .bind(app.deploy_config.replicas as i32)
        .bind(app.deploy_config.health_check.is_some())
        .bind(&hc_path)
        .bind(hc_port)
        .bind(hc_interval)
        .bind(hc_timeout)
        .bind(hc_retries)
        .bind(hc_start)
        .bind(&app.deploy_config.resources.memory_limit)
        .bind(&app.deploy_config.resources.memory_reservation)
        .bind(app.deploy_config.resources.cpu_limit.map(|c| c.to_string()))
        .bind(&ports)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Application::from(row))
    }

    /// Update application status
    pub async fn update_status(&self, id: Uuid, status: ApplicationStatus) -> Result<()> {
        let status_str = format!("{:?}", status).to_lowercase();

        sqlx::query("UPDATE applications SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(&status_str)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Delete application
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM applications WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get environment variables for application
    pub async fn get_environment_variables(&self, app_id: Uuid) -> Result<Vec<EnvironmentVariable>> {
        let rows = sqlx::query_as::<_, (String, String, bool)>(
            r#"
            SELECT key, value, is_secret
            FROM environment_variables
            WHERE resource_type = 'application' AND resource_id = $1
            ORDER BY key
            "#
        )
        .bind(app_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|(key, value, is_secret)| {
            EnvironmentVariable { key, value, is_secret }
        }).collect())
    }

    /// Set environment variable
    pub async fn set_environment_variable(
        &self,
        app_id: Uuid,
        key: &str,
        value: &str,
        is_secret: bool
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO environment_variables (resource_type, resource_id, key, value, is_secret)
            VALUES ('application', $1, $2, $3, $4)
            ON CONFLICT (resource_type, resource_id, key)
            DO UPDATE SET value = $3, is_secret = $4
            "#
        )
        .bind(app_id)
        .bind(key)
        .bind(value)
        .bind(is_secret)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }
}
