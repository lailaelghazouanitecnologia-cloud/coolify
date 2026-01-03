//! API middleware

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// Authentication middleware
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    // TODO: Implement JWT verification
    next.run(request).await
}

/// Team context middleware
pub async fn team_middleware(request: Request, next: Next) -> Response {
    // TODO: Extract team from header
    next.run(request).await
}
