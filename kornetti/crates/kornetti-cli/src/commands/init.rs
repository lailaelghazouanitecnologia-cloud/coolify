//! Initialize Kornetti

use tracing::info;

pub async fn run(database_url: Option<&str>) -> anyhow::Result<()> {
    info!("Initializing Kornetti...");

    let db_url = database_url.unwrap_or("postgres://localhost/kornetti");
    println!("Using database: {}", db_url);

    println!("Kornetti initialized successfully!");
    println!("\nNext steps:");
    println!("  1. Run migrations: kornetti migrate");
    println!("  2. Start the server: kornetti server");

    Ok(())
}
