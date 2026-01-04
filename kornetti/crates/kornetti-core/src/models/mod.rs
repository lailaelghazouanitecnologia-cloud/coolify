//! Domain models for Kornetti

// Core models
pub mod server;
pub mod application;
pub mod database;
pub mod service;
pub mod team;
pub mod project;
pub mod environment;
pub mod deployment;
pub mod user;
pub mod private_key;
pub mod environment_variable;
pub mod s3_storage;
pub mod git_source;
pub mod notification;
pub mod scheduled_task;

// Application-related models
pub mod application_deployment_queue;
pub mod application_preview;
pub mod application_setting;
pub mod persistent_volume;
pub mod server_setting;
pub mod ssl_certificate;
pub mod tag;
pub mod team_invitation;
pub mod service_component;
pub mod swarm;

// Destination models
pub mod standalone_docker;
// Note: swarm module already contains SwarmDocker

// Database models
pub mod standalone_database;
pub mod scheduled_database_backup;

// Configuration models
pub mod instance_settings;
pub mod oauth_setting;
pub mod cloud_provider_token;
pub mod github_app;
pub mod gitlab_app;
pub mod notification_settings;
pub mod local_file_volume;

// Execution history models
pub mod execution_history;

// Service sub-component models
pub mod service_application;
pub mod service_database;
pub mod shared_environment_variable;

// Core re-exports
pub use server::*;
pub use application::*;
pub use database::*;
pub use service::*;
pub use team::*;
pub use project::*;
pub use environment::*;
pub use deployment::*;
pub use user::*;
pub use private_key::*;
pub use environment_variable::*;
pub use s3_storage::*;
pub use git_source::*;
pub use notification::*;
pub use scheduled_task::*;

// Application-related re-exports
pub use application_deployment_queue::*;
pub use application_preview::*;
pub use application_setting::*;
pub use persistent_volume::*;
pub use server_setting::*;
pub use ssl_certificate::*;
pub use tag::*;
pub use team_invitation::*;
pub use service_component::*;
pub use swarm::*;

// Destination re-exports
pub use standalone_docker::*;

// Database re-exports
pub use standalone_database::*;
pub use scheduled_database_backup::*;

// Configuration re-exports
pub use instance_settings::*;
pub use oauth_setting::*;
pub use cloud_provider_token::*;
pub use github_app::*;
pub use gitlab_app::*;
pub use notification_settings::*;
pub use local_file_volume::*;

// Execution history re-exports
pub use execution_history::*;

// Service sub-component re-exports
pub use service_application::*;
pub use service_database::*;
pub use shared_environment_variable::*;
