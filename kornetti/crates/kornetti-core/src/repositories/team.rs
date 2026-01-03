//! Team repository

use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{Result, Error};
use crate::models::{Team, TeamSettings, TeamMember, TeamRole};

#[derive(FromRow)]
struct TeamRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    personal: bool,
    settings: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<TeamRow> for Team {
    fn from(row: TeamRow) -> Self {
        let settings: TeamSettings = serde_json::from_value(row.settings)
            .unwrap_or_default();

        Team {
            id: row.id,
            name: row.name,
            description: row.description,
            personal: row.personal,
            settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub struct TeamRepository {
    pool: PgPool,
}

impl TeamRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Find team by ID
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Team>> {
        let row = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(Team::from))
    }

    /// Find teams for a user
    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<Team>> {
        let rows = sqlx::query_as::<_, TeamRow>(
            r#"
            SELECT t.* FROM teams t
            JOIN team_members tm ON t.id = tm.team_id
            WHERE tm.user_id = $1
            ORDER BY t.personal DESC, t.name ASC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Team::from).collect())
    }

    /// Create team
    pub async fn create(&self, team: &Team) -> Result<Team> {
        let settings = serde_json::to_value(&team.settings)
            .map_err(|e| Error::Internal(e.to_string()))?;

        let row = sqlx::query_as::<_, TeamRow>(
            r#"
            INSERT INTO teams (id, name, description, personal, settings)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(team.id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(team.personal)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Team::from(row))
    }

    /// Update team
    pub async fn update(&self, team: &Team) -> Result<Team> {
        let settings = serde_json::to_value(&team.settings)
            .map_err(|e| Error::Internal(e.to_string()))?;

        let row = sqlx::query_as::<_, TeamRow>(
            r#"
            UPDATE teams
            SET name = $2, description = $3, settings = $4
            WHERE id = $1
            RETURNING *
            "#
        )
        .bind(team.id)
        .bind(&team.name)
        .bind(&team.description)
        .bind(&settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Team::from(row))
    }

    /// Delete team
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Add member to team
    pub async fn add_member(&self, team_id: Uuid, user_id: Uuid, role: TeamRole) -> Result<()> {
        let role_str = format!("{:?}", role).to_lowercase();

        sqlx::query(
            r#"
            INSERT INTO team_members (team_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (team_id, user_id) DO UPDATE SET role = $3
            "#
        )
        .bind(team_id)
        .bind(user_id)
        .bind(&role_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Remove member from team
    pub async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM team_members WHERE team_id = $1 AND user_id = $2")
            .bind(team_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get team members
    pub async fn get_members(&self, team_id: Uuid) -> Result<Vec<TeamMember>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, DateTime<Utc>)>(
            r#"
            SELECT id, team_id, user_id, role, created_at
            FROM team_members
            WHERE team_id = $1
            ORDER BY created_at ASC
            "#
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|(id, team_id, user_id, role, created_at)| {
            TeamMember {
                id,
                team_id,
                user_id,
                role: parse_team_role(&role),
                created_at,
            }
        }).collect())
    }

    /// Check if user is member of team
    pub async fn is_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool> {
        let row = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(*) FROM team_members WHERE team_id = $1 AND user_id = $2"
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.0 > 0)
    }

    /// Get user role in team
    pub async fn get_member_role(&self, team_id: Uuid, user_id: Uuid) -> Result<Option<TeamRole>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM team_members WHERE team_id = $1 AND user_id = $2"
        )
        .bind(team_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        Ok(row.map(|r| parse_team_role(&r.0)))
    }
}

fn parse_team_role(s: &str) -> TeamRole {
    match s {
        "owner" => TeamRole::Owner,
        "admin" => TeamRole::Admin,
        "member" => TeamRole::Member,
        "viewer" => TeamRole::Viewer,
        _ => TeamRole::Viewer,
    }
}
