-- Kornetti Additional Schema Migration
-- Git source integrations and notifications

-- GitHub Apps
CREATE TABLE github_apps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    app_id BIGINT NOT NULL,
    client_id VARCHAR(255) NOT NULL,
    client_secret TEXT NOT NULL, -- Encrypted
    private_key TEXT NOT NULL, -- Encrypted
    webhook_secret TEXT, -- Encrypted
    installation_id BIGINT,
    custom_url VARCHAR(500), -- For GitHub Enterprise
    api_url VARCHAR(500), -- For GitHub Enterprise
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_github_apps_team_id ON github_apps(team_id);

-- GitLab Apps
CREATE TABLE gitlab_apps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    app_id VARCHAR(255) NOT NULL,
    app_secret TEXT NOT NULL, -- Encrypted
    deploy_key_id BIGINT,
    access_token TEXT, -- Encrypted
    refresh_token TEXT, -- Encrypted
    token_expires_at TIMESTAMPTZ,
    custom_url VARCHAR(500), -- For self-hosted GitLab
    group_id BIGINT,
    is_public BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gitlab_apps_team_id ON gitlab_apps(team_id);

-- Bitbucket Apps
CREATE TABLE bitbucket_apps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    client_id VARCHAR(255) NOT NULL,
    client_secret TEXT NOT NULL, -- Encrypted
    access_token TEXT, -- Encrypted
    refresh_token TEXT, -- Encrypted
    token_expires_at TIMESTAMPTZ,
    workspace VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_bitbucket_apps_team_id ON bitbucket_apps(team_id);

-- Gitea/Forgejo Apps
CREATE TABLE gitea_apps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    base_url VARCHAR(500) NOT NULL,
    api_token TEXT NOT NULL, -- Encrypted
    oauth_id VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gitea_apps_team_id ON gitea_apps(team_id);

-- Notification Settings
CREATE TABLE notification_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    notification_type VARCHAR(50) NOT NULL, -- discord, telegram, slack, email
    enabled BOOLEAN NOT NULL DEFAULT true,
    -- Event toggles
    notify_on_deployment_success BOOLEAN NOT NULL DEFAULT true,
    notify_on_deployment_failure BOOLEAN NOT NULL DEFAULT true,
    notify_on_status_change BOOLEAN NOT NULL DEFAULT true,
    notify_on_backup_success BOOLEAN NOT NULL DEFAULT false,
    notify_on_backup_failure BOOLEAN NOT NULL DEFAULT true,
    notify_on_scheduled_task BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_settings_team_id ON notification_settings(team_id);

-- Discord Notifications
CREATE TABLE discord_notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_setting_id UUID NOT NULL REFERENCES notification_settings(id) ON DELETE CASCADE,
    webhook_url TEXT NOT NULL, -- Encrypted
    UNIQUE(notification_setting_id)
);

-- Telegram Notifications
CREATE TABLE telegram_notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_setting_id UUID NOT NULL REFERENCES notification_settings(id) ON DELETE CASCADE,
    bot_token TEXT NOT NULL, -- Encrypted
    chat_id VARCHAR(255) NOT NULL,
    UNIQUE(notification_setting_id)
);

-- Slack Notifications
CREATE TABLE slack_notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_setting_id UUID NOT NULL REFERENCES notification_settings(id) ON DELETE CASCADE,
    webhook_url TEXT NOT NULL, -- Encrypted
    channel VARCHAR(255),
    UNIQUE(notification_setting_id)
);

-- Email Notifications
CREATE TABLE email_notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    notification_setting_id UUID NOT NULL REFERENCES notification_settings(id) ON DELETE CASCADE,
    recipients TEXT[] NOT NULL,
    use_instance_smtp BOOLEAN NOT NULL DEFAULT true,
    smtp_host VARCHAR(255),
    smtp_port INTEGER,
    smtp_username VARCHAR(255),
    smtp_password TEXT, -- Encrypted
    smtp_encryption VARCHAR(20) DEFAULT 'tls',
    from_address VARCHAR(255),
    from_name VARCHAR(255),
    UNIQUE(notification_setting_id)
);

-- Team Invitations
CREATE TABLE team_invitations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    token VARCHAR(255) NOT NULL UNIQUE,
    invited_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(team_id, email)
);

CREATE INDEX idx_team_invitations_token ON team_invitations(token);
CREATE INDEX idx_team_invitations_email ON team_invitations(email);

-- Personal Access Tokens (for API authentication)
CREATE TABLE personal_access_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    abilities JSONB NOT NULL DEFAULT '["*"]',
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_personal_access_tokens_user_id ON personal_access_tokens(user_id);

-- Server Metrics History
CREATE TABLE server_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    cpu_percent REAL,
    memory_percent REAL,
    memory_used_mb BIGINT,
    memory_total_mb BIGINT,
    disk_percent REAL,
    disk_used_gb BIGINT,
    disk_total_gb BIGINT,
    load_average REAL[],
    network_rx_bytes BIGINT,
    network_tx_bytes BIGINT,
    docker_containers_running INTEGER,
    docker_containers_total INTEGER,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_server_metrics_server_id ON server_metrics(server_id, recorded_at DESC);

-- Container Status Tracking
CREATE TABLE container_status (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    container_id VARCHAR(255) NOT NULL,
    container_name VARCHAR(255) NOT NULL,
    -- Resource association (polymorphic)
    resource_type VARCHAR(50), -- application, database, service
    resource_id UUID,
    -- Status
    status VARCHAR(50) NOT NULL,
    health VARCHAR(50),
    exit_code INTEGER,
    -- Timestamps
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_container_status_server_id ON container_status(server_id);
CREATE INDEX idx_container_status_resource ON container_status(resource_type, resource_id);
CREATE UNIQUE INDEX idx_container_status_unique ON container_status(server_id, container_id);

-- Proxy Configuration Cache
CREATE TABLE proxy_configurations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    proxy_type VARCHAR(50) NOT NULL, -- traefik, nginx, caddy
    configuration TEXT NOT NULL,
    configuration_hash VARCHAR(255) NOT NULL,
    applied_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_proxy_config_server ON proxy_configurations(server_id, proxy_type);

-- Deployment Previews (PR deployments)
CREATE TABLE preview_deployments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    pull_request_id INTEGER NOT NULL,
    pull_request_number INTEGER NOT NULL,
    source_branch VARCHAR(255) NOT NULL,
    target_branch VARCHAR(255) NOT NULL,
    fqdn TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    commit_sha VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_preview_deployments_app ON preview_deployments(application_id);
CREATE UNIQUE INDEX idx_preview_deployments_pr ON preview_deployments(application_id, pull_request_id) WHERE deleted_at IS NULL;

-- Docker Registries
CREATE TABLE docker_registries (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    registry_type VARCHAR(50) NOT NULL, -- dockerhub, ghcr, gcr, ecr, custom
    url VARCHAR(500) NOT NULL,
    username VARCHAR(255),
    password TEXT, -- Encrypted
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_docker_registries_team_id ON docker_registries(team_id);

-- Shared Environment Variables (across resources)
CREATE TABLE shared_environment_variables (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    project_id UUID REFERENCES projects(id) ON DELETE CASCADE, -- NULL means team-wide
    environment_id UUID REFERENCES environments(id) ON DELETE CASCADE, -- NULL means project-wide
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    is_secret BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_shared_env_vars_team ON shared_environment_variables(team_id);
CREATE INDEX idx_shared_env_vars_project ON shared_environment_variables(project_id);
CREATE INDEX idx_shared_env_vars_environment ON shared_environment_variables(environment_id);

-- Application Labels (for Docker labels)
CREATE TABLE resource_labels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    resource_type VARCHAR(50) NOT NULL,
    resource_id UUID NOT NULL,
    key VARCHAR(255) NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_resource_labels ON resource_labels(resource_type, resource_id);

-- Subscription & Limits (for cloud hosting)
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id UUID NOT NULL REFERENCES teams(id) ON DELETE CASCADE UNIQUE,
    plan_name VARCHAR(100) NOT NULL DEFAULT 'free',
    stripe_customer_id VARCHAR(255),
    stripe_subscription_id VARCHAR(255),
    -- Limits
    servers_limit INTEGER NOT NULL DEFAULT 1,
    applications_limit INTEGER NOT NULL DEFAULT 5,
    databases_limit INTEGER NOT NULL DEFAULT 5,
    services_limit INTEGER NOT NULL DEFAULT 5,
    -- Usage
    servers_count INTEGER NOT NULL DEFAULT 0,
    applications_count INTEGER NOT NULL DEFAULT 0,
    databases_count INTEGER NOT NULL DEFAULT 0,
    services_count INTEGER NOT NULL DEFAULT 0,
    -- Billing
    current_period_start TIMESTAMPTZ,
    current_period_end TIMESTAMPTZ,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Apply updated_at triggers to new tables
CREATE TRIGGER update_github_apps_updated_at BEFORE UPDATE ON github_apps FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_gitlab_apps_updated_at BEFORE UPDATE ON gitlab_apps FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_bitbucket_apps_updated_at BEFORE UPDATE ON bitbucket_apps FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_gitea_apps_updated_at BEFORE UPDATE ON gitea_apps FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_notification_settings_updated_at BEFORE UPDATE ON notification_settings FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_personal_access_tokens_updated_at BEFORE UPDATE ON personal_access_tokens FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_proxy_configurations_updated_at BEFORE UPDATE ON proxy_configurations FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_preview_deployments_updated_at BEFORE UPDATE ON preview_deployments FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_docker_registries_updated_at BEFORE UPDATE ON docker_registries FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_shared_env_vars_updated_at BEFORE UPDATE ON shared_environment_variables FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_subscriptions_updated_at BEFORE UPDATE ON subscriptions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
