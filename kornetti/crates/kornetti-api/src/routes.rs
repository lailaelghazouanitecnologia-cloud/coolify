//! API routes

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use std::sync::Arc;
use crate::{handlers, state::AppState};

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Health check
        .route("/health", get(handlers::health::health_check))

        // Auth
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))

        // Teams
        .route("/teams", get(handlers::teams::list))
        .route("/teams", post(handlers::teams::create))
        .route("/teams/:id", get(handlers::teams::get))
        .route("/teams/:id", put(handlers::teams::update))
        .route("/teams/:id", delete(handlers::teams::delete))

        // Servers
        .route("/servers", get(handlers::servers::list))
        .route("/servers", post(handlers::servers::create))
        .route("/servers/:id", get(handlers::servers::get))
        .route("/servers/:id", put(handlers::servers::update))
        .route("/servers/:id", delete(handlers::servers::delete))
        .route("/servers/:id/validate", post(handlers::servers::validate))
        .route("/servers/:id/install-docker", post(handlers::servers::install_docker))
        .route("/servers/:id/resources", get(handlers::servers::resources))

        // Projects
        .route("/projects", get(handlers::projects::list))
        .route("/projects", post(handlers::projects::create))
        .route("/projects/:id", get(handlers::projects::get))
        .route("/projects/:id", put(handlers::projects::update))
        .route("/projects/:id", delete(handlers::projects::delete))

        // Applications
        .route("/applications", get(handlers::applications::list))
        .route("/applications", post(handlers::applications::create))
        .route("/applications/:id", get(handlers::applications::get))
        .route("/applications/:id", put(handlers::applications::update))
        .route("/applications/:id", delete(handlers::applications::delete))
        .route("/applications/:id/deploy", post(handlers::applications::deploy))
        .route("/applications/:id/stop", post(handlers::applications::stop))
        .route("/applications/:id/restart", post(handlers::applications::restart))

        // Deployments
        .route("/deployments", get(handlers::deployments::list))
        .route("/deployments/:id", get(handlers::deployments::get))
        .route("/deployments/:id/logs", get(handlers::deployments::logs))
        .route("/deployments/:id/cancel", post(handlers::deployments::cancel))

        // Databases
        .route("/databases", get(handlers::databases::list))
        .route("/databases", post(handlers::databases::create))
        .route("/databases/:id", get(handlers::databases::get))
        .route("/databases/:id", delete(handlers::databases::delete))
        .route("/databases/:id/start", post(handlers::databases::start))
        .route("/databases/:id/stop", post(handlers::databases::stop))
}
