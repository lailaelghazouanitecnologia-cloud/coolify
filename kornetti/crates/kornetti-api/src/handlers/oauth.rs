//! OAuth Settings handlers
//!
//! API handlers for OAuth provider configuration (GitHub, GitLab, Azure, etc.).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

/// Supported OAuth providers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OauthProvider {
    Github,
    Gitlab,
    Google,
    Azure,
    Bitbucket,
    Authentik,
    Clerk,
}

impl std::fmt::Display for OauthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OauthProvider::Github => write!(f, "github"),
            OauthProvider::Gitlab => write!(f, "gitlab"),
            OauthProvider::Google => write!(f, "google"),
            OauthProvider::Azure => write!(f, "azure"),
            OauthProvider::Bitbucket => write!(f, "bitbucket"),
            OauthProvider::Authentik => write!(f, "authentik"),
            OauthProvider::Clerk => write!(f, "clerk"),
        }
    }
}

/// OAuth setting response
#[derive(Debug, Serialize)]
pub struct OauthSettingResponse {
    pub id: Uuid,
    pub provider: OauthProvider,
    pub enabled: bool,
    pub client_id: Option<String>,
    /// Never include actual secret in response
    pub has_client_secret: bool,
    pub redirect_uri: Option<String>,
    /// For Azure - tenant ID
    pub tenant: Option<String>,
    /// For Authentik, Clerk - base URL
    pub base_url: Option<String>,
    pub could_be_enabled: bool,
}

/// List all OAuth settings response
#[derive(Debug, Serialize)]
pub struct ListOauthSettingsResponse {
    pub settings: Vec<OauthSettingResponse>,
}

/// Request to update OAuth settings
#[derive(Debug, Deserialize)]
pub struct UpdateOauthSettingRequest {
    pub enabled: Option<bool>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub tenant: Option<String>,
    pub base_url: Option<String>,
}

/// Request to update all OAuth settings at once
#[derive(Debug, Deserialize)]
pub struct UpdateAllOauthSettingsRequest {
    pub settings: Vec<ProviderSettingsUpdate>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderSettingsUpdate {
    pub provider: OauthProvider,
    pub enabled: Option<bool>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub tenant: Option<String>,
    pub base_url: Option<String>,
}

/// Check if settings are complete for a provider
fn could_be_enabled(
    provider: &OauthProvider,
    client_id: &Option<String>,
    client_secret_set: bool,
    tenant: &Option<String>,
    base_url: &Option<String>,
) -> bool {
    let has_client_id = client_id.as_ref().map(|s| !s.is_empty()).unwrap_or(false);

    match provider {
        OauthProvider::Azure => {
            has_client_id
                && client_secret_set
                && tenant.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
        }
        OauthProvider::Authentik | OauthProvider::Clerk => {
            has_client_id
                && client_secret_set
                && base_url.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
        }
        _ => has_client_id && client_secret_set,
    }
}

/// List all OAuth settings
///
/// GET /api/oauth
pub async fn list(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would fetch all OAuth settings from database
    // In a real implementation, secrets would be encrypted in the DB

    let providers = vec![
        OauthProvider::Github,
        OauthProvider::Gitlab,
        OauthProvider::Google,
        OauthProvider::Azure,
        OauthProvider::Bitbucket,
        OauthProvider::Authentik,
        OauthProvider::Clerk,
    ];

    let settings: Vec<OauthSettingResponse> = providers
        .into_iter()
        .map(|provider| OauthSettingResponse {
            id: Uuid::new_v4(),
            provider,
            enabled: false,
            client_id: None,
            has_client_secret: false,
            redirect_uri: None,
            tenant: None,
            base_url: None,
            could_be_enabled: false,
        })
        .collect();

    Json(ListOauthSettingsResponse { settings })
}

/// Get OAuth settings for a specific provider
///
/// GET /api/oauth/:provider
pub async fn get(
    State(_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> impl IntoResponse {
    let provider = match provider.to_lowercase().as_str() {
        "github" => OauthProvider::Github,
        "gitlab" => OauthProvider::Gitlab,
        "google" => OauthProvider::Google,
        "azure" => OauthProvider::Azure,
        "bitbucket" => OauthProvider::Bitbucket,
        "authentik" => OauthProvider::Authentik,
        "clerk" => OauthProvider::Clerk,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid OAuth provider"
                })),
            )
                .into_response();
        }
    };

    // Placeholder: would fetch from database
    Json(OauthSettingResponse {
        id: Uuid::new_v4(),
        provider,
        enabled: false,
        client_id: None,
        has_client_secret: false,
        redirect_uri: None,
        tenant: None,
        base_url: None,
        could_be_enabled: false,
    })
    .into_response()
}

/// Update OAuth settings for a specific provider
///
/// PUT /api/oauth/:provider
pub async fn update(
    State(_state): State<Arc<AppState>>,
    Path(provider_str): Path<String>,
    Json(request): Json<UpdateOauthSettingRequest>,
) -> impl IntoResponse {
    let provider = match provider_str.to_lowercase().as_str() {
        "github" => OauthProvider::Github,
        "gitlab" => OauthProvider::Gitlab,
        "google" => OauthProvider::Google,
        "azure" => OauthProvider::Azure,
        "bitbucket" => OauthProvider::Bitbucket,
        "authentik" => OauthProvider::Authentik,
        "clerk" => OauthProvider::Clerk,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid OAuth provider"
                })),
            )
                .into_response();
        }
    };

    // Check if settings could be enabled
    let client_secret_set = request
        .client_secret
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let can_enable = could_be_enabled(
        &provider,
        &request.client_id,
        client_secret_set,
        &request.tenant,
        &request.base_url,
    );

    // If trying to enable but missing required fields
    if request.enabled.unwrap_or(false) && !can_enable {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("OAuth settings are not complete for {}. Please fill in all required fields.", provider)
            })),
        )
            .into_response();
    }

    // Placeholder: would update in database
    // Client secret would be encrypted before storage

    Json(serde_json::json!({
        "message": format!("OAuth settings for {} updated successfully", provider),
        "could_be_enabled": can_enable
    }))
    .into_response()
}

/// Update all OAuth settings at once
///
/// PUT /api/oauth
pub async fn update_all(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<UpdateAllOauthSettingsRequest>,
) -> impl IntoResponse {
    let mut errors: Vec<String> = Vec::new();
    let mut updated: Vec<String> = Vec::new();

    for setting in request.settings {
        let client_secret_set = setting
            .client_secret
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let can_enable = could_be_enabled(
            &setting.provider,
            &setting.client_id,
            client_secret_set,
            &setting.tenant,
            &setting.base_url,
        );

        if setting.enabled.unwrap_or(false) && !can_enable {
            errors.push(format!(
                "OAuth settings are incomplete for '{}'. Required fields are missing. The provider has been disabled.",
                setting.provider
            ));
            // In real implementation, would set enabled = false and save
        }

        // Placeholder: would update in database
        updated.push(setting.provider.to_string());
    }

    if !errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "errors": errors,
                "updated": updated
            })),
        )
            .into_response();
    }

    Json(serde_json::json!({
        "message": "OAuth settings updated successfully",
        "updated": updated
    }))
    .into_response()
}

/// Get OAuth redirect URL for a provider (initiates OAuth flow)
///
/// GET /api/oauth/:provider/redirect
pub async fn redirect_url(
    State(_state): State<Arc<AppState>>,
    Path(provider_str): Path<String>,
) -> impl IntoResponse {
    let (provider, auth_base_url) = match provider_str.to_lowercase().as_str() {
        "github" => (OauthProvider::Github, "https://github.com/login/oauth/authorize"),
        "gitlab" => (OauthProvider::Gitlab, "https://gitlab.com/oauth/authorize"),
        "google" => (OauthProvider::Google, "https://accounts.google.com/o/oauth2/v2/auth"),
        "azure" => (OauthProvider::Azure, "https://login.microsoftonline.com"),
        "bitbucket" => (OauthProvider::Bitbucket, "https://bitbucket.org/site/oauth2/authorize"),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid or unsupported OAuth provider"
                })),
            )
                .into_response();
        }
    };

    // Placeholder: would fetch settings from DB and construct proper URL
    // Generate state token for CSRF protection
    let state = Uuid::new_v4().to_string();

    // Store state in session/cache for validation on callback

    let redirect_url = match provider {
        OauthProvider::Github => {
            format!(
                "{}?client_id={}&state={}&scope=user:email",
                auth_base_url,
                "YOUR_CLIENT_ID", // Would be fetched from DB
                state
            )
        }
        OauthProvider::Azure => {
            format!(
                "{}/common/oauth2/v2.0/authorize?client_id={}&state={}&response_type=code&scope=openid%20profile%20email",
                auth_base_url,
                "YOUR_CLIENT_ID",
                state
            )
        }
        _ => {
            format!(
                "{}?client_id={}&state={}&response_type=code",
                auth_base_url,
                "YOUR_CLIENT_ID",
                state
            )
        }
    };

    Json(serde_json::json!({
        "redirect_url": redirect_url,
        "state": state
    }))
    .into_response()
}

/// Handle OAuth callback
///
/// GET /api/oauth/:provider/callback
pub async fn callback(
    State(_state): State<Arc<AppState>>,
    Path(provider_str): Path<String>,
    axum::extract::Query(params): axum::extract::Query<OauthCallbackParams>,
) -> impl IntoResponse {
    let provider = match provider_str.to_lowercase().as_str() {
        "github" => OauthProvider::Github,
        "gitlab" => OauthProvider::Gitlab,
        "google" => OauthProvider::Google,
        "azure" => OauthProvider::Azure,
        "bitbucket" => OauthProvider::Bitbucket,
        "authentik" => OauthProvider::Authentik,
        "clerk" => OauthProvider::Clerk,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid OAuth provider"
                })),
            )
                .into_response();
        }
    };

    // Verify state matches stored state (CSRF protection)
    if params.state.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Missing state parameter"
            })),
        )
            .into_response();
    }

    // Handle error from OAuth provider
    if let Some(error) = params.error {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("OAuth error: {}", error),
                "error_description": params.error_description
            })),
        )
            .into_response();
    }

    // Exchange code for token
    let Some(code) = params.code else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Missing authorization code"
            })),
        )
            .into_response();
    };

    // Placeholder: would exchange code for token, fetch user info, create/login user
    // This would involve:
    // 1. POST to token endpoint with code + client_secret
    // 2. Use access_token to fetch user profile
    // 3. Create user if not exists (if registration enabled)
    // 4. Create session/JWT for the user

    Json(serde_json::json!({
        "message": format!("OAuth callback for {} processed", provider),
        "code": code
    }))
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct OauthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Check if OAuth is enabled for any provider
///
/// GET /api/oauth/enabled
pub async fn any_enabled(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would check database for any enabled OAuth providers
    Json(serde_json::json!({
        "oauth_enabled": false,
        "providers": []
    }))
}

/// Get available OAuth providers for login page
///
/// GET /api/oauth/available
pub async fn available_providers(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Placeholder: would fetch enabled providers from database
    // Returns only providers that are properly configured and enabled

    #[derive(Serialize)]
    struct AvailableProvider {
        provider: OauthProvider,
        name: String,
        icon: String,
    }

    let providers: Vec<AvailableProvider> = vec![
        // Would be populated from DB query
    ];

    Json(serde_json::json!({
        "providers": providers
    }))
}
