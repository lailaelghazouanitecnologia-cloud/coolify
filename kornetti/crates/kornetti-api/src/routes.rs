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

        // Tags
        .route("/tags", get(handlers::tags::list))
        .route("/tags", post(handlers::tags::create))
        .route("/tags/:id", get(handlers::tags::get))
        .route("/tags/:id", put(handlers::tags::update))
        .route("/tags/:id", delete(handlers::tags::delete))
        .route("/tags/:id/attach", post(handlers::tags::attach))
        .route("/tags/:id/detach/:resource_type/:resource_id", delete(handlers::tags::detach))
        .route("/tags/:id/resources", get(handlers::tags::resources))

        // Backups
        .route("/backups", get(handlers::backups::list))
        .route("/backups", post(handlers::backups::create))
        .route("/backups/:id", get(handlers::backups::get))
        .route("/backups/:id", put(handlers::backups::update))
        .route("/backups/:id", delete(handlers::backups::delete))
        .route("/backups/:id/run", post(handlers::backups::run))
        .route("/backups/:id/executions", get(handlers::backups::executions))
        .route("/backups/:backup_id/executions/:execution_id", get(handlers::backups::get_execution))
        .route("/backups/:backup_id/executions/:execution_id/download", get(handlers::backups::download))
        .route("/backups/:backup_id/executions/:execution_id/restore", post(handlers::backups::restore))

        // Team Invitations
        .route("/team/invitations", get(handlers::team_invitations::list))
        .route("/team/invitations", post(handlers::team_invitations::create))
        .route("/team/invitations/:id", get(handlers::team_invitations::get))
        .route("/team/invitations/:id/resend", post(handlers::team_invitations::resend))
        .route("/team/invitations/:id", delete(handlers::team_invitations::cancel))
        .route("/team/members", get(handlers::team_invitations::list_members))
        .route("/team/members/:user_id", put(handlers::team_invitations::update_member))
        .route("/team/members/:user_id", delete(handlers::team_invitations::remove_member))

        // Settings
        .route("/settings", get(handlers::settings::get_instance))
        .route("/settings", put(handlers::settings::update_instance))
        .route("/settings/s3", get(handlers::settings::list_s3_storages))
        .route("/settings/s3", post(handlers::settings::create_s3_storage))
        .route("/settings/s3/:id", get(handlers::settings::get_s3_storage))
        .route("/settings/s3/:id", put(handlers::settings::update_s3_storage))
        .route("/settings/s3/:id", delete(handlers::settings::delete_s3_storage))
        .route("/settings/s3/:id/test", post(handlers::settings::test_s3_storage))
        .route("/settings/license", get(handlers::settings::get_license))
        .route("/settings/license", put(handlers::settings::update_license))
        .route("/settings/status", get(handlers::settings::get_system_status))
        .route("/settings/cleanup", post(handlers::settings::trigger_cleanup))

        // GitHub Apps
        .route("/github-apps", get(handlers::github_apps::list))
        .route("/github-apps", post(handlers::github_apps::create))
        .route("/github-apps/:id", get(handlers::github_apps::get))
        .route("/github-apps/:id", put(handlers::github_apps::update))
        .route("/github-apps/:id", delete(handlers::github_apps::delete))
        .route("/github-apps/:id/repositories", get(handlers::github_apps::repositories))
        .route("/github-apps/:id/repositories/:owner/:repo/branches", get(handlers::github_apps::branches))
        .route("/github-apps/:id/installation-url", get(handlers::github_apps::installation_url))
        .route("/github-apps/:id/installation-callback", get(handlers::github_apps::installation_callback))
        .route("/github-apps/:id/refresh-token", post(handlers::github_apps::refresh_token))
        .route("/github-apps/:id/check-permissions", get(handlers::github_apps::check_permissions))

        // Cloud Provider Tokens
        .route("/cloud-tokens", get(handlers::cloud_providers::list))
        .route("/cloud-tokens", post(handlers::cloud_providers::create))
        .route("/cloud-tokens/:id", get(handlers::cloud_providers::get))
        .route("/cloud-tokens/:id", put(handlers::cloud_providers::update))
        .route("/cloud-tokens/:id", delete(handlers::cloud_providers::delete))
        .route("/cloud-tokens/:id/validate", post(handlers::cloud_providers::validate))

        // Hetzner-specific endpoints
        .route("/cloud-tokens/:id/hetzner/locations", get(handlers::cloud_providers::hetzner_locations))
        .route("/cloud-tokens/:id/hetzner/server-types", get(handlers::cloud_providers::hetzner_server_types))
        .route("/cloud-tokens/:id/hetzner/images", get(handlers::cloud_providers::hetzner_images))
        .route("/cloud-tokens/:id/hetzner/ssh-keys", get(handlers::cloud_providers::hetzner_ssh_keys))
        .route("/cloud-tokens/:id/hetzner/servers", post(handlers::cloud_providers::create_hetzner_server))
        .route("/cloud-tokens/:id/hetzner/servers/:server_id", delete(handlers::cloud_providers::delete_hetzner_server))

        // OAuth Settings
        .route("/oauth", get(handlers::oauth::list))
        .route("/oauth", put(handlers::oauth::update_all))
        .route("/oauth/enabled", get(handlers::oauth::any_enabled))
        .route("/oauth/available", get(handlers::oauth::available_providers))
        .route("/oauth/:provider", get(handlers::oauth::get))
        .route("/oauth/:provider", put(handlers::oauth::update))
        .route("/oauth/:provider/redirect", get(handlers::oauth::redirect_url))
        .route("/oauth/:provider/callback", get(handlers::oauth::callback))

        // Resources (cross-type resource listing)
        .route("/resources", get(handlers::resources::list))
        .route("/resources/stats", get(handlers::resources::stats))
        .route("/resources/:uuid", get(handlers::resources::get_by_uuid))

        // Destinations (Docker networks)
        .route("/destinations", get(handlers::destinations::list))
        .route("/destinations", post(handlers::destinations::create))
        .route("/destinations/:id", get(handlers::destinations::get))
        .route("/destinations/:id", put(handlers::destinations::update))
        .route("/destinations/:id", delete(handlers::destinations::delete))
        .route("/destinations/:id/verify", post(handlers::destinations::verify))
        .route("/destinations/:id/recreate", post(handlers::destinations::recreate))
        // Swarm-specific destination endpoints
        .route("/destinations/:id/swarm/nodes", get(handlers::destinations::swarm_nodes))
        .route("/destinations/:id/swarm/services", get(handlers::destinations::swarm_services))
        .route("/destinations/:id/swarm/init", post(handlers::destinations::swarm_init))
        .route("/destinations/:id/swarm/tokens", get(handlers::destinations::swarm_tokens))
        .route("/destinations/:id/swarm/leave", post(handlers::destinations::swarm_leave))

        // Deploy API (webhook-style deployments)
        .route("/deploy", get(handlers::deploy::deploy_by_uuid))
        .route("/deploy/bulk", post(handlers::deploy::bulk_deploy))
}

/// Webhook routes that don't require authentication
pub fn webhook_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/source/github/events", post(handlers::webhooks::github))
        .route("/source/github/events/:app_id", post(handlers::webhooks::github_app))
        .route("/source/gitlab/events", post(handlers::webhooks::gitlab))
        .route("/source/bitbucket/events", post(handlers::webhooks::bitbucket))
}

/// Public routes that don't require authentication
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Invitation acceptance/decline (public links in emails)
        .route("/invitations/:token/accept", post(handlers::team_invitations::accept))
        .route("/invitations/:token/decline", post(handlers::team_invitations::decline))
        // Health check
        .route("/health", get(handlers::health::health_check))
}
