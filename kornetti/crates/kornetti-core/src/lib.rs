//! Kornetti Core
//!
//! Core domain models and business logic for the Kornetti deployment platform.
//!
//! # Modules
//!
//! - `models`: Domain models (Server, Application, Database, Service, etc.)
//! - `actions`: Business action implementations (InstallDocker, StartProxy, etc.)
//! - `repositories`: Database access layer with SQLx
//! - `compose`: Docker Compose file parsing and generation
//! - `security`: Security utilities (encryption, validation, authentication)
//! - `config`: Configuration management
//! - `error`: Error types and handling
//! - `traits`: Shared traits

pub mod models;
pub mod error;
pub mod config;
pub mod traits;
pub mod repositories;
pub mod actions;
pub mod compose;
pub mod security;

pub use error::{Error, Result};
pub use config::Config;
pub use repositories::Database;
pub use compose::{ComposeFile, Service as ComposeService, ComposeBuilder};
pub use security::{Encryptor, EncryptionKey, validate_shell_safe_path, sanitize_string};
