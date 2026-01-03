-- Kornetti Initial Schema Migration
-- Based on Coolify's database structure

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Teams (Multi-tenancy root)
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    personal BOOLEAN NOT NULL DEFAULT false,
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    email_verified_at TIMESTAMPTZ,
    password_hash VARCHAR(255) NOT NULL,
    two_factor_secret TEXT,
    two_factor_confirmed_at TIMESTAMPTZ,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Team Members (pivot table)
CREATE TABLE team_members (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(team_id, user_id)
);

-- Private Keys (SSH keys)
CREATE TABLE private_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    private_key TEXT NOT NULL,  -- Encrypted
    public_key TEXT,
    fingerprint VARCHAR(255),
    is_git_related BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Servers
CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    private_key_id UUID REFERENCES private_keys(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    ip VARCHAR(255) NOT NULL,
    port INTEGER NOT NULL DEFAULT 22,
    "user" VARCHAR(255) NOT NULL DEFAULT 'root',
    status VARCHAR(50) NOT NULL DEFAULT 'new',
    validation_logs TEXT,
    log_drain_notification_sent BOOLEAN NOT NULL DEFAULT false,
    -- Cloud provider info
    provider VARCHAR(50),
    provider_id VARCHAR(255),
    region VARCHAR(255),
    -- Settings (JSONB for flexibility)
    settings JSONB NOT NULL DEFAULT '{
        "docker_installed": false,
        "docker_version": null,
        "proxy_type": "traefik",
        "wildcard_domain": null,
        "concurrent_builds": 2,
        "sentinel_enabled": false,
        "is_build_server": false,
        "is_swarm_manager": false,
        "is_swarm_worker": false
    }',
    -- Metrics
    metrics JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_servers_team_id ON servers(team_id);
CREATE INDEX idx_servers_status ON servers(status);

-- Projects
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_projects_team_id ON projects(team_id);

-- Environments
CREATE TABLE environments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_environments_project_id ON environments(project_id);

-- Applications
CREATE TABLE applications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    fqdn TEXT,
    -- Git source
    git_repository VARCHAR(500),
    git_branch VARCHAR(255) DEFAULT 'main',
    git_commit_sha VARCHAR(255),
    private_key_id UUID REFERENCES private_keys(id) ON DELETE SET NULL,
    -- Docker source
    docker_image VARCHAR(500),
    docker_registry_id UUID,
    -- Build configuration
    build_pack VARCHAR(50) NOT NULL DEFAULT 'nixpacks',
    dockerfile_path VARCHAR(500) DEFAULT 'Dockerfile',
    docker_compose_location VARCHAR(500) DEFAULT 'docker-compose.yml',
    dockerfile_content TEXT,
    docker_compose_content TEXT,
    build_command TEXT,
    install_command TEXT,
    start_command TEXT,
    base_directory VARCHAR(500) DEFAULT '/',
    publish_directory VARCHAR(500),
    -- Deploy configuration
    status VARCHAR(50) NOT NULL DEFAULT 'stopped',
    replicas INTEGER NOT NULL DEFAULT 1,
    health_check_enabled BOOLEAN NOT NULL DEFAULT true,
    health_check_path VARCHAR(255) DEFAULT '/',
    health_check_port INTEGER,
    health_check_interval INTEGER DEFAULT 30,
    health_check_timeout INTEGER DEFAULT 5,
    health_check_retries INTEGER DEFAULT 3,
    health_check_start_period INTEGER DEFAULT 30,
    -- Resource limits
    limits_memory VARCHAR(50),
    limits_memory_swap VARCHAR(50),
    limits_memory_swappiness INTEGER DEFAULT 60,
    limits_memory_reservation VARCHAR(50),
    limits_cpus VARCHAR(50),
    limits_cpu_shares INTEGER,
    -- Ports & networking
    ports_mappings JSONB DEFAULT '[]',
    -- Settings
    settings JSONB NOT NULL DEFAULT '{
        "is_static": false,
        "is_spa": false,
        "preview_deployments_enabled": false,
        "auto_deploy_enabled": true,
        "force_https": true,
        "www_redirect": true
    }',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_applications_environment_id ON applications(environment_id);
CREATE INDEX idx_applications_server_id ON applications(server_id);
CREATE INDEX idx_applications_status ON applications(status);

-- Environment Variables
CREATE TABLE environment_variables (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Polymorphic relationship
    resource_type VARCHAR(50) NOT NULL, -- 'application', 'database', 'service'
    resource_id UUID NOT NULL,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT false,
    is_build BOOLEAN NOT NULL DEFAULT false,
    is_preview BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_env_vars_resource ON environment_variables(resource_type, resource_id);

-- Persistent Volumes
CREATE TABLE persistent_volumes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    resource_type VARCHAR(50) NOT NULL,
    resource_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    mount_path VARCHAR(500) NOT NULL,
    host_path VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Application Deployment Queue
CREATE TABLE application_deployments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    deployment_uuid VARCHAR(255) NOT NULL UNIQUE,
    status VARCHAR(50) NOT NULL DEFAULT 'queued',
    deployment_type VARCHAR(50) NOT NULL DEFAULT 'deploy',
    -- Git info
    commit_sha VARCHAR(255),
    commit_message TEXT,
    -- Triggered by
    triggered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    force_rebuild BOOLEAN NOT NULL DEFAULT false,
    rollback_to UUID REFERENCES application_deployments(id),
    -- PR deployment
    pull_request_id INTEGER,
    pull_request_html_url TEXT,
    -- Logs
    logs TEXT,
    -- Timestamps
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deployments_application_id ON application_deployments(application_id);
CREATE INDEX idx_deployments_status ON application_deployments(status);
CREATE INDEX idx_deployments_queued_at ON application_deployments(queued_at);

-- Standalone Databases
CREATE TABLE standalone_databases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    database_type VARCHAR(50) NOT NULL, -- postgresql, mysql, mongodb, redis, etc.
    image VARCHAR(255) NOT NULL,
    version VARCHAR(50),
    status VARCHAR(50) NOT NULL DEFAULT 'stopped',
    -- Connection info
    internal_db_url TEXT,
    external_db_url TEXT,
    public_port INTEGER,
    -- Credentials (encrypted)
    database_name VARCHAR(255),
    database_user VARCHAR(255),
    database_password TEXT,
    root_password TEXT,
    -- Configuration
    configuration JSONB NOT NULL DEFAULT '{}',
    -- Limits
    limits_memory VARCHAR(50),
    limits_cpus VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_databases_environment_id ON standalone_databases(environment_id);
CREATE INDEX idx_databases_server_id ON standalone_databases(server_id);

-- Database Backups
CREATE TABLE database_backups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    database_id UUID NOT NULL REFERENCES standalone_databases(id) ON DELETE CASCADE,
    filename VARCHAR(500) NOT NULL,
    size BIGINT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    s3_storage_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scheduled Database Backups
CREATE TABLE scheduled_database_backups (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    database_id UUID NOT NULL REFERENCES standalone_databases(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT true,
    frequency VARCHAR(100) NOT NULL DEFAULT '0 0 * * *', -- Cron expression
    number_of_backups_to_keep INTEGER NOT NULL DEFAULT 7,
    s3_storage_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Services (Docker Compose based)
CREATE TABLE services (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    docker_compose TEXT NOT NULL,
    docker_compose_raw TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'stopped',
    configuration_hash VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Service Applications (containers within a service)
CREATE TABLE service_applications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    fqdn TEXT,
    image VARCHAR(500),
    exclude_from_status BOOLEAN NOT NULL DEFAULT false,
    required_fqdn BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Service Databases (database containers within a service)
CREATE TABLE service_databases (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    image VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- S3 Storage configurations
CREATE TABLE s3_storages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    endpoint VARCHAR(500) NOT NULL,
    bucket VARCHAR(255) NOT NULL,
    region VARCHAR(255) NOT NULL,
    access_key VARCHAR(255) NOT NULL,
    secret_key TEXT NOT NULL, -- Encrypted
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scheduled Tasks
CREATE TABLE scheduled_tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Polymorphic
    resource_type VARCHAR(50) NOT NULL,
    resource_id UUID NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    name VARCHAR(255) NOT NULL,
    command TEXT NOT NULL,
    frequency VARCHAR(100) NOT NULL, -- Cron expression
    container VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scheduled Task Executions
CREATE TABLE scheduled_task_executions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    scheduled_task_id UUID NOT NULL REFERENCES scheduled_tasks(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'running',
    output TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ
);

-- Job Queue (for background processing)
CREATE TABLE jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    queue VARCHAR(255) NOT NULL DEFAULT 'default',
    job_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    available_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reserved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jobs_queue_status ON jobs(queue, status, available_at);

-- Failed Jobs
CREATE TABLE failed_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID,
    queue VARCHAR(255) NOT NULL,
    job_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    exception TEXT NOT NULL,
    failed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Activity Log (audit trail)
CREATE TABLE activity_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Subject (what was affected)
    subject_type VARCHAR(100),
    subject_id UUID,
    -- Causer (who did it)
    causer_type VARCHAR(100),
    causer_id UUID,
    -- Event details
    event VARCHAR(100) NOT NULL,
    description TEXT,
    properties JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_activity_logs_team ON activity_logs(team_id, created_at DESC);
CREATE INDEX idx_activity_logs_subject ON activity_logs(subject_type, subject_id);

-- API Tokens
CREATE TABLE api_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    abilities JSONB NOT NULL DEFAULT '["read"]',
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Webhook configurations
CREATE TABLE webhooks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    secret VARCHAR(255),
    events JSONB NOT NULL DEFAULT '[]',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Instance Settings (global configuration)
CREATE TABLE instance_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- General
    is_registration_enabled BOOLEAN NOT NULL DEFAULT true,
    is_https_forced BOOLEAN NOT NULL DEFAULT false,
    -- DNS
    fqdn TEXT,
    wildcard_domain TEXT,
    -- SMTP
    smtp_enabled BOOLEAN NOT NULL DEFAULT false,
    smtp_host VARCHAR(255),
    smtp_port INTEGER DEFAULT 587,
    smtp_username VARCHAR(255),
    smtp_password TEXT,
    smtp_from_address VARCHAR(255),
    smtp_from_name VARCHAR(255),
    -- Notifications
    discord_webhook_url TEXT,
    slack_webhook_url TEXT,
    telegram_token TEXT,
    telegram_chat_id VARCHAR(255),
    -- Auto-update
    is_auto_update_enabled BOOLEAN NOT NULL DEFAULT false,
    new_version_available VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default instance settings
INSERT INTO instance_settings (id) VALUES (uuid_generate_v4());

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply updated_at triggers
CREATE TRIGGER update_teams_updated_at BEFORE UPDATE ON teams FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_private_keys_updated_at BEFORE UPDATE ON private_keys FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_servers_updated_at BEFORE UPDATE ON servers FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_projects_updated_at BEFORE UPDATE ON projects FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_environments_updated_at BEFORE UPDATE ON environments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_applications_updated_at BEFORE UPDATE ON applications FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_env_vars_updated_at BEFORE UPDATE ON environment_variables FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_databases_updated_at BEFORE UPDATE ON standalone_databases FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_services_updated_at BEFORE UPDATE ON services FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_s3_storages_updated_at BEFORE UPDATE ON s3_storages FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_scheduled_tasks_updated_at BEFORE UPDATE ON scheduled_tasks FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_scheduled_db_backups_updated_at BEFORE UPDATE ON scheduled_database_backups FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_webhooks_updated_at BEFORE UPDATE ON webhooks FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_instance_settings_updated_at BEFORE UPDATE ON instance_settings FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
