//! Project management commands

use tracing::info;

pub async fn list() -> anyhow::Result<()> {
    info!("Listing projects...");
    println!("No projects found.");
    Ok(())
}

pub async fn create(name: &str) -> anyhow::Result<()> {
    info!("Creating project: {}", name);
    println!("Project '{}' created successfully.", name);
    Ok(())
}

pub async fn delete(project: &str) -> anyhow::Result<()> {
    info!("Deleting project: {}", project);
    println!("Project '{}' deleted.", project);
    Ok(())
}
