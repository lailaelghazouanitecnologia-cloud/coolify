//! Application state

use std::sync::Arc;
use kornetti_core::Config;
use kornetti_core::repositories::{
    ServerRepository, ApplicationRepository, ProjectRepository,
    TeamRepository, UserRepository, DeploymentRepository, DatabaseRepository,
};
use kornetti_providers::ProviderFactory;
use kornetti_ssh::SshClient;
use sqlx::PgPool;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub providers: ProviderFactory,
    pub ssh: Arc<SshClient>,
}

impl AppState {
    pub fn new(config: Config, db: PgPool) -> Self {
        let providers = ProviderFactory::new(config.providers.clone());
        let ssh = Arc::new(SshClient::new());

        Self {
            config,
            db,
            providers,
            ssh,
        }
    }

    // Repository accessors for cleaner handler code

    /// Get server repository
    pub fn servers(&self) -> ServerRepository {
        ServerRepository::new(self.db.clone())
    }

    /// Get application repository
    pub fn applications(&self) -> ApplicationRepository {
        ApplicationRepository::new(self.db.clone())
    }

    /// Get project repository
    pub fn projects(&self) -> ProjectRepository {
        ProjectRepository::new(self.db.clone())
    }

    /// Get team repository
    pub fn teams(&self) -> TeamRepository {
        TeamRepository::new(self.db.clone())
    }

    /// Get user repository
    pub fn users(&self) -> UserRepository {
        UserRepository::new(self.db.clone())
    }

    /// Get deployment repository
    pub fn deployments(&self) -> DeploymentRepository {
        DeploymentRepository::new(self.db.clone())
    }

    /// Get database repository
    pub fn databases(&self) -> DatabaseRepository {
        DatabaseRepository::new(self.db.clone())
    }
}

/// Authenticated user context from JWT/session
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: uuid::Uuid,
    pub team_id: uuid::Uuid,
    pub email: String,
    pub is_admin: bool,
}

impl AuthContext {
    pub fn new(user_id: uuid::Uuid, team_id: uuid::Uuid, email: String, is_admin: bool) -> Self {
        Self { user_id, team_id, email, is_admin }
    }
}
