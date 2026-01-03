//! Kornetti Core
//!
//! Core domain models and business logic for the Kornetti deployment platform.

pub mod models;
pub mod error;
pub mod config;
pub mod traits;

pub use error::{Error, Result};
pub use config::Config;
