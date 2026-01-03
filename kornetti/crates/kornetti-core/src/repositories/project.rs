//! Project repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{Project, Environment};

#[derive(FromRow)]
struct ProjectRow {
    id: Uuid,
    team_id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct EnvironmentRow {
    id: Uuid,
    project_id: Uuid,
    name: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<ProjectRow> for Project {
    fn from(row: ProjectRow) -> Self {
        Project {
            id: row.id,
            team_id: row.team_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<EnvironmentRow> for Environment {
    fn from(row: EnvironmentRow) -> Self {
        Environment {
            id: row.id,
            project_id: row.project_id,
            name: row.name,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub struct ProjectRepository {
    pool: PgPool,
}

impl ProjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find all projects for a team
    pub async fn find_by_team(&self, team_id: Uuid) -> Result<Vec<Project>> {
        let rows = sqlx::query_as::<_, ProjectRow>(
            "SELECT * FROM projects WHERE team_id = $1 ORDER BY name"
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Project::from).collect())
    }

    /// Find project by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Project>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT * FROM projects WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Project::from))
    }

    /// Find project by ID and team (for authorization)
    pub async fn find_by_id_and_team(&self, id: Uuid, team_id: Uuid) -> Result<Option<Project>> {
        let row = sqlx::query_as::<_, ProjectRow>(
            "SELECT * FROM projects WHERE id = $1 AND team_id = $2"
        )
        .bind(id)
        .bind(team_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Project::from))
    }

    /// Create project
    pub async fn create(&self, project: &Project) -> Result<Project> {
        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            INSERT INTO projects (id, team_id, name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(project.id)
        .bind(project.team_id)
        .bind(&project.name)
        .bind(&project.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Project::from(row))
    }

    /// Update project
    pub async fn update(&self, project: &Project) -> Result<Project> {
        let row = sqlx::query_as::<_, ProjectRow>(
            r#"
            UPDATE projects
            SET name = $2, description = $3
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(project.id)
        .bind(&project.name)
        .bind(&project.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Project::from(row))
    }

    /// Delete project
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    // ===== Environment methods =====

    /// Find environments for a project
    pub async fn find_environments(&self, project_id: Uuid) -> Result<Vec<Environment>> {
        let rows = sqlx::query_as::<_, EnvironmentRow>(
            "SELECT * FROM environments WHERE project_id = $1 ORDER BY name"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Environment::from).collect())
    }

    /// Find environment by ID
    pub async fn find_environment_by_id(&self, id: Uuid) -> Result<Option<Environment>> {
        let row = sqlx::query_as::<_, EnvironmentRow>(
            "SELECT * FROM environments WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Environment::from))
    }

    /// Create environment
    pub async fn create_environment(&self, env: &Environment) -> Result<Environment> {
        let row = sqlx::query_as::<_, EnvironmentRow>(
            r#"
            INSERT INTO environments (id, project_id, name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#
        )
        .bind(env.id)
        .bind(env.project_id)
        .bind(&env.name)
        .bind(&env.description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Environment::from(row))
    }

    /// Delete environment
    pub async fn delete_environment(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM environments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Create project with default production environment
    pub async fn create_with_environment(&self, project: &Project) -> Result<(Project, Environment)> {
        let project = self.create(project).await?;

        let env = Environment::production(project.id);
        let env = self.create_environment(&env).await?;

        Ok((project, env))
    }
}
