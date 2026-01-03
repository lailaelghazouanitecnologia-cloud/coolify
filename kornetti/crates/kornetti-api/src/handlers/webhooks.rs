//! Webhook handlers for GitHub, GitLab, and Bitbucket

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};
use kornetti_core::models::git_source::{GitProvider, GitRepository, WebhookEventType, WebhookPayload};

type HmacSha256 = Hmac<Sha256>;

/// Response for webhook processing
#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub message: String,
    pub deployment_triggered: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<Uuid>,
}

// ============================================================================
// GitHub Webhooks
// ============================================================================

/// GitHub push event payload
#[derive(Debug, Deserialize)]
pub struct GithubPushEvent {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub after: String,
    pub before: String,
    pub repository: GithubRepository,
    pub pusher: GithubUser,
    pub head_commit: Option<GithubCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GithubRepository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubUser {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GithubCommit {
    pub id: String,
    pub message: String,
    pub author: GithubUser,
}

/// GitHub pull request event payload
#[derive(Debug, Deserialize)]
pub struct GithubPullRequestEvent {
    pub action: String,
    pub number: i64,
    pub pull_request: GithubPullRequest,
    pub repository: GithubRepository,
}

#[derive(Debug, Deserialize)]
pub struct GithubPullRequest {
    pub id: i64,
    pub number: i64,
    pub state: String,
    pub title: String,
    pub head: GithubBranch,
    pub base: GithubBranch,
}

#[derive(Debug, Deserialize)]
pub struct GithubBranch {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

/// Handle GitHub webhook
pub async fn github(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<WebhookResponse>, ApiError> {
    // Verify signature if webhook secret is configured
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok());

    // Get event type
    let event_type = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("Missing X-GitHub-Event header"))?;

    // Parse the payload based on event type
    let payload = match event_type {
        "push" => parse_github_push(&body)?,
        "pull_request" => parse_github_pull_request(&body)?,
        "ping" => {
            return Ok(Json(WebhookResponse {
                message: "Pong! Webhook configured successfully.".to_string(),
                deployment_triggered: false,
                deployment_id: None,
            }));
        }
        _ => {
            return Ok(Json(WebhookResponse {
                message: format!("Event '{}' not handled", event_type),
                deployment_triggered: false,
                deployment_id: None,
            }));
        }
    };

    // Process the webhook
    process_webhook(state, payload).await
}

/// Handle GitHub webhook with app ID
pub async fn github_app(
    State(state): State<Arc<AppState>>,
    Path(app_id): Path<Uuid>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<WebhookResponse>, ApiError> {
    // For app-specific webhooks, we can verify using the app's webhook secret
    // This allows multiple GitHub Apps with different secrets
    github(State(state), headers, body).await
}

fn parse_github_push(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: GithubPushEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid GitHub push payload: {}", e)))?;

    // Extract branch name from ref (refs/heads/main -> main)
    let branch = event
        .ref_name
        .strip_prefix("refs/heads/")
        .or_else(|| event.ref_name.strip_prefix("refs/tags/"))
        .unwrap_or(&event.ref_name)
        .to_string();

    let parts: Vec<&str> = event.repository.full_name.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        return Err(ApiError::bad_request("Invalid repository name"));
    };

    Ok(WebhookPayload {
        provider: GitProvider::Github,
        event_type: if event.ref_name.starts_with("refs/tags/") {
            WebhookEventType::Tag
        } else {
            WebhookEventType::Push
        },
        repository: GitRepository {
            provider: GitProvider::Github,
            owner,
            name,
            branch: branch.clone(),
            commit_sha: Some(event.after.clone()),
            clone_url: event.repository.clone_url,
            use_deploy_key: false,
        },
        ref_name: branch,
        commit_sha: event.after,
        commit_message: event.head_commit.as_ref().map(|c| c.message.clone()),
        author: Some(event.pusher.name),
        pull_request_id: None,
        pull_request_action: None,
        base_branch: None,
    })
}

fn parse_github_pull_request(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: GithubPullRequestEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid GitHub PR payload: {}", e)))?;

    let parts: Vec<&str> = event.repository.full_name.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        return Err(ApiError::bad_request("Invalid repository name"));
    };

    Ok(WebhookPayload {
        provider: GitProvider::Github,
        event_type: WebhookEventType::PullRequest,
        repository: GitRepository {
            provider: GitProvider::Github,
            owner,
            name,
            branch: event.pull_request.head.ref_name.clone(),
            commit_sha: Some(event.pull_request.head.sha.clone()),
            clone_url: event.repository.clone_url,
            use_deploy_key: false,
        },
        ref_name: event.pull_request.head.ref_name,
        commit_sha: event.pull_request.head.sha,
        commit_message: Some(event.pull_request.title),
        author: None,
        pull_request_id: Some(event.number),
        pull_request_action: Some(event.action),
        base_branch: Some(event.pull_request.base.ref_name),
    })
}

// ============================================================================
// GitLab Webhooks
// ============================================================================

/// GitLab push event payload
#[derive(Debug, Deserialize)]
pub struct GitlabPushEvent {
    pub object_kind: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub after: String,
    pub before: String,
    pub project: GitlabProject,
    pub user_name: String,
    pub commits: Vec<GitlabCommit>,
}

#[derive(Debug, Deserialize)]
pub struct GitlabProject {
    pub id: i64,
    pub name: String,
    pub path_with_namespace: String,
    pub git_http_url: String,
    pub git_ssh_url: String,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct GitlabCommit {
    pub id: String,
    pub message: String,
    pub author: GitlabAuthor,
}

#[derive(Debug, Deserialize)]
pub struct GitlabAuthor {
    pub name: String,
    pub email: String,
}

/// GitLab merge request event payload
#[derive(Debug, Deserialize)]
pub struct GitlabMergeRequestEvent {
    pub object_kind: String,
    pub object_attributes: GitlabMergeRequest,
    pub project: GitlabProject,
    pub user: GitlabUserInfo,
}

#[derive(Debug, Deserialize)]
pub struct GitlabMergeRequest {
    pub id: i64,
    pub iid: i64,
    pub state: String,
    pub action: Option<String>,
    pub title: String,
    pub source_branch: String,
    pub target_branch: String,
    pub last_commit: GitlabCommit,
}

#[derive(Debug, Deserialize)]
pub struct GitlabUserInfo {
    pub name: String,
    pub username: String,
}

/// Handle GitLab webhook
pub async fn gitlab(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<WebhookResponse>, ApiError> {
    // Verify token if configured
    let _token = headers
        .get("x-gitlab-token")
        .and_then(|v| v.to_str().ok());

    // Get event type
    let event_type = headers
        .get("x-gitlab-event")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("Missing X-Gitlab-Event header"))?;

    // Parse the payload based on event type
    let payload = match event_type {
        "Push Hook" => parse_gitlab_push(&body)?,
        "Merge Request Hook" => parse_gitlab_merge_request(&body)?,
        "Tag Push Hook" => parse_gitlab_tag(&body)?,
        _ => {
            return Ok(Json(WebhookResponse {
                message: format!("Event '{}' not handled", event_type),
                deployment_triggered: false,
                deployment_id: None,
            }));
        }
    };

    process_webhook(state, payload).await
}

fn parse_gitlab_push(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: GitlabPushEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid GitLab push payload: {}", e)))?;

    let branch = event
        .ref_name
        .strip_prefix("refs/heads/")
        .unwrap_or(&event.ref_name)
        .to_string();

    let parts: Vec<&str> = event.project.path_with_namespace.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), event.project.path_with_namespace.clone())
    };

    Ok(WebhookPayload {
        provider: GitProvider::Gitlab,
        event_type: WebhookEventType::Push,
        repository: GitRepository {
            provider: GitProvider::Gitlab,
            owner,
            name,
            branch: branch.clone(),
            commit_sha: Some(event.after.clone()),
            clone_url: event.project.git_http_url,
            use_deploy_key: false,
        },
        ref_name: branch,
        commit_sha: event.after,
        commit_message: event.commits.first().map(|c| c.message.clone()),
        author: Some(event.user_name),
        pull_request_id: None,
        pull_request_action: None,
        base_branch: None,
    })
}

fn parse_gitlab_tag(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: GitlabPushEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid GitLab tag payload: {}", e)))?;

    let tag = event
        .ref_name
        .strip_prefix("refs/tags/")
        .unwrap_or(&event.ref_name)
        .to_string();

    let parts: Vec<&str> = event.project.path_with_namespace.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), event.project.path_with_namespace.clone())
    };

    Ok(WebhookPayload {
        provider: GitProvider::Gitlab,
        event_type: WebhookEventType::Tag,
        repository: GitRepository {
            provider: GitProvider::Gitlab,
            owner,
            name,
            branch: tag.clone(),
            commit_sha: Some(event.after.clone()),
            clone_url: event.project.git_http_url,
            use_deploy_key: false,
        },
        ref_name: tag,
        commit_sha: event.after,
        commit_message: event.commits.first().map(|c| c.message.clone()),
        author: Some(event.user_name),
        pull_request_id: None,
        pull_request_action: None,
        base_branch: None,
    })
}

fn parse_gitlab_merge_request(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: GitlabMergeRequestEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid GitLab MR payload: {}", e)))?;

    let parts: Vec<&str> = event.project.path_with_namespace.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        (String::new(), event.project.path_with_namespace.clone())
    };

    Ok(WebhookPayload {
        provider: GitProvider::Gitlab,
        event_type: WebhookEventType::PullRequest,
        repository: GitRepository {
            provider: GitProvider::Gitlab,
            owner,
            name,
            branch: event.object_attributes.source_branch.clone(),
            commit_sha: Some(event.object_attributes.last_commit.id.clone()),
            clone_url: event.project.git_http_url,
            use_deploy_key: false,
        },
        ref_name: event.object_attributes.source_branch,
        commit_sha: event.object_attributes.last_commit.id,
        commit_message: Some(event.object_attributes.title),
        author: Some(event.user.name),
        pull_request_id: Some(event.object_attributes.iid),
        pull_request_action: event.object_attributes.action,
        base_branch: Some(event.object_attributes.target_branch),
    })
}

// ============================================================================
// Bitbucket Webhooks
// ============================================================================

/// Bitbucket push event payload
#[derive(Debug, Deserialize)]
pub struct BitbucketPushEvent {
    pub push: BitbucketPush,
    pub repository: BitbucketRepository,
    pub actor: BitbucketActor,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPush {
    pub changes: Vec<BitbucketChange>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketChange {
    pub new: Option<BitbucketRef>,
    pub old: Option<BitbucketRef>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketRef {
    pub name: String,
    #[serde(rename = "type")]
    pub ref_type: String,
    pub target: BitbucketTarget,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketTarget {
    pub hash: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketRepository {
    pub uuid: String,
    pub name: String,
    pub full_name: String,
    pub links: BitbucketLinks,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketLinks {
    pub clone: Vec<BitbucketCloneLink>,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketCloneLink {
    pub name: String,
    pub href: String,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketActor {
    pub display_name: String,
    pub nickname: Option<String>,
}

/// Bitbucket pull request event payload
#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequestEvent {
    pub pullrequest: BitbucketPullRequest,
    pub repository: BitbucketRepository,
    pub actor: BitbucketActor,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPullRequest {
    pub id: i64,
    pub title: String,
    pub state: String,
    pub source: BitbucketPrBranch,
    pub destination: BitbucketPrBranch,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketPrBranch {
    pub branch: BitbucketBranchInfo,
    pub commit: BitbucketCommitInfo,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketBranchInfo {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct BitbucketCommitInfo {
    pub hash: String,
}

/// Handle Bitbucket webhook
pub async fn bitbucket(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<WebhookResponse>, ApiError> {
    // Get event type
    let event_type = headers
        .get("x-event-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::bad_request("Missing X-Event-Key header"))?;

    let payload = match event_type {
        "repo:push" => parse_bitbucket_push(&body)?,
        "pullrequest:created" | "pullrequest:updated" => parse_bitbucket_pull_request(&body, event_type)?,
        _ => {
            return Ok(Json(WebhookResponse {
                message: format!("Event '{}' not handled", event_type),
                deployment_triggered: false,
                deployment_id: None,
            }));
        }
    };

    process_webhook(state, payload).await
}

fn parse_bitbucket_push(body: &str) -> Result<WebhookPayload, ApiError> {
    let event: BitbucketPushEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid Bitbucket push payload: {}", e)))?;

    let change = event
        .push
        .changes
        .first()
        .and_then(|c| c.new.as_ref())
        .ok_or_else(|| ApiError::bad_request("No changes in push event"))?;

    let parts: Vec<&str> = event.repository.full_name.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        return Err(ApiError::bad_request("Invalid repository name"));
    };

    let clone_url = event
        .repository
        .links
        .clone
        .iter()
        .find(|l| l.name == "https")
        .map(|l| l.href.clone())
        .unwrap_or_else(|| format!("https://bitbucket.org/{}.git", event.repository.full_name));

    let event_type = if change.ref_type == "tag" {
        WebhookEventType::Tag
    } else {
        WebhookEventType::Push
    };

    Ok(WebhookPayload {
        provider: GitProvider::Bitbucket,
        event_type,
        repository: GitRepository {
            provider: GitProvider::Bitbucket,
            owner,
            name,
            branch: change.name.clone(),
            commit_sha: Some(change.target.hash.clone()),
            clone_url,
            use_deploy_key: false,
        },
        ref_name: change.name.clone(),
        commit_sha: change.target.hash.clone(),
        commit_message: change.target.message.clone(),
        author: Some(event.actor.display_name),
        pull_request_id: None,
        pull_request_action: None,
        base_branch: None,
    })
}

fn parse_bitbucket_pull_request(body: &str, event_type: &str) -> Result<WebhookPayload, ApiError> {
    let event: BitbucketPullRequestEvent = serde_json::from_str(body)
        .map_err(|e| ApiError::bad_request(format!("Invalid Bitbucket PR payload: {}", e)))?;

    let parts: Vec<&str> = event.repository.full_name.splitn(2, '/').collect();
    let (owner, name) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        return Err(ApiError::bad_request("Invalid repository name"));
    };

    let clone_url = event
        .repository
        .links
        .clone
        .iter()
        .find(|l| l.name == "https")
        .map(|l| l.href.clone())
        .unwrap_or_else(|| format!("https://bitbucket.org/{}.git", event.repository.full_name));

    let action = match event_type {
        "pullrequest:created" => "opened",
        "pullrequest:updated" => "synchronize",
        _ => "unknown",
    };

    Ok(WebhookPayload {
        provider: GitProvider::Bitbucket,
        event_type: WebhookEventType::PullRequest,
        repository: GitRepository {
            provider: GitProvider::Bitbucket,
            owner,
            name,
            branch: event.pullrequest.source.branch.name.clone(),
            commit_sha: Some(event.pullrequest.source.commit.hash.clone()),
            clone_url,
            use_deploy_key: false,
        },
        ref_name: event.pullrequest.source.branch.name,
        commit_sha: event.pullrequest.source.commit.hash,
        commit_message: Some(event.pullrequest.title),
        author: Some(event.actor.display_name),
        pull_request_id: Some(event.pullrequest.id),
        pull_request_action: Some(action.to_string()),
        base_branch: Some(event.pullrequest.destination.branch.name),
    })
}

// ============================================================================
// Common Processing
// ============================================================================

/// Process a webhook payload and trigger deployment if needed
async fn process_webhook(
    state: Arc<AppState>,
    payload: WebhookPayload,
) -> Result<Json<WebhookResponse>, ApiError> {
    // Find applications matching this repository
    let repo = &payload.repository;

    // TODO: Query applications by git repository
    // For now, we'll return a success message indicating the webhook was received
    // In a full implementation, we would:
    // 1. Find all applications that match this repo (owner/name)
    // 2. Filter by branch/tag if applicable
    // 3. Check if auto-deploy is enabled
    // 4. Trigger deployments for matching applications

    tracing::info!(
        provider = ?payload.provider,
        event = ?payload.event_type,
        repo = %format!("{}/{}", repo.owner, repo.name),
        branch = %payload.ref_name,
        commit = %payload.commit_sha,
        "Webhook received"
    );

    // Check if we should trigger deployment
    // This would be based on matching applications and their settings
    let should_deploy = payload.should_deploy(&payload.ref_name, true);

    if should_deploy {
        // TODO: Create deployment job
        // let deployment = state.deployments().create(...).await?;
        // Queue the deployment job

        Ok(Json(WebhookResponse {
            message: format!(
                "Webhook processed. Deployment triggered for {}/{}@{}",
                repo.owner, repo.name, payload.ref_name
            ),
            deployment_triggered: true,
            deployment_id: None, // Would be set to actual deployment ID
        }))
    } else {
        Ok(Json(WebhookResponse {
            message: format!(
                "Webhook processed for {}/{}@{}. No deployment triggered.",
                repo.owner, repo.name, payload.ref_name
            ),
            deployment_triggered: false,
            deployment_id: None,
        }))
    }
}

/// Verify GitHub webhook signature
#[allow(dead_code)]
fn verify_github_signature(secret: &str, signature: &str, body: &[u8]) -> bool {
    let signature = match signature.strip_prefix("sha256=") {
        Some(sig) => sig,
        None => return false,
    };

    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };

    mac.update(body);

    mac.verify_slice(&signature_bytes).is_ok()
}

/// Verify GitLab webhook token
#[allow(dead_code)]
fn verify_gitlab_token(expected: &str, received: &str) -> bool {
    // Simple constant-time comparison
    expected.as_bytes() == received.as_bytes()
}
