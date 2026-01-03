//! API middleware
//!
//! Authentication and authorization middleware matching Coolify patterns:
//! - Bearer token (JWT) authentication
//! - Personal Access Token (Sanctum-like) authentication
//! - IP allowlist checking
//! - Team context extraction
//! - Request logging

use axum::{
    body::Body,
    extract::{Request, State, ConnectInfo},
    http::{header, StatusCode},
    middleware::Next,
    response::{Response, IntoResponse},
    Json,
};
use serde_json::json;
use std::net::SocketAddr;
use uuid::Uuid;
use tracing::{info, warn};

use crate::auth::{JwtConfig, Claims};
use crate::state::{AppState, AuthContext};
use kornetti_core::security::check_ip_against_allowlist;

/// Authentication error response
fn auth_error(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "error": "Unauthorized",
            "message": message
        }))
    ).into_response()
}

/// Forbidden error response
fn forbidden_error(message: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "Forbidden",
            "message": message
        }))
    ).into_response()
}

/// Authentication middleware
///
/// Validates Bearer token (JWT) or Personal Access Token from Authorization header.
/// Extracts user context and adds it to request extensions.
///
/// Matches Coolify's middleware pattern from `app/Http/Middleware/ApiAllowed.php`
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let auth_result = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            validate_bearer_token(&state, token).await
        }
        Some(header) => {
            // Try as a Personal Access Token (API token)
            validate_api_token(&state, header).await
        }
        None => {
            return auth_error("Authorization header required");
        }
    };

    match auth_result {
        Ok(auth_context) => {
            // Add auth context to request extensions
            request.extensions_mut().insert(auth_context);
            next.run(request).await
        }
        Err(message) => auth_error(&message),
    }
}

/// Validate JWT bearer token
async fn validate_bearer_token(state: &AppState, token: &str) -> Result<AuthContext, String> {
    let jwt_config = JwtConfig {
        secret: state.config.jwt_secret.clone(),
        expiration_hours: 24, // Default expiration
    };

    let claims = jwt_config
        .verify_token(token)
        .map_err(|_| "Invalid or expired token".to_string())?;

    // Fetch user details from database
    let user = state
        .users()
        .find_by_id(claims.sub)
        .await
        .map_err(|_| "User not found".to_string())?
        .ok_or_else(|| "User not found".to_string())?;

    Ok(AuthContext {
        user_id: claims.sub,
        team_id: claims.team_id,
        email: user.email,
        is_admin: user.is_admin,
    })
}

/// Validate Personal Access Token (similar to Laravel Sanctum)
async fn validate_api_token(state: &AppState, token: &str) -> Result<AuthContext, String> {
    // Hash the token for lookup (tokens are stored as hashes)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    // Look up token in database
    let result = sqlx::query!(
        r#"
        SELECT
            pat.id,
            pat.user_id,
            pat.team_id,
            pat.abilities,
            pat.expires_at,
            u.email,
            u.is_admin
        FROM personal_access_tokens pat
        JOIN users u ON pat.user_id = u.id
        WHERE pat.token = $1
        "#,
        token_hash
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| "Token validation failed".to_string())?
    .ok_or_else(|| "Invalid token".to_string())?;

    // Check expiration
    if let Some(expires_at) = result.expires_at {
        if expires_at < chrono::Utc::now() {
            return Err("Token expired".to_string());
        }
    }

    // Update last_used_at
    let _ = sqlx::query!(
        "UPDATE personal_access_tokens SET last_used_at = NOW() WHERE id = $1",
        result.id
    )
    .execute(&state.db)
    .await;

    Ok(AuthContext {
        user_id: result.user_id,
        team_id: result.team_id.unwrap_or_default(),
        email: result.email,
        is_admin: result.is_admin,
    })
}

/// IP allowlist middleware
///
/// Checks if the client IP is in the user's API allowlist.
/// Matches Coolify's `checkIPAgainstAllowlist()` function.
pub async fn ip_allowlist_middleware(
    State(state): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request,
    next: Next,
) -> Response {
    let auth_context = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return next.run(request).await, // Skip if not authenticated
    };

    // Get client IP
    let client_ip = get_client_ip(&request, connect_info);

    // Fetch user's IP allowlist
    let allowlist = match fetch_ip_allowlist(&state, auth_context.user_id).await {
        Ok(list) => list,
        Err(_) => return next.run(request).await, // Skip on error
    };

    // Empty allowlist means allow all
    if allowlist.is_empty() {
        return next.run(request).await;
    }

    // Check against allowlist
    if !check_ip_against_allowlist(&client_ip, &allowlist) {
        warn!(
            user_id = %auth_context.user_id,
            ip = %client_ip,
            "API access denied: IP not in allowlist"
        );
        return forbidden_error(&format!("IP {} is not in the allowed list", client_ip));
    }

    next.run(request).await
}

/// Get client IP from request, checking X-Forwarded-For and X-Real-IP headers
fn get_client_ip(request: &Request, connect_info: Option<ConnectInfo<SocketAddr>>) -> String {
    // Check X-Forwarded-For header (when behind proxy)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // Take first IP (client IP)
            if let Some(ip) = forwarded_str.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }

    // Fall back to connection address
    connect_info
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Fetch user's IP allowlist from database
async fn fetch_ip_allowlist(state: &AppState, user_id: Uuid) -> Result<Vec<String>, String> {
    let result = sqlx::query_scalar!(
        r#"SELECT allowed_ips FROM users WHERE id = $1"#,
        user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    match result {
        Some(Some(ips)) => {
            // Parse JSON array of IPs
            serde_json::from_value::<Vec<String>>(ips)
                .map_err(|e| e.to_string())
        }
        _ => Ok(Vec::new()),
    }
}

/// Team context middleware
///
/// Extracts team ID from X-Team-ID header or uses default from auth context.
/// Validates that user belongs to the specified team.
pub async fn team_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_context = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => return next.run(request).await,
    };

    // Check for X-Team-ID header
    let team_id = request
        .headers()
        .get("x-team-id")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(auth_context.team_id);

    // Validate user belongs to team
    let is_member = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM team_members
            WHERE team_id = $1 AND user_id = $2
        ) as "exists!"
        "#,
        team_id,
        auth_context.user_id
    )
    .fetch_one(&state.db)
    .await;

    match is_member {
        Ok(true) => {
            // Update auth context with correct team
            let updated_context = AuthContext {
                team_id,
                ..auth_context
            };
            request.extensions_mut().insert(updated_context);
            next.run(request).await
        }
        _ => forbidden_error("You do not have access to this team"),
    }
}

/// Request logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request processed"
    );

    response
}

/// Admin-only middleware
///
/// Requires authenticated user to be an admin.
pub async fn admin_middleware(request: Request, next: Next) -> Response {
    let auth_context = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx,
        None => return auth_error("Authentication required"),
    };

    if !auth_context.is_admin {
        return forbidden_error("Administrator access required");
    }

    next.run(request).await
}

/// Require specific ability on Personal Access Token
pub fn require_ability(ability: &'static str) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            // For now, pass through - token abilities would be checked here
            // This mirrors Coolify's Sanctum abilities check
            next.run(request).await
        })
    }
}
