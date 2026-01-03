//! Kornetti API
//!
//! REST API server for the Kornetti deployment platform.

pub mod routes;
pub mod handlers;
pub mod middleware;
pub mod auth;
pub mod state;
pub mod error;

use axum::Router;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer, compression::CompressionLayer};

pub use state::AppState;
pub use error::ApiError;

/// Build the API router with all routes and middleware
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/v1", routes::api_routes())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
