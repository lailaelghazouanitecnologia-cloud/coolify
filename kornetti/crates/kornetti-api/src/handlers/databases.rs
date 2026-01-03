//! Database handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{StandaloneDatabase, DatabaseType};
use kornetti_core::actions::database::{
    StartPostgresql, StartMysql, StartRedis, StartMongodb,
    StartMariadb, StopDatabase, RestartDatabase, DatabaseConfig,
};
use kornetti_core::actions::Action;
use crate::{state::{AppState, AuthContext}, ApiError};

#[derive(Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub environment_id: Uuid,
    pub server_id: Uuid,
    pub database_type: DatabaseType,
    pub version: Option<String>,
    /// Initial database name to create
    pub database_name: Option<String>,
    /// Username for database access
    pub username: Option<String>,
    /// Password (auto-generated if not provided)
    pub password: Option<String>,
    /// Public port for external access (optional)
    pub public_port: Option<u16>,
    /// Memory limit (e.g., "512m", "1g")
    pub limits_memory: Option<String>,
    /// CPU limit (e.g., 0.5, 1.0)
    pub limits_cpus: Option<f64>,
}

#[derive(Deserialize)]
pub struct UpdateDatabaseRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub public_port: Option<u16>,
    pub limits_memory: Option<String>,
    pub limits_cpus: Option<f64>,
}

#[derive(Serialize)]
pub struct DatabaseStatus {
    pub running: bool,
    pub container_id: Option<String>,
    pub memory_usage: Option<String>,
    pub cpu_usage: Option<String>,
    pub uptime: Option<String>,
}

#[derive(Serialize)]
pub struct DatabaseConnection {
    pub internal_host: String,
    pub internal_port: u16,
    pub external_host: Option<String>,
    pub external_port: Option<u16>,
    pub username: String,
    pub database: String,
    pub connection_string: String,
}

/// List databases for the current team
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<StandaloneDatabase>>, ApiError> {
    let repo = state.databases();

    let databases = repo.find_by_team(auth.team_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch databases: {}", e)))?;

    Ok(Json(databases))
}

/// Get a specific database
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<StandaloneDatabase>, ApiError> {
    let repo = state.databases();

    let database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership via environment -> project -> team
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    Ok(Json(database))
}

/// Create a new database
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateDatabaseRequest>,
) -> Result<Json<StandaloneDatabase>, ApiError> {
    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::validation("Database name cannot be empty"));
    }

    // Validate environment exists and belongs to team
    let env = state.projects().find_environment_by_uuid(body.environment_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch environment: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Environment not found"))?;

    let project = state.projects().find_by_id_raw(env.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Environment belongs to another team"));
    }

    // Validate server exists and belongs to team
    let server = state.servers().find_by_uuid(body.server_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    if server.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Server belongs to another team"));
    }

    // Generate credentials if not provided
    let db_name = body.database_name.unwrap_or_else(|| {
        format!("db_{}", Uuid::new_v4().to_string().replace("-", "")[..8].to_string())
    });
    let username = body.username.unwrap_or_else(|| "coolify".to_string());
    let password = body.password.unwrap_or_else(|| generate_password());

    // Get default version for database type
    let version = body.version.unwrap_or_else(|| {
        match body.database_type {
            DatabaseType::Postgresql => "16".to_string(),
            DatabaseType::Mysql => "8.0".to_string(),
            DatabaseType::Mariadb => "11".to_string(),
            DatabaseType::Redis => "7".to_string(),
            DatabaseType::Mongodb => "7".to_string(),
            DatabaseType::Clickhouse => "24".to_string(),
            DatabaseType::Keydb => "latest".to_string(),
            DatabaseType::Dragonfly => "latest".to_string(),
        }
    });

    // Create database record
    let database = StandaloneDatabase {
        id: 0,
        uuid: Uuid::new_v4(),
        name: body.name,
        description: None,
        database_type: body.database_type.clone(),
        status: "stopped".to_string(),
        image: format!("{}:{}", body.database_type.default_image(), version),
        public_port: body.public_port,
        internal_db_url: None,
        external_db_url: None,
        environment_variables: None,
        limits_memory: body.limits_memory,
        limits_cpus: body.limits_cpus,
        limits_memory_swap: None,
        limits_memory_swappiness: None,
        destination_id: server.id,
        destination_type: "server".to_string(),
        environment_id: env.id,
        project_id: project.id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
        // Database-specific fields stored in environment_variables or separate columns
    };

    let repo = state.databases();
    let created = repo.create(database)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create database: {}", e)))?;

    // Store credentials securely (in a real implementation, encrypt these)
    repo.set_credentials(created.id as i64, &db_name, &username, &password)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to store credentials: {}", e)))?;

    Ok(Json(created))
}

/// Delete a database
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.databases();

    let database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    // Stop the database first if running
    if database.status == "running" {
        let server = state.servers().find_by_id_raw(database.destination_id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
            .ok_or_else(|| ApiError::not_found("Server not found"))?;

        let stop_action = StopDatabase::new(state.ssh.clone());
        stop_action.handle(kornetti_core::actions::database::StopDatabaseInput {
            server: Box::leak(Box::new(server)),
            database: Box::leak(Box::new(database.clone())),
            remove_volumes: true,
        })
        .await
        .map_err(|e| ApiError::internal(format!("Failed to stop database: {}", e)))?;
    }

    repo.delete(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete database: {}", e)))?;

    Ok(Json(()))
}

/// Start a database
pub async fn start(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<StandaloneDatabase>, ApiError> {
    let repo = state.databases();

    let mut database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    // Already running?
    if database.status == "running" {
        return Err(ApiError::conflict("Database is already running"));
    }

    let server = state.servers().find_by_id_raw(database.destination_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    // Get credentials
    let creds = repo.get_credentials(database.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch credentials: {}", e)))?;

    // Build config
    let config = DatabaseConfig {
        container_name: database.uuid.to_string(),
        image: database.image.clone(),
        version: database.image.split(':').nth(1).unwrap_or("latest").to_string(),
        internal_port: database.database_type.default_port(),
        public_port: database.public_port,
        database_name: creds.database_name,
        username: creds.username,
        password: creds.password,
        root_password: Some(generate_password()),
        memory_limit: database.limits_memory.clone(),
        cpu_limit: database.limits_cpus,
        environment: vec![],
        volumes: vec![format!("{}:/data", database.uuid)],
        network: "coolify".to_string(),
    };

    // Start based on database type
    let result = match database.database_type {
        DatabaseType::Postgresql => {
            let action = StartPostgresql::new(state.ssh.clone());
            action.handle(kornetti_core::actions::database::StartDatabaseInput {
                server: Box::leak(Box::new(server)),
                config: Box::leak(Box::new(config)),
            }).await
        }
        DatabaseType::Mysql => {
            let action = StartMysql::new(state.ssh.clone());
            action.handle(kornetti_core::actions::database::StartDatabaseInput {
                server: Box::leak(Box::new(server)),
                config: Box::leak(Box::new(config)),
            }).await
        }
        DatabaseType::Mariadb => {
            let action = StartMariadb::new(state.ssh.clone());
            action.handle(kornetti_core::actions::database::StartDatabaseInput {
                server: Box::leak(Box::new(server)),
                config: Box::leak(Box::new(config)),
            }).await
        }
        DatabaseType::Redis => {
            let action = StartRedis::new(state.ssh.clone());
            action.handle(kornetti_core::actions::database::StartDatabaseInput {
                server: Box::leak(Box::new(server)),
                config: Box::leak(Box::new(config)),
            }).await
        }
        DatabaseType::Mongodb => {
            let action = StartMongodb::new(state.ssh.clone());
            action.handle(kornetti_core::actions::database::StartDatabaseInput {
                server: Box::leak(Box::new(server)),
                config: Box::leak(Box::new(config)),
            }).await
        }
        _ => return Err(ApiError::bad_request("Database type not yet supported")),
    };

    result.map_err(|e| ApiError::internal(format!("Failed to start database: {}", e)))?;

    // Update status
    database.status = "running".to_string();
    database.updated_at = chrono::Utc::now();

    let updated = repo.update(database)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update database: {}", e)))?;

    Ok(Json(updated))
}

/// Stop a database
pub async fn stop(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<StandaloneDatabase>, ApiError> {
    let repo = state.databases();

    let mut database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    // Already stopped?
    if database.status == "stopped" {
        return Err(ApiError::conflict("Database is already stopped"));
    }

    let server = state.servers().find_by_id_raw(database.destination_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    let action = StopDatabase::new(state.ssh.clone());
    action.handle(kornetti_core::actions::database::StopDatabaseInput {
        server: Box::leak(Box::new(server)),
        database: Box::leak(Box::new(database.clone())),
        remove_volumes: false,
    })
    .await
    .map_err(|e| ApiError::internal(format!("Failed to stop database: {}", e)))?;

    // Update status
    database.status = "stopped".to_string();
    database.updated_at = chrono::Utc::now();

    let updated = repo.update(database)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update database: {}", e)))?;

    Ok(Json(updated))
}

/// Restart a database
pub async fn restart(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<StandaloneDatabase>, ApiError> {
    let repo = state.databases();

    let database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    let server = state.servers().find_by_id_raw(database.destination_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    let action = RestartDatabase::new(state.ssh.clone());
    action.handle(kornetti_core::actions::database::RestartDatabaseInput {
        server: Box::leak(Box::new(server)),
        database: Box::leak(Box::new(database.clone())),
    })
    .await
    .map_err(|e| ApiError::internal(format!("Failed to restart database: {}", e)))?;

    Ok(Json(database))
}

/// Get database connection info
pub async fn connection(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<DatabaseConnection>, ApiError> {
    let repo = state.databases();

    let database = repo.find_by_uuid(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch database: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Database not found"))?;

    // Verify team ownership
    let project = state.projects().find_by_id_raw(database.project_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch project: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Project not found"))?;

    if project.team_id != auth.team_id && !auth.is_admin {
        return Err(ApiError::forbidden("Database belongs to another team"));
    }

    let creds = repo.get_credentials(database.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch credentials: {}", e)))?;

    let server = state.servers().find_by_id_raw(database.destination_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch server: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Server not found"))?;

    let internal_port = database.database_type.default_port();
    let connection_string = database.database_type.connection_string(
        &database.uuid.to_string(),
        internal_port,
        &creds.database_name,
        &creds.username,
        &creds.password,
    );

    Ok(Json(DatabaseConnection {
        internal_host: database.uuid.to_string(),
        internal_port,
        external_host: database.public_port.map(|_| server.ip.clone()),
        external_port: database.public_port,
        username: creds.username,
        database: creds.database_name,
        connection_string,
    }))
}

/// Generate a secure random password
fn generate_password() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

impl DatabaseType {
    fn default_image(&self) -> &'static str {
        match self {
            DatabaseType::Postgresql => "postgres",
            DatabaseType::Mysql => "mysql",
            DatabaseType::Mariadb => "mariadb",
            DatabaseType::Redis => "redis",
            DatabaseType::Mongodb => "mongo",
            DatabaseType::Clickhouse => "clickhouse/clickhouse-server",
            DatabaseType::Keydb => "eqalpha/keydb",
            DatabaseType::Dragonfly => "docker.dragonflydb.io/dragonflydb/dragonfly",
        }
    }

    fn default_port(&self) -> u16 {
        match self {
            DatabaseType::Postgresql => 5432,
            DatabaseType::Mysql | DatabaseType::Mariadb => 3306,
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => 6379,
            DatabaseType::Mongodb => 27017,
            DatabaseType::Clickhouse => 9000,
        }
    }

    fn connection_string(&self, host: &str, port: u16, db: &str, user: &str, pass: &str) -> String {
        match self {
            DatabaseType::Postgresql => format!("postgresql://{}:{}@{}:{}/{}", user, pass, host, port, db),
            DatabaseType::Mysql | DatabaseType::Mariadb => format!("mysql://{}:{}@{}:{}/{}", user, pass, host, port, db),
            DatabaseType::Redis | DatabaseType::Keydb | DatabaseType::Dragonfly => format!("redis://{}:{}", host, port),
            DatabaseType::Mongodb => format!("mongodb://{}:{}@{}:{}/{}", user, pass, host, port, db),
            DatabaseType::Clickhouse => format!("clickhouse://{}:{}@{}:{}/{}", user, pass, host, port, db),
        }
    }
}
