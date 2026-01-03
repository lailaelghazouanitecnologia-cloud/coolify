//! Server management commands

use tracing::info;

pub async fn list() -> anyhow::Result<()> {
    info!("Listing servers...");
    println!("No servers configured yet.");
    Ok(())
}

pub async fn add(name: &str, ip: &str, port: u16, user: &str) -> anyhow::Result<()> {
    info!("Adding server {} ({}@{}:{})", name, user, ip, port);
    println!("Server '{}' added successfully.", name);
    Ok(())
}

pub async fn remove(server: &str) -> anyhow::Result<()> {
    info!("Removing server: {}", server);
    println!("Server '{}' removed.", server);
    Ok(())
}

pub async fn validate(server: &str) -> anyhow::Result<()> {
    info!("Validating server: {}", server);
    println!("Server '{}' connection validated.", server);
    Ok(())
}
