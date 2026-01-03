//! Team handlers

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use kornetti_core::models::{Team, TeamSettings, TeamRole};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberResponse {
    pub user_id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: TeamRole,
    pub joined_at: chrono::DateTime<chrono::Utc>,
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
        id: Uuid::new_v4(),
        name: body.name,
        description: body.description,
        personal: false,
        settings: TeamSettings::default(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let created = repo.create(&team)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create team: {}", e)))?;

    // Add the creator as owner
    repo.add_member(created.id, auth.user_id, TeamRole::Owner)
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
    let role = repo.get_member_role(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_update = auth.is_admin || role.map(|r| r.can_manage_team()).unwrap_or(false);
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

    let updated = repo.update(&team)
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
    let role = repo.get_member_role(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_delete = auth.is_admin || role == Some(TeamRole::Owner);
    if !can_delete {
        return Err(ApiError::forbidden("Only team owners can delete teams"));
    }

    // Cannot delete personal team
    if team.personal {
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
) -> Result<Json<Vec<kornetti_core::models::TeamMember>>, ApiError> {
    let repo = state.teams();

    // Verify user is a member
    let is_member = repo.is_member(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))?;

    if !is_member && !auth.is_admin {
        return Err(ApiError::forbidden("You are not a member of this team"));
    }

    let members = repo.get_members(id)
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

    // Verify team exists
    let _ = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user has permission to invite
    let role = repo.get_member_role(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_invite = auth.is_admin || role.map(|r| r.can_manage_team()).unwrap_or(false);
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
    let is_member = repo.is_member(id, user.id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check membership: {}", e)))?;

    if is_member {
        return Err(ApiError::conflict("User is already a member of this team"));
    }

    // Parse role
    let member_role = match body.role.as_deref() {
        Some("owner") => TeamRole::Owner,
        Some("admin") => TeamRole::Admin,
        Some("viewer") => TeamRole::Viewer,
        _ => TeamRole::Member,
    };

    // Add member
    repo.add_member(id, user.id, member_role)
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

    // Verify team exists
    let _ = repo.find_by_id(id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch team: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Team not found"))?;

    // Verify user has permission
    let role = repo.get_member_role(id, auth.user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check role: {}", e)))?;

    let can_remove = auth.is_admin || role.map(|r| r.can_manage_team()).unwrap_or(false);

    // Users can remove themselves
    let is_self = user_id == auth.user_id;

    if !can_remove && !is_self {
        return Err(ApiError::forbidden("You don't have permission to remove members"));
    }

    // Get the role of user being removed
    let target_role = repo.get_member_role(id, user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to check target role: {}", e)))?;

    // Cannot remove the last owner - would need to count owners
    if target_role == Some(TeamRole::Owner) {
        let members = repo.get_members(id)
            .await
            .map_err(|e| ApiError::internal(format!("Failed to count owners: {}", e)))?;

        let owner_count = members.iter().filter(|m| m.role == TeamRole::Owner).count();

        if owner_count <= 1 {
            return Err(ApiError::bad_request("Cannot remove the last owner"));
        }
    }

    repo.remove_member(id, user_id)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to remove member: {}", e)))?;

    Ok(Json(()))
}
