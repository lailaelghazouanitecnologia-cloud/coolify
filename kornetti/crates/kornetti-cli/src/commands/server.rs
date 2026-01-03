//! Server command - start the Kornetti API server

use std::sync::Arc;
use tracing::info;
use kornetti_core::Config;
use kornetti_api::{build_router, AppState};

pub async fn run(host: &str, port: u16) -> anyhow::Result<()> {
    info!("Starting Kornetti server on {}:{}", host, port);

    // Load configuration
    let config = Config::default();

    // Connect to database
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/kornetti".to_string());

    let db = sqlx::PgPool::connect(&db_url).await?;
    info!("Connected to database");

    // Create application state
    let state = Arc::new(AppState::new(config, db));

    // Build router
    let app = build_router(state);

    // Start server
    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Kornetti server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
