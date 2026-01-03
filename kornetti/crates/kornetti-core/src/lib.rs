//! Kornetti Core
//!
//! Core domain models and business logic for the Kornetti deployment platform.
//!
//! # Modules
//!
//! - `models`: Domain models (Server, Application, Database, Service, etc.)
//! - `actions`: Business action implementations (InstallDocker, StartProxy, etc.)
//! - `repositories`: Database access layer with SQLx
//! - `config`: Configuration management
//! - `error`: Error types and handling
//! - `traits`: Shared traits

pub mod models;
pub mod error;
pub mod config;
pub mod traits;
pub mod repositories;
pub mod actions;

pub use error::{Error, Result};
pub use config::Config;
pub use repositories::Database;
