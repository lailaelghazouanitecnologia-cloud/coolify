//! Kornetti CLI
//!
//! Command-line interface for the Kornetti deployment platform.

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

#[derive(Parser)]
#[command(name = "kornetti")]
#[command(author, version, about = "Next-generation cloud deployment platform", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true, default_value = "kornetti.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Kornetti server
    Server {
        /// Host to bind to
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Manage servers
    Servers {
        #[command(subcommand)]
        command: ServersCommands,
    },

    /// Manage projects
    Projects {
        #[command(subcommand)]
        command: ProjectsCommands,
    },

    /// Deploy applications
    Deploy {
        /// Application ID or name
        app: String,

        /// Force rebuild
        #[arg(short, long)]
        force: bool,
    },

    /// Initialize a new Kornetti instance
    Init {
        /// Database URL
        #[arg(long)]
        database_url: Option<String>,
    },

    /// Run database migrations
    Migrate,

    /// Show version information
    Version,
}

#[derive(Subcommand)]
enum ServersCommands {
    /// List all servers
    List,

    /// Add a new server
    Add {
        /// Server name
        name: String,

        /// Server IP address
        #[arg(short, long)]
        ip: String,

        /// SSH port
        #[arg(short, long, default_value = "22")]
        port: u16,

        /// SSH user
        #[arg(short, long, default_value = "root")]
        user: String,
    },

    /// Remove a server
    Remove {
        /// Server ID or name
        server: String,
    },

    /// Validate server connection
    Validate {
        /// Server ID or name
        server: String,
    },
}

#[derive(Subcommand)]
enum ProjectsCommands {
    /// List all projects
    List,

    /// Create a new project
    Create {
        /// Project name
        name: String,
    },

    /// Delete a project
    Delete {
        /// Project ID or name
        project: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Initialize tracing
    let filter = if cli.verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    match cli.command {
        Commands::Server { host, port } => {
            commands::server::run(&host, port).await?;
        }
        Commands::Servers { command } => match command {
            ServersCommands::List => commands::servers::list().await?,
            ServersCommands::Add { name, ip, port, user } => {
                commands::servers::add(&name, &ip, port, &user).await?
            }
            ServersCommands::Remove { server } => commands::servers::remove(&server).await?,
            ServersCommands::Validate { server } => commands::servers::validate(&server).await?,
        },
        Commands::Projects { command } => match command {
            ProjectsCommands::List => commands::projects::list().await?,
            ProjectsCommands::Create { name } => commands::projects::create(&name).await?,
            ProjectsCommands::Delete { project } => commands::projects::delete(&project).await?,
        },
        Commands::Deploy { app, force } => {
            commands::deploy::run(&app, force).await?;
        }
        Commands::Init { database_url } => {
            commands::init::run(database_url.as_deref()).await?;
        }
        Commands::Migrate => {
            commands::migrate::run().await?;
        }
        Commands::Version => {
            println!("kornetti {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
