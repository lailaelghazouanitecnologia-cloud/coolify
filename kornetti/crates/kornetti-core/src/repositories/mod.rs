//! Repository layer for database persistence
//!
//! Provides async database operations for all domain models.

pub mod server;
pub mod application;
pub mod project;
pub mod team;
pub mod user;
pub mod deployment;
pub mod database;
pub mod private_key;
pub mod environment_variable;
pub mod notification;

pub use server::ServerRepository;
pub use application::ApplicationRepository;
pub use project::ProjectRepository;
pub use team::TeamRepository;
pub use user::UserRepository;
pub use deployment::DeploymentRepository;
pub use database::DatabaseRepository;
pub use private_key::PrivateKeyRepository;
pub use environment_variable::EnvironmentVariableRepository;
pub use notification::{NotificationRepository, NotificationSettings, NotificationTypeSettings, NotificationEvent};

use sqlx::PgPool;

/// Database connection wrapper
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Connect to database with URL
    pub async fn connect(url: &str) -> crate::Result<Self> {
        let pool = PgPool::connect(url)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(Self::new(pool))
    }

    /// Run migrations
    pub async fn migrate(&self) -> crate::Result<()> {
        sqlx::migrate!("../../migrations")
            .run(&self.pool)
            .await
            .map_err(|e| crate::Error::Database(e.to_string()))?;
        Ok(())
    }
}
