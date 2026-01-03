import { useParams } from 'react-router-dom'
import { Server, Activity, HardDrive, Cpu, MemoryStick } from 'lucide-react'
import { Card, CardHeader, CardTitle, CardContent, CardDescription } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { useServer, useServerResources, useValidateServer, useInstallDocker } from '@/api/servers'

export function ServerDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { data: server, isLoading } = useServer(id!)
  const { data: resources } = useServerResources(id!)
  const validateServer = useValidateServer()
  const installDocker = useInstallDocker()

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
      </div>
    )
  }

  if (!server) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <Server className="h-12 w-12 text-muted-foreground" />
        <h3 className="mt-4 text-lg font-medium">Server not found</h3>
      </div>
    )
  }

  const formatBytes = (bytes: number) => {
    const gb = bytes / (1024 * 1024 * 1024)
    return `${gb.toFixed(1)} GB`
  }

  const formatPercentage = (used: number, total: number) => {
    return `${((used / total) * 100).toFixed(1)}%`
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-4">
          <div className="flex h-16 w-16 items-center justify-center rounded-lg bg-muted">
            <Server className="h-8 w-8" />
          </div>
          <div>
            <h1 className="text-3xl font-bold">{server.name}</h1>
            <p className="text-muted-foreground">
              {server.ip}:{server.port} • {server.user}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Badge
            variant={
              server.status === 'ready'
                ? 'success'
                : server.status === 'error'
                ? 'error'
                : 'warning'
            }
          >
            {server.status}
          </Badge>
        </div>
      </div>

      {/* Actions */}
      <div className="flex gap-2">
        <Button
          variant="outline"
          onClick={() => validateServer.mutate(id!)}
          loading={validateServer.isPending}
        >
          Validate Connection
        </Button>
        {!server.settings.docker_installed && (
          <Button
            onClick={() => installDocker.mutate(id!)}
            loading={installDocker.isPending}
          >
            Install Docker
          </Button>
        )}
      </div>

      {/* Resources */}
      {resources && (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">CPU</CardTitle>
              <Cpu className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{resources.cpu_usage.toFixed(1)}%</div>
              <div className="mt-2 h-2 overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${resources.cpu_usage}%` }}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Memory</CardTitle>
              <MemoryStick className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {formatPercentage(resources.memory_used, resources.memory_total)}
              </div>
              <p className="text-xs text-muted-foreground">
                {formatBytes(resources.memory_used)} / {formatBytes(resources.memory_total)}
              </p>
              <div className="mt-2 h-2 overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-primary transition-all"
                  style={{
                    width: `${(resources.memory_used / resources.memory_total) * 100}%`,
                  }}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Disk</CardTitle>
              <HardDrive className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {formatPercentage(resources.disk_used, resources.disk_total)}
              </div>
              <p className="text-xs text-muted-foreground">
                {formatBytes(resources.disk_used)} / {formatBytes(resources.disk_total)}
              </p>
              <div className="mt-2 h-2 overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-primary transition-all"
                  style={{
                    width: `${(resources.disk_used / resources.disk_total) * 100}%`,
                  }}
                />
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex flex-row items-center justify-between pb-2">
              <CardTitle className="text-sm font-medium">Containers</CardTitle>
              <Activity className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">{resources.containers_running}</div>
              <p className="text-xs text-muted-foreground">
                {resources.containers_total} total
              </p>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Server Info */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Connection Details</CardTitle>
            <CardDescription>SSH connection information</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex justify-between">
              <span className="text-muted-foreground">IP Address</span>
              <span className="font-mono">{server.ip}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Port</span>
              <span className="font-mono">{server.port}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">User</span>
              <span className="font-mono">{server.user}</span>
            </div>
            {server.provider && (
              <div className="flex justify-between">
                <span className="text-muted-foreground">Provider</span>
                <span className="capitalize">{server.provider}</span>
              </div>
            )}
            {server.region && (
              <div className="flex justify-between">
                <span className="text-muted-foreground">Region</span>
                <span>{server.region}</span>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Configuration</CardTitle>
            <CardDescription>Server settings and features</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Docker</span>
              <span>
                {server.settings.docker_installed
                  ? server.settings.docker_version || 'Installed'
                  : 'Not installed'}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Proxy</span>
              <span className="capitalize">{server.settings.proxy_type}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Wildcard Domain</span>
              <span>{server.settings.wildcard_domain || 'Not configured'}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Concurrent Builds</span>
              <span>{server.settings.concurrent_builds}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Sentinel</span>
              <Badge variant={server.settings.sentinel_enabled ? 'success' : 'secondary'}>
                {server.settings.sentinel_enabled ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
