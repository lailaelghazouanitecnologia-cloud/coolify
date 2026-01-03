//! Team handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::Team;
use crate::{state::{AppState, AuthContext}, ApiError};

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct TeamWithMembers {
    #[serde(flatten)]
    pub team: Team,
    pub member_count: i64,
}

#[derive(Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: Option<String>,
}

/// List teams the current user belongs to
pub async fn list(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<Team>>, ApiError> {
    let repo = state.teams();

    let teams = repo.find_by_user(auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch teams: {}", e)))?;

    Ok(Json(teams))
}

/// Get a specific team
pub async fn get(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Team>, ApiError> {
    let repo = state.teams();

    let team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user is a member of this team
    let is_member = repo.is_member(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))?;

    if !is_member && !auth.is_admin {
        return Err(ApiError::forbidden("You are not a member of this team"));
    }

    Ok(Json(team))
}

/// Create a new team
pub async fn create(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<CreateTeamRequest>,
) -> Result<Json<Team>, ApiError> {
    // Validate name
    if body.name.trim().is_empty() {
        return Err(ApiError::validation("Team name cannot be empty"));
    }

    if body.name.len() > 255 {
        return Err(ApiError::validation("Team name is too long"));
    }

    let repo = state.teams();

    // Create team
    let team = Team {
        id: 0, // Will be set by database
        uuid: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        personal_team: false,
        show_boarding: false,
        custom_server_limit: None,
        discord_enabled: false,
        discord_webhook_url: None,
        discord_notifications_test: false,
        discord_notifications_deployments: false,
        discord_notifications_status_changes: false,
        discord_notifications_database_backups: false,
        discord_notifications_scheduled_tasks: false,
        discord_notifications_server_disk_usage: false,
        smtp_enabled: false,
        smtp_from_address: None,
        smtp_from_name: None,
        smtp_recipients: None,
        smtp_host: None,
        smtp_port: None,
        smtp_encryption: None,
        smtp_username: None,
        smtp_password: None,
        smtp_timeout: None,
        smtp_notifications_test: false,
        smtp_notifications_deployments: false,
        smtp_notifications_status_changes: false,
        smtp_notifications_database_backups: false,
        smtp_notifications_scheduled_tasks: false,
        smtp_notifications_server_disk_usage: false,
        telegram_enabled: false,
        telegram_token: None,
        telegram_chat_id: None,
        telegram_notifications_test: false,
        telegram_notifications_deployments: false,
        telegram_notifications_status_changes: false,
        telegram_notifications_database_backups: false,
        telegram_notifications_scheduled_tasks: false,
        telegram_notifications_server_disk_usage: false,
        resend_enabled: false,
        resend_api_key: None,
        use_instance_email_settings: false,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        deleted_at: None,
    };

    let created = repo.create(team)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create team: {}", e)))?;

    // Add the creator as owner
    repo.add_member(created.id as i64, auth.user_id, "owner")
        .await
        .map_err(|e| ApiError::internal(format!("Failed to add owner: {}", e)))?;

    Ok(Json(created))
}

/// Update a team
pub async fn update(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTeamRequest>,
) -> Result<Json<Team>, ApiError> {
    let repo = state.teams();

    let mut team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user has permission to update
    let role = repo.get_member_role(team.id as i64, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_update = auth.is_admin || matches!(role.as_deref(), Some("owner") | Some("admin"));
    if !can_update {
        return Err(ApiError::forbidden("You don't have permission to update this team"));
    }

    // Apply updates
    if let Some(name) = body.name {
        if name.trim().is_empty() {
            return Err(ApiError::validation("Team name cannot be empty"));
        }
        team.name = name;
    }

    if let Some(description) = body.description {
        team.description = Some(description);
    }

    team.updated_at = chrono::Utc::now();

    let updated = repo.update(team)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update team: {}", e)))?;

    Ok(Json(updated))
}

/// Delete a team
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    let repo = state.teams();

    let team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Only owner or admin can delete
    let role = repo.get_member_role(team.id as i64, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_delete = auth.is_admin || role.as_deref() == Some("owner");
    if !can_delete {
        return Err(ApiError::forbidden("Only team owners can delete teams"));
    }

    // Cannot delete personal team
    if team.personal_team {
        return Err(ApiError::bad_request("Cannot delete personal team"));
    }

    repo.delete(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to delete team: {}", e)))?;

    Ok(Json(()))
}

/// List team members
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TeamMember>>, ApiError> {
    let repo = state.teams();

    // Verify user is a member
    let team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    let is_member = repo.is_member(team.id as i64, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))?;

    if !is_member && !auth.is_admin {
        return Err(ApiError::forbidden("You are not a member of this team"));
    }

    let members = repo.get_members(team.id as i64)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch members: {}", e)))?;

    Ok(Json(members))
}

/// Invite a member to the team
pub async fn invite_member(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<InviteMemberRequest>,
) -> Result<Json<()>, ApiError> {
    let repo = state.teams();

    let team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user has permission to invite
    let role = repo.get_member_role(team.id as i64, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_invite = auth.is_admin || matches!(role.as_deref(), Some("owner") | Some("admin"));
    if !can_invite {
        return Err(ApiError::forbidden("You don't have permission to invite members"));
    }

    // Find user by email
    let user_repo = state.users();
    let user = user_repo.find_by_email(&body.email)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to find user: {}", e)))?
        .ok_or_else(|| ApiError::not_found("User not found"))?;

    // Check if already a member
    let is_member = repo.is_member(team.id as i64, user.uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))?;

    if is_member {
        return Err(ApiError::conflict("User is already a member of this team"));
    }

    // Add member
    let member_role = body.role.as_deref().unwrap_or("member");
    repo.add_member(team.id as i64, user.uuid, member_role)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to add member: {}", e)))?;

    Ok(Json(()))
}

/// Remove a member from the team
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, ApiError> {
    let repo = state.teams();

    let team = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user has permission
    let role = repo.get_member_role(team.id as i64, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_remove = auth.is_admin || matches!(role.as_deref(), Some("owner") | Some("admin"));

    // Users can remove themselves
    let is_self = user_id == auth.user_id;

    if !can_remove && !is_self {
        return Err(ApiError::forbidden("You don't have permission to remove members"));
    }

    // Cannot remove the last owner
    if role.as_deref() == Some("owner") {
        let owner_count = repo.count_owners(team.id as i64)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to count owners: {}", e)))?;

        if owner_count <= 1 {
            return Err(ApiError::bad_request("Cannot remove the last owner"));
        }
    }

    repo.remove_member(team.id as i64, user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to remove member: {}", e)))?;

    Ok(Json(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub user_id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}
