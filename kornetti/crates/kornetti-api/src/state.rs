//! Application state

use kornetti_core::Config;
use kornetti_providers::ProviderFactory;
use sqlx::PgPool;

/// Shared application state
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub providers: ProviderFactory,
}

impl AppState {
    pub fn new(config: Config, db: PgPool) -> Self {
        let providers = ProviderFactory::new(config.providers.clone());
        Self { config, db, providers }
    }
}
