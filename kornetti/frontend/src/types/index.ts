// Core domain types for Kornetti frontend

export type UUID = string

export interface User {
  id: UUID
  email: string
  name: string
  avatar_url?: string
  created_at: string
}

export interface Team {
  id: UUID
  name: string
  description?: string
  personal: boolean
  created_at: string
}

export interface TeamMember {
  id: UUID
  team_id: UUID
  user_id: UUID
  role: TeamRole
  user: User
}

export type TeamRole = 'owner' | 'admin' | 'member' | 'viewer'

export interface Project {
  id: UUID
  team_id: UUID
  name: string
  description?: string
  environments: Environment[]
  created_at: string
  updated_at: string
}

export interface Environment {
  id: UUID
  project_id: UUID
  name: string
  description?: string
  created_at: string
}

export interface Server {
  id: UUID
  team_id: UUID
  name: string
  description?: string
  ip: string
  port: number
  user: string
  status: ServerStatus
  provider?: CloudProvider
  provider_id?: string
  region?: string
  settings: ServerSettings
  created_at: string
  updated_at: string
}

export type ServerStatus =
  | 'new'
  | 'validating'
  | 'reachable'
  | 'unreachable'
  | 'installing'
  | 'ready'
  | 'error'

export type CloudProvider =
  | 'vultr'
  | 'hetzner'
  | 'digitalocean'
  | 'aws'
  | 'linode'
  | 'gcp'
  | 'azure'
  | 'custom'

export interface ServerSettings {
  docker_installed: boolean
  docker_version?: string
  proxy_type: ProxyType
  wildcard_domain?: string
  concurrent_builds: number
  sentinel_enabled: boolean
}

export type ProxyType = 'traefik' | 'caddy' | 'none'

export interface Application {
  id: UUID
  environment_id: UUID
  server_id: UUID
  name: string
  description?: string
  fqdn?: string
  status: ApplicationStatus
  source: ApplicationSource
  build_config: BuildConfig
  deploy_config: DeployConfig
  created_at: string
  updated_at: string
}

export type ApplicationStatus =
  | 'stopped'
  | 'starting'
  | 'running'
  | 'stopping'
  | 'restarting'
  | 'degraded'
  | 'error'

export type ApplicationSource =
  | { type: 'git'; repository_url: string; branch: string; commit_sha?: string }
  | { type: 'docker_image'; image: string; tag: string }
  | { type: 'docker_compose'; content: string }
  | { type: 'dockerfile'; content: string; context: string }

export interface BuildConfig {
  build_pack: BuildPack
  dockerfile_path?: string
  build_command?: string
  install_command?: string
  start_command?: string
  base_directory?: string
  publish_directory?: string
}

export type BuildPack =
  | 'nixpacks'
  | 'dockerfile'
  | 'docker_image'
  | 'docker_compose'
  | 'static'

export interface DeployConfig {
  replicas: number
  health_check?: HealthCheck
  resources: ResourceLimits
  ports: PortMapping[]
  volumes: VolumeMapping[]
  environment_variables: EnvironmentVariable[]
}

export interface HealthCheck {
  path: string
  port: number
  interval: number
  timeout: number
  retries: number
  start_period: number
}

export interface ResourceLimits {
  memory_limit?: string
  memory_reservation?: string
  cpu_limit?: number
  cpu_reservation?: number
}

export interface PortMapping {
  container_port: number
  host_port?: number
  protocol: 'tcp' | 'udp'
  public: boolean
}

export interface VolumeMapping {
  container_path: string
  host_path?: string
  volume_name?: string
}

export interface EnvironmentVariable {
  key: string
  value: string
  is_secret: boolean
}

export interface Deployment {
  id: UUID
  application_id: UUID
  server_id: UUID
  status: DeploymentStatus
  deployment_type: DeploymentType
  commit_sha?: string
  commit_message?: string
  triggered_by?: UUID
  started_at: string
  finished_at?: string
  created_at: string
}

export type DeploymentStatus =
  | 'queued'
  | 'in_progress'
  | 'finished'
  | 'failed'
  | 'cancelled'

export type DeploymentType = 'deploy' | 'redeploy' | 'rollback' | 'pull_request'

export interface DeploymentLog {
  timestamp: string
  level: 'debug' | 'info' | 'warning' | 'error'
  message: string
  step?: string
}

export interface Database {
  id: UUID
  environment_id: UUID
  server_id: UUID
  name: string
  description?: string
  database_type: DatabaseType
  version: string
  status: DatabaseStatus
  created_at: string
}

export type DatabaseType =
  | 'postgresql'
  | 'mysql'
  | 'mariadb'
  | 'mongodb'
  | 'redis'
  | 'keydb'
  | 'dragonfly'
  | 'clickhouse'

export type DatabaseStatus =
  | 'stopped'
  | 'starting'
  | 'running'
  | 'stopping'
  | 'error'

export interface Service {
  id: UUID
  environment_id: UUID
  server_id: UUID
  name: string
  description?: string
  status: ServiceStatus
  created_at: string
}

export type ServiceStatus =
  | 'stopped'
  | 'starting'
  | 'running'
  | 'partially_running'
  | 'stopping'
  | 'error'

// API Response types
export interface PaginatedResponse<T> {
  data: T[]
  meta: {
    current_page: number
    per_page: number
    total: number
    total_pages: number
  }
}

export interface ApiError {
  message: string
  code?: string
  details?: Record<string, string[]>
}
