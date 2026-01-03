//! Kornetti Core
//!
//! Core domain models and business logic for the Kornetti deployment platform.

pub mod models;
pub mod error;
pub mod config;
pub mod traits;
pub mod repositories;

pub use error::{Error, Result};
pub use config::Config;
pub use repositories::Database;
