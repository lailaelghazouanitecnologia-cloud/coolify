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

        // Services
        .route("/services", get(handlers::services::list))
        .route("/services", post(handlers::services::create))
        .route("/services/:id", get(handlers::services::get))
        .route("/services/:id", put(handlers::services::update))
        .route("/services/:id", delete(handlers::services::delete))
        .route("/services/:id/start", post(handlers::services::start))
        .route("/services/:id/stop", post(handlers::services::stop))
        .route("/services/:id/restart", post(handlers::services::restart))
        .route("/services/:id/deploy", post(handlers::services::deploy))

        // Environment Variables
        .route("/environment-variables", get(handlers::environment_variables::list))
        .route("/environment-variables", post(handlers::environment_variables::create))
        .route("/environment-variables/bulk", post(handlers::environment_variables::bulk_create))
        .route("/environment-variables/:id", get(handlers::environment_variables::get))
        .route("/environment-variables/:id", put(handlers::environment_variables::update))
        .route("/environment-variables/:id", delete(handlers::environment_variables::delete))
        .route("/environment-variables/:resource_type/:resource_id", delete(handlers::environment_variables::delete_for_resource))

        // Private Keys
        .route("/private-keys", get(handlers::private_keys::list))
        .route("/private-keys", post(handlers::private_keys::create))
        .route("/private-keys/generate", post(handlers::private_keys::generate))
        .route("/private-keys/:id", get(handlers::private_keys::get))
        .route("/private-keys/:id", put(handlers::private_keys::update))
        .route("/private-keys/:id", delete(handlers::private_keys::delete))
        .route("/private-keys/:id/public-key", get(handlers::private_keys::get_public_key))

        // Notifications
        .route("/notifications", get(handlers::notifications::list))
        .route("/notifications", post(handlers::notifications::create))
        .route("/notifications/:id", get(handlers::notifications::get))
        .route("/notifications/:id", put(handlers::notifications::update))
        .route("/notifications/:id", delete(handlers::notifications::delete))
        .route("/notifications/:id/enable", post(handlers::notifications::enable))
        .route("/notifications/:id/disable", post(handlers::notifications::disable))
        .route("/notifications/:id/test", post(handlers::notifications::test))

        // Webhooks
        .route("/webhooks/github", post(handlers::webhooks::github))
        .route("/webhooks/github/:app_id", post(handlers::webhooks::github_app))
        .route("/webhooks/gitlab", post(handlers::webhooks::gitlab))
        .route("/webhooks/bitbucket", post(handlers::webhooks::bitbucket))
}

/// Webhook routes that don't require authentication
pub fn webhook_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/source/github/events", post(handlers::webhooks::github))
        .route("/source/github/events/:app_id", post(handlers::webhooks::github_app))
        .route("/source/gitlab/events", post(handlers::webhooks::gitlab))
        .route("/source/bitbucket/events", post(handlers::webhooks::bitbucket))
}
