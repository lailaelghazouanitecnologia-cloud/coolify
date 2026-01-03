# Kornetti

> Next-generation cloud deployment platform written in Rust

Kornetti is a complete rewrite of Coolify, designed for high performance, reliability, and scalability. It provides a self-hostable platform for deploying applications and managing servers.

## Architecture

```
kornetti/
├── crates/                    # Rust workspace crates
│   ├── kornetti-core/         # Core domain models and traits
│   ├── kornetti-api/          # REST API server (Axum)
│   ├── kornetti-cli/          # Command-line interface
│   ├── kornetti-providers/    # Cloud provider integrations
│   ├── kornetti-ssh/          # SSH client and remote execution
│   ├── kornetti-docker/       # Docker and container orchestration
│   ├── kornetti-proxy/        # Traefik/Caddy configuration
│   └── kornetti-scheduler/    # Job queue and scheduler
├── frontend/                  # React frontend (Bun + TypeScript)
└── Cargo.toml                 # Workspace configuration
```

## Tech Stack

### Backend (Rust)
- **Runtime**: Tokio async runtime
- **Web Framework**: Axum
- **Database**: PostgreSQL (SQLx)
- **Cache/Queue**: Redis
- **SSH**: russh
- **Docker**: bollard

### Frontend (TypeScript)
- **Runtime**: Bun
- **Framework**: React 18
- **State**: Zustand + React Query
- **Styling**: Tailwind CSS
- **UI Components**: Radix UI

### Cloud Providers
- Vultr
- Hetzner Cloud
- DigitalOcean
- AWS EC2
- Linode
- Google Cloud Platform (planned)
- Microsoft Azure (planned)

## Getting Started

### Prerequisites
- Rust 1.75+
- Bun 1.0+
- PostgreSQL 15+
- Redis 7+

### Development

```bash
# Clone the repository
git clone https://github.com/coollabsio/coolify
cd coolify/kornetti

# Backend
cargo build
cargo run --bin kornetti -- server

# Frontend
cd frontend
bun install
bun run dev
```

### CLI Usage

```bash
# Initialize Kornetti
kornetti init --database-url postgres://localhost/kornetti

# Run migrations
kornetti migrate

# Start the server
kornetti server --host 0.0.0.0 --port 8080

# Manage servers
kornetti servers list
kornetti servers add myserver --ip 1.2.3.4 --user root

# Deploy applications
kornetti deploy my-app
```

## Project Structure

### Core Crate (`kornetti-core`)

Domain models and shared types:
- `Server`, `Application`, `Database`, `Service`
- `Team`, `Project`, `Environment`
- `Deployment`, `DeploymentLog`
- Traits: `CloudProvider`, `RemoteExecutor`, `ContainerOrchestrator`

### API Crate (`kornetti-api`)

REST API server with endpoints for:
- Authentication & authorization
- Server management
- Project & environment management
- Application deployment
- Database provisioning
- Real-time logs and monitoring

### Providers Crate (`kornetti-providers`)

Cloud provider integrations:
- Server provisioning (create, delete, reboot)
- Region and size listing
- OS image selection
- SSH key management

### SSH Crate (`kornetti-ssh`)

Remote command execution:
- SSH connection management
- Command execution with output streaming
- File upload/download (SCP/SFTP)
- Connection pooling

### Docker Crate (`kornetti-docker`)

Container orchestration:
- Container lifecycle management
- Docker Compose parsing and generation
- Image management
- Log streaming

### Proxy Crate (`kornetti-proxy`)

Reverse proxy configuration:
- Traefik dynamic configuration
- Caddy configuration
- SSL/TLS management
- Domain routing

### Scheduler Crate (`kornetti-scheduler`)

Background job processing:
- Redis-backed job queue
- Concurrent worker execution
- Scheduled jobs (cron-like)
- Retry logic with backoff

## Configuration

Create a `kornetti.toml` file:

```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgres://localhost/kornetti"
max_connections = 10

[redis]
url = "redis://localhost:6379"

[providers.vultr]
api_key = "your-api-key"

[providers.hetzner]
api_token = "your-token"

[providers.digitalocean]
api_token = "your-token"
```

## License

Apache 2.0 - See [LICENSE](../LICENSE) for details.
